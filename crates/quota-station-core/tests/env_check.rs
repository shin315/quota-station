// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The variables that override a login (M14, I), on fake home folders with
//! fake shell files and a fake `launchctl` that records its words. Nothing
//! here reads the real home, its shell files or launchd's environment.
//!
//! The trap: a value planted in a shell file and in launchd's answer must
//! never reach what the check returns, the alerts, the app's files or the
//! window's model.
// Unix only: the fake `launchctl` is a shell script, and the files read are zsh's and bash's.
#![cfg(unix)]

mod common;

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use common::{script, temp_dir};
use quota_station_core::agents::env_check::{self, Check, Found, Place, Weight, VARS};
use quota_station_core::agents::Agent;
use quota_station_core::alerts::{self, Effect, Kind, Memory, Tone};
use quota_station_core::app_dir::AppDir;
use time::UtcOffset;

/// Planted as every value: none of it may come out.
const TRAP: &str = "sk-ant-TRAP-9f3c2e71d0b54a68";

/// A fake Mac: a home with shell files, an app folder, and a `launchctl`
/// that writes each call's words to `argv` and answers `getenv NAME` with
/// the trap for the names in `set`.
struct Mac {
    root: PathBuf,
    home: PathBuf,
    app: AppDir,
    launchctl: PathBuf,
}

impl Mac {
    fn new(tag: &str) -> Mac {
        let root = temp_dir(tag);
        let home = root.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let app = AppDir::new(root.join("app"));
        std::fs::create_dir_all(app.path()).unwrap();
        let launchctl = script(
            &root.join("bin/launchctl"),
            &format!(
                r#"d='{root}'
printf '%s ' "$@" >> "$d/argv"; printf '\n' >> "$d/argv"
[ "$1" = getenv ] || exit 1
grep -qx "$2" "$d/set" 2>/dev/null && printf '%s\n' '{TRAP}'
exit 0
"#,
                root = root.display()
            ),
        );
        Mac { root, home, app, launchctl }
    }

    fn file(&self, name: &str, text: &str) {
        std::fs::write(self.home.join(name), text).unwrap();
    }

    fn launchd(&self, names: &[&str]) {
        std::fs::write(self.root.join("set"), names.join("\n") + "\n").unwrap();
    }

    fn check(&self) -> Check {
        env_check::check(&self.home, &self.launchctl)
    }

    fn argv(&self) -> Vec<String> {
        std::fs::read_to_string(self.root.join("argv"))
            .unwrap_or_default()
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }
}

fn ids(c: &Check) -> Vec<(String, Option<u32>)> {
    c.found.iter().map(|f| (f.id.clone(), f.line)).collect()
}

/// Every file under `dir`, read whole.
fn every_file(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(every_file(&p));
        } else {
            out.push((p.clone(), String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into()));
        }
    }
    out
}

fn no_trap(what: &str, text: &str) {
    assert!(!text.contains(TRAP), "{what} holds the value: {text}");
    assert!(!text.contains("TRAP"), "{what} holds part of the value: {text}");
}

#[test]
fn names_are_found_at_the_start_of_a_line_in_each_shell_file() {
    let mac = Mac::new("env-files");
    mac.file(
        ".zshrc",
        "# my shell\n\
         alias ll='ls -l'\n\
         # export ANTHROPIC_API_KEY=old\n\
         export ANTHROPIC_API_KEY=first\n\
         echo hi\n\
         export ANTHROPIC_API_KEY=\"second\"\n\
         export ANTHROPIC_API_KEY_BACKUP=x\n",
    );
    mac.file(
        ".zprofile",
        "path=(/opt/homebrew/bin $path)\n\n\nOPENAI_BASE_URL=https://relay.example.com/v1\n",
    );
    mac.file(".zshenv", "  export\tCLAUDE_CONFIG_DIR=$HOME/.claude-work\n");
    mac.file(".bash_profile", "unset CODEX_HOME\nexport PATH=$PATH:/x\n");
    mac.file(".profile", "export GOOGLE_CLOUD_PROJECT=demo-project\n");
    let c = mac.check();
    assert_eq!(
        ids(&c),
        vec![
            // The last line that sets it is the one that wins.
            ("ANTHROPIC_API_KEY@zshrc".to_string(), Some(6)),
            ("CLAUDE_CONFIG_DIR@zshenv".into(), Some(1)),
            ("OPENAI_BASE_URL@zprofile".into(), Some(4)),
            ("GOOGLE_CLOUD_PROJECT@profile".into(), Some(1)),
        ]
    );
    assert!(c.unread.is_empty(), "{:?}", c.unread);
    let key = &c.found[0];
    assert_eq!((key.agent, key.weight, key.place), (Agent::Claude, Weight::Bad, Place::Zshrc));
    assert_eq!(key.at(), "~/.zshrc dòng 6");
    assert_eq!(
        key.effect,
        "Trong Terminal, Claude Code dùng khoá này thay cho tài khoản claude-swap đang chọn."
    );
    assert_eq!(c.found[2].weight, Weight::Warn);
}

#[test]
fn no_file_and_nothing_in_launchd_is_nothing_set() {
    let mac = Mac::new("env-none");
    let c = mac.check();
    assert!(c.found.is_empty() && c.unread.is_empty(), "{c:?}");
}

#[test]
fn launchctl_is_asked_getenv_for_each_name_and_nothing_else() {
    let mac = Mac::new("env-launchctl");
    mac.launchd(&["OPENAI_API_KEY", "GEMINI_API_KEY"]);
    let c = mac.check();
    assert_eq!(
        ids(&c),
        vec![
            ("OPENAI_API_KEY@launchctl".to_string(), None),
            ("GEMINI_API_KEY@launchctl".into(), None)
        ]
    );
    let words: Vec<String> = VARS.iter().map(|v| format!("getenv {}", v.name)).collect();
    assert_eq!(mac.argv(), words);
    assert_eq!(
        c.found[0].effect,
        "Trong các app mở từ Dock, Codex có thể dùng khoá này thay cho đăng nhập ChatGPT."
    );
    assert_eq!(c.found[0].at(), "launchctl");
}

#[test]
fn a_place_that_cannot_be_looked_at_is_said_and_the_rest_still_checked() {
    let mac = Mac::new("env-unread");
    mac.file(".zshrc", "export ANTHROPIC_BASE_URL=https://proxy.example.com\n");
    mac.file(".zprofile", "export CODEX_HOME=/tmp/x\n");
    std::fs::set_permissions(mac.home.join(".zprofile"), std::fs::Permissions::from_mode(0o000))
        .unwrap();
    // A folder where a file should be, and a file too large to be a shell file.
    std::fs::create_dir_all(mac.home.join(".profile")).unwrap();
    std::fs::write(mac.home.join(".zshenv"), vec![b'#'; 2 * 1024 * 1024]).unwrap();
    let c = env_check::check(&mac.home, &mac.root.join("bin/no-launchctl"));
    std::fs::set_permissions(mac.home.join(".zprofile"), std::fs::Permissions::from_mode(0o600))
        .unwrap();
    assert_eq!(ids(&c), vec![("ANTHROPIC_BASE_URL@zshrc".to_string(), Some(1))]);
    let mut unread = c.unread.clone();
    unread.sort();
    assert_eq!(unread, vec![Place::Zshenv, Place::Zprofile, Place::Profile, Place::Launchctl]);
}

#[test]
fn a_shell_file_is_never_run() {
    let mac = Mac::new("env-norun");
    let ran = mac.root.join("ran");
    mac.file(
        ".zshrc",
        &format!(
            "touch '{r}'\nexport ANTHROPIC_API_KEY=\"$(touch '{r}')x\"\n`touch '{r}'`\nsource ~/.zprofile\n",
            r = ran.display()
        ),
    );
    mac.file(".zprofile", &format!("touch '{}'\n", ran.display()));
    let c = mac.check();
    assert_eq!(ids(&c), vec![("ANTHROPIC_API_KEY@zshrc".to_string(), Some(2))]);
    assert!(!ran.exists(), "a shell file was run");
    // launchctl is the only command, and only `getenv`.
    assert!(mac.argv().iter().all(|w| w.starts_with("getenv ")), "{:?}", mac.argv());
}

/// The trap: the value is planted in every place, and every way out of the
/// check is searched for it.
#[test]
fn the_value_never_reaches_the_result_the_alerts_the_files_or_the_window() {
    let mac = Mac::new("env-trap");
    let every: Vec<&str> = VARS.iter().map(|v| v.name).collect();
    let lines: String =
        every.iter().map(|n| format!("export {n}=\"{TRAP}\"\n{n}={TRAP}\n")).collect();
    for f in [".zshenv", ".zprofile", ".zshrc", ".bash_profile", ".profile"] {
        mac.file(f, &lines);
    }
    mac.launchd(&every);

    let c = mac.check();
    assert_eq!(c.found.len(), 11 * 6, "every name in every place");
    no_trap("the check", &serde_json::to_string(&c).unwrap());
    no_trap("the check's debug form", &format!("{c:?}"));

    // "Bỏ qua" on one, kept in the app's folder.
    let mut ignored: BTreeSet<String> = ["ANTHROPIC_API_KEY@zshrc".to_string()].into();
    env_check::save_ignored(&mac.app, &ignored).unwrap();
    let shown = env_check::shown(&c, &mut ignored);
    assert_eq!(shown.len(), 11 * 6 - 1);

    // One card per variable, kept as the app keeps its alerts.
    let mut memory = Memory::default();
    let sent = alerts::env_alerts(&mut memory, &shown, 1_768_547_700);
    assert_eq!(sent.len(), 11);
    alerts::env_ignored(&mut memory, "OPENAI_API_KEY", 1_768_547_760, UtcOffset::UTC);
    alerts::save(&mac.app, &memory).unwrap();
    no_trap("the alerts", &serde_json::to_string(&memory).unwrap());

    // Nothing the app wrote holds it.
    for (path, text) in every_file(mac.app.path()) {
        no_trap(&path.display().to_string(), &text);
    }
    // Nor what the window is sent: the found as the app sends them.
    let window = serde_json::json!({ "found": shown, "unread": c.unread, "alerts": memory.recent });
    no_trap("the window's model", &window.to_string());
    // The window's own tests draw this very model (`src/fixtures/env-trap.json`):
    // `UPDATE_UI_FIXTURES=1` writes it again after a change to its shape.
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/fixtures/env-trap.json");
    let text = serde_json::to_string_pretty(&window).unwrap() + "\n";
    if std::env::var_os("UPDATE_UI_FIXTURES").is_some() {
        std::fs::write(&fixture, &text).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&fixture).unwrap(), text, "run with UPDATE_UI_FIXTURES=1");
    // The shell files were not touched.
    assert_eq!(std::fs::read_to_string(mac.home.join(".zshrc")).unwrap(), lines);
}

#[test]
fn ignored_places_are_hidden_and_forgotten_once_gone() {
    let mac = Mac::new("env-ignore");
    mac.file(".zshrc", "export ANTHROPIC_API_KEY=x\nexport OPENAI_BASE_URL=y\n");
    let c = mac.check();
    let mut ignored: BTreeSet<String> =
        ["ANTHROPIC_API_KEY@zshrc".to_string(), "CODEX_HOME@profile".into()].into();
    env_check::save_ignored(&mac.app, &ignored).unwrap();
    assert_eq!(env_check::load_ignored(&mac.app), ignored);
    let shown = env_check::shown(&c, &mut ignored);
    assert_eq!(
        shown.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        vec!["OPENAI_BASE_URL@zshrc"]
    );
    // CODEX_HOME is set nowhere now: set again later, it shows again.
    assert_eq!(ignored, ["ANTHROPIC_API_KEY@zshrc".to_string()].into());
    // The same name in another place is not ignored.
    mac.launchd(&["ANTHROPIC_API_KEY"]);
    let shown = env_check::shown(&mac.check(), &mut ignored);
    assert!(shown.iter().any(|f| f.id == "ANTHROPIC_API_KEY@launchctl"));
    assert!(!shown.iter().any(|f| f.id == "ANTHROPIC_API_KEY@zshrc"));
}

fn found(mac: &Mac) -> Vec<Found> {
    mac.check().found
}

#[test]
fn one_alert_per_variable_sent_again_only_for_a_new_place() {
    let mac = Mac::new("env-alerts");
    mac.file(".zshrc", "export ANTHROPIC_API_KEY=x\nexport OPENAI_BASE_URL=y\n");
    mac.launchd(&["ANTHROPIC_API_KEY"]);
    let mut m = Memory::default();
    let sent = alerts::env_alerts(&mut m, &found(&mac), 100);
    assert_eq!(sent.len(), 2);
    let key = m.recent.iter().find(|r| r.alert.title.starts_with("ANTHROPIC")).unwrap();
    assert_eq!(key.alert.kind, Kind::Env);
    assert_eq!(key.alert.tone, Tone::Bad);
    assert_eq!(key.alert.title, "ANTHROPIC_API_KEY đang đè đăng nhập Claude Code");
    assert_eq!(
        key.alert.body,
        "Đặt ở ~/.zshrc dòng 1 và launchctl. Trong Terminal, Claude Code dùng khoá này thay cho tài khoản claude-swap đang chọn."
    );
    assert_eq!(
        key.alert.actions.iter().map(|a| (a.id.as_str(), a.label.as_str())).collect::<Vec<_>>(),
        vec![("env:settings", "Xem"), ("env:ignore:ANTHROPIC_API_KEY", "Bỏ qua")]
    );
    let base = m.recent.iter().find(|r| r.alert.title.starts_with("OPENAI")).unwrap();
    assert_eq!(
        (base.alert.tone, base.alert.title.as_str()),
        (Tone::Warn, "OPENAI_BASE_URL đang đè đăng nhập Codex")
    );
    // In the app only: never handed to macOS.
    assert!(m.recent.iter().all(|r| !r.notified));

    // The same places again, even on another line: nothing new.
    mac.file(".zshrc", "\n\nexport ANTHROPIC_API_KEY=x\nexport OPENAI_BASE_URL=y\n");
    assert!(alerts::env_alerts(&mut m, &found(&mac), 200).is_empty());
    // A new place: a new card.
    mac.file(".zprofile", "OPENAI_BASE_URL=z\n");
    let again = alerts::env_alerts(&mut m, &found(&mac), 300);
    assert_eq!(again.len(), 1);
    assert_eq!(
        m.recent[0].alert.body.split(". ").next().unwrap(),
        "Đặt ở ~/.zprofile dòng 1 và ~/.zshrc dòng 4"
    );
}

#[test]
fn the_card_buttons_open_settings_and_ignore() {
    let mac = Mac::new("env-buttons");
    mac.file(".zshrc", "export ANTHROPIC_API_KEY=x\nexport CODEX_HOME=y\n");
    let mut m = Memory::default();
    let ids = alerts::env_alerts(&mut m, &found(&mac), 100);
    assert_eq!(m.unanswered(), 2);
    let codex = ids.iter().find(|i| i.contains("CODEX_HOME")).unwrap().clone();
    let key = ids.iter().find(|i| i.contains("ANTHROPIC")).unwrap().clone();
    assert_eq!(
        alerts::answer(&mut m, &codex, "env:settings", 110, UtcOffset::UTC),
        Ok(Effect::OpenSettings)
    );
    // "Bỏ qua" in Cài đặt answers the card that nobody answered yet.
    assert_eq!(
        alerts::env_ignored(&mut m, "ANTHROPIC_API_KEY", 120, UtcOffset::UTC),
        vec![key.clone()]
    );
    assert_eq!(m.unanswered(), 0);
    let r = m.record(&key).unwrap().answer.as_ref().unwrap();
    assert_eq!(r.label, "Bỏ qua");
    assert_eq!(r.result.as_ref().unwrap().title, "Đã bỏ qua ANTHROPIC_API_KEY");
    assert!(alerts::env_ignored(&mut m, "ANTHROPIC_API_KEY", 130, UtcOffset::UTC).is_empty());

    let mut fresh = Memory::default();
    let id = alerts::env_alerts(&mut fresh, &found(&mac)[..1], 100).remove(0);
    assert_eq!(
        alerts::answer(&mut fresh, &id, "env:ignore:ANTHROPIC_API_KEY", 110, UtcOffset::UTC),
        Ok(Effect::IgnoreEnv { name: "ANTHROPIC_API_KEY".into() })
    );
}

#[test]
fn open_file_goes_to_the_line_in_an_editor_that_can() {
    let mac = Mac::new("env-open");
    let apps = mac.root.join("Applications");
    let path = Path::new("/Users/someone/My Home/.zshrc");
    let (args, editor) = env_check::open_args(path, Some(42), std::slice::from_ref(&apps));
    assert_eq!((args, editor), (vec!["-t".to_string(), path.display().to_string()], None));
    std::fs::create_dir_all(apps.join("Zed.app")).unwrap();
    std::fs::create_dir_all(apps.join("Cursor.app")).unwrap();
    let (args, editor) = env_check::open_args(path, Some(42), std::slice::from_ref(&apps));
    assert_eq!(editor, Some("Cursor"));
    assert_eq!(args, vec!["cursor://file/Users/someone/My%20Home/.zshrc:42".to_string()]);
    std::fs::create_dir_all(apps.join("Visual Studio Code.app")).unwrap();
    let (args, editor) = env_check::open_args(path, Some(7), &[mac.root.join("none"), apps]);
    assert_eq!(editor, Some("Visual Studio Code"));
    assert_eq!(args, vec!["vscode://file/Users/someone/My%20Home/.zshrc:7".to_string()]);
}
