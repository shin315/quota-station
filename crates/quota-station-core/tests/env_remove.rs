// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Claude Code's settings and PowerShell functions in the check, and "Gỡ"
//! with its "Hoàn tác" (M14), on fake home folders in a temporary folder.
//! Nothing here reads or writes the real home, its profiles or Claude
//! Code's settings. The places that are no file (launchd, the registry)
//! are answered by a fake, so every test runs on macOS and on Windows.
//!
//! The trap: a value planted in every file must never reach a check, a
//! preview, an error or anything the app keeps in its own folder, apart
//! from the undo record, which holds the one cut line and nothing else.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
use quota_station_core::agents::env_check::{self, Check, Place};
use quota_station_core::agents::env_remove::{self, Refusal, RemoveError, UndoError};
use quota_station_core::app_dir::AppDir;

const TRAP: &str = "sk-ant-TRAP-5e1d77c2a09b";
const NOW: i64 = 1_768_547_700;

struct Home {
    home: PathBuf,
    app: AppDir,
}

impl Home {
    fn new(tag: &str) -> Home {
        let root = temp_dir(tag);
        let home = root.join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::create_dir_all(home.join("Documents/WindowsPowerShell")).unwrap();
        let app = AppDir::new(root.join("app"));
        std::fs::create_dir_all(app.path()).unwrap();
        Home { home, app }
    }

    fn path(&self, place: Place) -> PathBuf {
        self.home.join(place.file().unwrap())
    }

    fn put(&self, place: Place, bytes: impl AsRef<[u8]>) {
        std::fs::write(self.path(place), bytes).unwrap();
    }

    fn bytes(&self, place: Place) -> Vec<u8> {
        std::fs::read(self.path(place)).unwrap()
    }

    /// Every file place of both systems; nothing set elsewhere.
    fn check(&self) -> Check {
        let places: Vec<Place> =
            Place::ALL.into_iter().chain(Place::WINDOWS).filter(|p| p.file().is_some()).collect();
        let mut unique = places.clone();
        unique.sort();
        unique.dedup();
        env_check::check_with(&self.home, &unique, &|_, _| Ok(false))
    }

    fn found(&self) -> Vec<(String, Option<u32>, Option<String>)> {
        self.check().found.into_iter().map(|f| (f.id, f.line, f.only_in)).collect()
    }

    /// Previews and removes `id`.
    fn remove(&self, id: &str) -> Result<(), RemoveError> {
        let plan = env_remove::plan(&self.home, id).expect("a plan");
        env_remove::remove(&self.app, plan, NOW)
    }

    fn undo(&self, id: &str) -> Result<(), UndoError> {
        env_remove::undo(&self.home, &self.app, id)
    }
}

fn utf16le(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        out.extend(u.to_le_bytes());
    }
    out
}

fn no_trap(what: &str, text: &str) {
    assert!(!text.contains(TRAP) && !text.contains("TRAP"), "{what} holds the value: {text}");
}

/// Every file under `dir`, read whole.
fn every_file(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(every_file(&p));
        } else {
            out.push((p.clone(), std::fs::read(&p).unwrap()));
        }
    }
    out
}

/// What the owner's PC had: the token and the address in Claude Code's
/// settings, and a profile function that sets them only when called.
const SETTINGS: &str = r#"{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "sk-ant-TRAP-5e1d77c2a09b",
    "ANTHROPIC_BASE_URL": "https://relay.example.com",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "made-up-model",
    "OPENAI_API_KEY": "sk-ant-TRAP-5e1d77c2a09b"
  },
  "permissions": {
    "allow": ["Bash(git status)"]
  }
}
"#;

const PROFILE: &str = "Set-Alias g git\r\n\
function ccd {\r\n\
    # Claude Code through the relay\r\n\
    $env:ANTHROPIC_BASE_URL = \"https://relay.example.com\"\r\n\
    $env:ANTHROPIC_AUTH_TOKEN = \"sk-ant-TRAP-5e1d77c2a09b\"\r\n\
    claude @args\r\n\
}\r\n\
$env:CODEX_HOME = 'D:\\codex'\r\n";

#[test]
fn claude_code_settings_and_their_local_twin_are_checked() {
    let h = Home::new("envrm-settings");
    h.put(Place::ClaudeSettings, SETTINGS);
    h.put(
        Place::ClaudeLocal,
        "{\n  \"env\": { \"ANTHROPIC_API_KEY\": \"sk-ant-TRAP-5e1d77c2a09b\" }\n}\n",
    );
    let c = h.check();
    let got: Vec<(&str, Option<u32>)> = c.found.iter().map(|f| (f.id.as_str(), f.line)).collect();
    assert_eq!(
        got,
        [
            ("ANTHROPIC_API_KEY@claudeLocalSettings", Some(2)),
            ("ANTHROPIC_AUTH_TOKEN@claudeSettings", Some(4)),
            ("ANTHROPIC_BASE_URL@claudeSettings", Some(5)),
        ],
        "only Claude Code's own names count in its settings"
    );
    assert!(c.unread.is_empty(), "{:?}", c.unread);
    let token = &c.found[1];
    assert_eq!(token.at(), "~/.claude/settings.json dòng 4");
    assert_eq!(
        token.effect,
        "Ở mọi lần chạy, Claude Code gửi token này thay cho đăng nhập của tài khoản Claude đang chọn."
    );
    no_trap("the check", &serde_json::to_string(&c).unwrap());

    // Not JSON: said, not guessed at. No `env`, or an empty file: nothing.
    h.put(Place::ClaudeLocal, "{ \"env\": ");
    h.put(Place::ClaudeSettings, "{\"model\": \"opus\"}");
    let c = h.check();
    assert!(c.found.is_empty(), "{:?}", c.found);
    assert_eq!(c.unread, [Place::ClaudeLocal]);
    h.put(Place::ClaudeLocal, "");
    assert!(h.check().unread.is_empty());
}

#[test]
fn a_profile_line_inside_a_function_counts_only_when_it_is_called() {
    let h = Home::new("envrm-function");
    h.put(Place::PsProfile, PROFILE);
    let c = h.check();
    let got: Vec<(&str, Option<u32>, Option<&str>)> =
        c.found.iter().map(|f| (f.id.as_str(), f.line, f.only_in.as_deref())).collect();
    assert_eq!(
        got,
        [
            ("ANTHROPIC_AUTH_TOKEN@psProfile", Some(5), Some("ccd")),
            ("ANTHROPIC_BASE_URL@psProfile", Some(4), Some("ccd")),
            ("CODEX_HOME@psProfile", Some(8), None),
        ]
    );
    assert_eq!(c.found[0].at(), "$PROFILE dòng 5 (trong hàm ccd)");
    assert_eq!(
        c.found[1].effect,
        "Trong PowerShell, khi gọi hàm ccd, Claude Code gửi yêu cầu tới địa chỉ này thay cho máy chủ của Anthropic."
    );
    // The window gets `onlyIn` only when there is one.
    let json = serde_json::to_value(&c.found).unwrap();
    assert_eq!(json[0]["onlyIn"], "ccd");
    assert!(json[2].get("onlyIn").is_none());

    // A line that runs as the profile loads wins over one in a function;
    // one inside a here-string or a block comment sets nothing; braces in
    // strings and comments do not open a block.
    h.put(
        Place::PsProfile,
        "function Global:ccd\n{\n  $x = '{'  # }\n  $env:ANTHROPIC_API_KEY = 'a'\n}\n\
         $env:ANTHROPIC_API_KEY = 'b'\n\
         $note = @'\n$env:OPENAI_API_KEY = 'c'\n'@\n\
         <#\n$env:GEMINI_API_KEY = 'd'\n#>\n\
         if ($true) {\n  $env:CODEX_HOME = 'e'\n}\n\
         filter f { $_ }\n$env:GOOGLE_API_KEY = 'f'\n",
    );
    assert_eq!(
        h.found(),
        [
            ("ANTHROPIC_API_KEY@psProfile".to_string(), Some(6), None),
            ("CODEX_HOME@psProfile".into(), Some(14), None),
            ("GOOGLE_API_KEY@psProfile".into(), Some(17), None),
        ]
    );
    h.put(Place::PsProfile, "function Global:ccd\n{\n  $env:ANTHROPIC_API_KEY = 'a'\n}\n");
    assert_eq!(
        h.found(),
        [("ANTHROPIC_API_KEY@psProfile".to_string(), Some(3), Some("Global:ccd".into()))]
    );
}

#[test]
fn a_profile_with_a_byte_order_mark_or_in_utf16_is_read() {
    let h = Home::new("envrm-bom");
    h.put(
        Place::PsProfile,
        [b"\xEF\xBB\xBF".as_slice(), b"$env:ANTHROPIC_API_KEY = 'x'\r\n"].concat(),
    );
    assert_eq!(h.found(), [("ANTHROPIC_API_KEY@psProfile".to_string(), Some(1), None)]);
    h.put(Place::PsProfile, utf16le("# mine\r\n$env:OPENAI_API_KEY = 'x'\r\n"));
    assert_eq!(h.found(), [("OPENAI_API_KEY@psProfile".to_string(), Some(2), None)]);
    // UTF-16 that does not decode is said, not guessed at.
    h.put(Place::PsProfile, [0xFF, 0xFE, 0x24]);
    assert_eq!(h.check().unread, [Place::PsProfile]);
}

#[test]
fn the_preview_masks_the_value_and_names_the_line() {
    let h = Home::new("envrm-preview");
    h.put(Place::PsProfile, PROFILE);
    h.put(Place::ClaudeSettings, SETTINGS);
    h.put(Place::Zshrc, "export OPENAI_API_KEY=\"sk-ant-TRAP-5e1d77c2a09b\" # work\n");
    let p = env_remove::plan(&h.home, "ANTHROPIC_AUTH_TOKEN@psProfile").unwrap();
    assert_eq!(
        (p.preview.line, p.preview.only_in.as_deref(), p.preview.shown.as_str(), p.preview.refusal),
        (5, Some("ccd"), "$env:ANTHROPIC_AUTH_TOKEN = ••••••", None)
    );
    let j = env_remove::plan(&h.home, "ANTHROPIC_AUTH_TOKEN@claudeSettings").unwrap();
    assert_eq!((j.preview.line, j.preview.shown.as_str()), (4, "\"ANTHROPIC_AUTH_TOKEN\": ••••••"));
    let z = env_remove::plan(&h.home, "OPENAI_API_KEY@zshrc").unwrap();
    assert_eq!(z.preview.shown, "export OPENAI_API_KEY=••••••");
    for plan in [&p, &j, &z] {
        no_trap("the preview", &serde_json::to_string(&plan.preview).unwrap());
        no_trap("the plan's debug form", &format!("{plan:?}"));
    }
    // Not set there, or no file there: nothing to plan.
    assert_eq!(env_remove::plan(&h.home, "GEMINI_API_KEY@zshrc").unwrap_err(), Refusal::Gone);
    assert_eq!(env_remove::plan(&h.home, "GEMINI_API_KEY@zprofile").unwrap_err(), Refusal::Gone);
    assert_eq!(env_remove::plan(&h.home, "GEMINI_API_KEY@launchctl").unwrap_err(), Refusal::NoFile);
    assert_eq!(env_remove::plan(&h.home, "PATH@zshrc").unwrap_err(), Refusal::Gone);
}

/// Each case: the file, the id, and the file after "Gỡ".
#[test]
fn remove_takes_out_exactly_the_line_in_its_encoding_and_line_endings() {
    let cases: Vec<(Place, Vec<u8>, &str, Vec<u8>)> = vec![
        // LF; the last line that sets it is the one taken.
        (
            Place::Zshrc,
            b"alias ll='ls -l'\nexport ANTHROPIC_API_KEY=old\nexport ANTHROPIC_API_KEY=\"sk-TRAP\"\necho hi\n".to_vec(),
            "ANTHROPIC_API_KEY@zshrc",
            b"alias ll='ls -l'\nexport ANTHROPIC_API_KEY=old\necho hi\n".to_vec(),
        ),
        // CRLF, and a last line with no line ending: the one before it goes.
        (
            Place::PsProfile,
            b"Set-Alias g git\r\n$env:ANTHROPIC_API_KEY = 'sk-TRAP'".to_vec(),
            "ANTHROPIC_API_KEY@psProfile",
            b"Set-Alias g git".to_vec(),
        ),
        // The only line.
        (Place::Profile, b"export CODEX_HOME=/x".to_vec(), "CODEX_HOME@profile", b"".to_vec()),
        // A byte order mark stays, and so do the other lines' bytes,
        // including ones that are not UTF-8 (a code page).
        (
            Place::PsProfile,
            [b"\xEF\xBB\xBF".as_slice(), b"$env:OPENAI_BASE_URL = 'x'\r\n# caf\xE9\r\n"].concat(),
            "OPENAI_BASE_URL@psProfile",
            [b"\xEF\xBB\xBF".as_slice(), b"# caf\xE9\r\n"].concat(),
        ),
        (
            Place::PsProfile,
            b"# caf\xE9\n$env:OPENAI_BASE_URL = 'x'\n".to_vec(),
            "OPENAI_BASE_URL@psProfile",
            b"# caf\xE9\n".to_vec(),
        ),
        // UTF-16 with its mark, CRLF, inside a function.
        (
            Place::Pwsh,
            utf16le("function ccd {\r\n  $env:ANTHROPIC_AUTH_TOKEN = \"sk-TRAP\"\r\n  claude\r\n}\r\n"),
            "ANTHROPIC_AUTH_TOKEN@pwshProfile",
            utf16le("function ccd {\r\n  claude\r\n}\r\n"),
        ),
    ];
    for (n, (place, before, id, after)) in cases.into_iter().enumerate() {
        let h = Home::new(&format!("envrm-line-{n}"));
        std::fs::create_dir_all(h.path(place).parent().unwrap()).unwrap();
        h.put(place, &before);
        h.remove(id).unwrap_or_else(|e| panic!("case {n}: {e:?}"));
        assert_eq!(h.bytes(place), after, "case {n}");
        // Then "Hoàn tác" gives back every byte.
        h.undo(id).unwrap_or_else(|e| panic!("case {n}: {e:?}"));
        assert_eq!(h.bytes(place), before, "case {n} undone");
        assert!(!env_remove::can_undo(&h.app, id), "case {n}: the record goes");
    }
}

/// Each case: the settings, the id, and the settings after "Gỡ".
#[test]
fn remove_takes_out_exactly_the_key_and_keeps_the_json_valid() {
    let cases: Vec<(&str, &str, &str)> = vec![
        // The first of several: up to the next key.
        (
            "{\n  \"env\": {\n    \"ANTHROPIC_AUTH_TOKEN\": \"x\",\n    \"ANTHROPIC_BASE_URL\": \"y\"\n  }\n}\n",
            "ANTHROPIC_AUTH_TOKEN@claudeSettings",
            "{\n  \"env\": {\n    \"ANTHROPIC_BASE_URL\": \"y\"\n  }\n}\n",
        ),
        // The last of several: with the comma before it.
        (
            "{\r\n  \"env\": {\r\n    \"A\": \"1\",\r\n    \"ANTHROPIC_BASE_URL\": \"y\"\r\n  },\r\n  \"model\": \"opus\"\r\n}\r\n",
            "ANTHROPIC_BASE_URL@claudeSettings",
            "{\r\n  \"env\": {\r\n    \"A\": \"1\"\r\n  },\r\n  \"model\": \"opus\"\r\n}\r\n",
        ),
        // A middle one, on one line.
        (
            "{\"env\":{\"A\":\"1\",\"ANTHROPIC_API_KEY\":\"k\",\"B\":\"2\"}}",
            "ANTHROPIC_API_KEY@claudeSettings",
            "{\"env\":{\"A\":\"1\",\"B\":\"2\"}}",
        ),
        // The only one: `env` stays, empty.
        (
            "{\n  \"env\": {\n    \"CLAUDE_CODE_OAUTH_TOKEN\": \"t\"\n  }\n}",
            "CLAUDE_CODE_OAUTH_TOKEN@claudeLocalSettings",
            "{\n  \"env\": {}\n}",
        ),
        // A repeated key: the last, which JSON readers keep.
        (
            "{\"env\": {\"ANTHROPIC_API_KEY\": \"a\", \"ANTHROPIC_API_KEY\": \"b\"}}",
            "ANTHROPIC_API_KEY@claudeSettings",
            "{\"env\": {\"ANTHROPIC_API_KEY\": \"a\"}}",
        ),
    ];
    for (n, (before, id, after)) in cases.into_iter().enumerate() {
        let h = Home::new(&format!("envrm-json-{n}"));
        let place = env_check::parse_id(id).unwrap().1;
        h.put(place, before);
        h.remove(id).unwrap_or_else(|e| panic!("case {n}: {e:?}"));
        let got = String::from_utf8(h.bytes(place)).unwrap();
        assert_eq!(got, after, "case {n}");
        serde_json::from_str::<serde_json::Value>(&got).expect("still JSON");
        h.undo(id).unwrap();
        assert_eq!(h.bytes(place), before.as_bytes(), "case {n} undone");
    }
    // The owner's settings: only the token goes; the model names, the
    // other agent's key and the permissions stay.
    let h = Home::new("envrm-json-owner");
    let bom = [b"\xEF\xBB\xBF".as_slice(), SETTINGS.as_bytes()].concat();
    h.put(Place::ClaudeSettings, &bom);
    h.remove("ANTHROPIC_AUTH_TOKEN@claudeSettings").unwrap();
    h.remove("ANTHROPIC_BASE_URL@claudeSettings").unwrap();
    let after = h.bytes(Place::ClaudeSettings);
    assert!(after.starts_with(b"\xEF\xBB\xBF"), "the mark stays");
    let expect = SETTINGS
        .replace("    \"ANTHROPIC_AUTH_TOKEN\": \"sk-ant-TRAP-5e1d77c2a09b\",\n", "")
        .replace("    \"ANTHROPIC_BASE_URL\": \"https://relay.example.com\",\n", "");
    assert_eq!(&after[3..], expect.as_bytes());
    assert!(h.check().found.is_empty());
}

#[test]
fn a_line_that_is_not_a_plain_assignment_is_refused_and_left_alone() {
    let cases: Vec<(Place, &str, &str, Refusal)> = vec![
        (
            Place::Zshrc,
            "export ANTHROPIC_API_KEY=\"a\nb\"\n",
            "ANTHROPIC_API_KEY@zshrc",
            Refusal::Multiline,
        ),
        (
            Place::Zshrc,
            "export ANTHROPIC_API_KEY=a \\\n  b\n",
            "ANTHROPIC_API_KEY@zshrc",
            Refusal::Continued,
        ),
        (
            Place::Zshrc,
            "echo a \\\nANTHROPIC_API_KEY=a\n",
            "ANTHROPIC_API_KEY@zshrc",
            Refusal::Continued,
        ),
        (
            Place::Zshrc,
            "export ANTHROPIC_API_KEY=a OPENAI_API_KEY=b\n",
            "ANTHROPIC_API_KEY@zshrc",
            Refusal::Several,
        ),
        (
            Place::PsProfile,
            "$env:ANTHROPIC_API_KEY = @'\nx\n'@\n",
            "ANTHROPIC_API_KEY@psProfile",
            Refusal::HereString,
        ),
        (
            Place::PsProfile,
            "$env:ANTHROPIC_API_KEY = 'a' + `\n  'b'\n",
            "ANTHROPIC_API_KEY@psProfile",
            Refusal::Continued,
        ),
        (
            Place::PsProfile,
            "$env:ANTHROPIC_API_KEY = 'a'; $env:OPENAI_API_KEY = 'b'\n",
            "ANTHROPIC_API_KEY@psProfile",
            Refusal::Several,
        ),
        (
            Place::PsProfile,
            "function f {\n$env:ANTHROPIC_API_KEY = 'a' }\n",
            "ANTHROPIC_API_KEY@psProfile",
            Refusal::Several,
        ),
    ];
    for (n, (place, text, id, why)) in cases.into_iter().enumerate() {
        let h = Home::new(&format!("envrm-refuse-{n}"));
        h.put(place, text);
        let plan = env_remove::plan(&h.home, id).unwrap_or_else(|e| panic!("case {n}: {e:?}"));
        assert_eq!(plan.preview.refusal, Some(why), "case {n}");
        assert_eq!(
            env_remove::remove(&h.app, plan, NOW),
            Err(RemoveError::Refused { refusal: why }),
            "case {n}"
        );
        assert_eq!(h.bytes(place), text.as_bytes(), "case {n}: untouched");
        assert!(every_file(h.app.path()).is_empty(), "case {n}: nothing kept");
    }
    // Settings that are not JSON.
    let h = Home::new("envrm-refuse-json");
    h.put(Place::ClaudeSettings, "{\"env\": {\"ANTHROPIC_API_KEY\": 1,}}");
    assert_eq!(
        env_remove::plan(&h.home, "ANTHROPIC_API_KEY@claudeSettings").unwrap_err(),
        Refusal::Json
    );
}

#[test]
fn a_file_changed_after_the_preview_is_not_written() {
    let h = Home::new("envrm-changed");
    h.put(Place::Zshrc, "export ANTHROPIC_API_KEY=a\n");
    let plan = env_remove::plan(&h.home, "ANTHROPIC_API_KEY@zshrc").unwrap();
    h.put(Place::Zshrc, "export ANTHROPIC_API_KEY=a\nexport OPENAI_API_KEY=b\n");
    assert_eq!(env_remove::remove(&h.app, plan, NOW), Err(RemoveError::Changed));
    assert_eq!(h.bytes(Place::Zshrc), b"export ANTHROPIC_API_KEY=a\nexport OPENAI_API_KEY=b\n");
    assert!(
        !env_remove::can_undo(&h.app, "ANTHROPIC_API_KEY@zshrc"),
        "no record of a removal that did not happen"
    );
    // Nothing left beside the file.
    let beside: Vec<_> =
        std::fs::read_dir(&h.home).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(beside.len(), 3, "{beside:?}");
}

#[test]
fn undo_is_refused_once_the_file_changed_and_kept_for_later() {
    let h = Home::new("envrm-undo-changed");
    h.put(Place::Zshrc, "export ANTHROPIC_API_KEY=a\necho hi\n");
    h.remove("ANTHROPIC_API_KEY@zshrc").unwrap();
    assert_eq!(h.bytes(Place::Zshrc), b"echo hi\n");
    h.put(Place::Zshrc, "echo hi\necho more\n");
    assert_eq!(h.undo("ANTHROPIC_API_KEY@zshrc"), Err(UndoError::Changed));
    assert_eq!(h.bytes(Place::Zshrc), b"echo hi\necho more\n");
    // The owner takes their change back: undo works again.
    h.put(Place::Zshrc, "echo hi\n");
    h.undo("ANTHROPIC_API_KEY@zshrc").unwrap();
    assert_eq!(h.bytes(Place::Zshrc), b"export ANTHROPIC_API_KEY=a\necho hi\n");
    assert_eq!(h.undo("ANTHROPIC_API_KEY@zshrc"), Err(UndoError::Missing));
    // A file gone since: changed, nothing written.
    h.remove("ANTHROPIC_API_KEY@zshrc").unwrap();
    std::fs::remove_file(h.path(Place::Zshrc)).unwrap();
    assert_eq!(h.undo("ANTHROPIC_API_KEY@zshrc"), Err(UndoError::Changed));
    assert!(!h.path(Place::Zshrc).exists());
}

#[test]
fn the_record_holds_the_cut_line_only_in_the_apps_own_folder_for_its_owner() {
    let h = Home::new("envrm-record");
    h.put(Place::PsProfile, PROFILE);
    h.put(Place::ClaudeSettings, SETTINGS);
    h.remove("ANTHROPIC_AUTH_TOKEN@psProfile").unwrap();
    h.remove("ANTHROPIC_AUTH_TOKEN@claudeSettings").unwrap();
    // Nothing beside the owner's files but the files.
    for dir in [h.home.join(".claude"), h.home.join("Documents/WindowsPowerShell")] {
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
    }
    // The records: one per place, in the app's folder (on Windows its
    // local twin, which a test folder is its own), each the cut span alone.
    let files = every_file(h.app.path());
    assert_eq!(files.len(), 2, "{:?}", files.iter().map(|f| &f.0).collect::<Vec<_>>());
    for (path, bytes) in &files {
        assert!(path.starts_with(h.app.path().join("agents/env-undo")), "{}", path.display());
        let text = String::from_utf8(bytes.clone()).unwrap();
        // The span is base64: the value is not there as text, and the
        // rest of the file (the other key, the address) is not there at all.
        assert!(!text.contains("TRAP") && !text.contains("relay"), "{text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(path).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(
                std::fs::metadata(path.parent().unwrap()).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
    // A record older than a week goes at the next removal.
    h.put(Place::Zshrc, "export CODEX_HOME=/x\n");
    let plan = env_remove::plan(&h.home, "CODEX_HOME@zshrc").unwrap();
    env_remove::remove(&h.app, plan, NOW + env_remove::KEEP_FOR + 1).unwrap();
    assert_eq!(every_file(h.app.path()).len(), 1);
    assert!(env_remove::can_undo(&h.app, "CODEX_HOME@zshrc"));
}

#[cfg(unix)]
#[test]
fn a_symlinked_file_keeps_its_link_and_its_mode() {
    use std::os::unix::fs::PermissionsExt;
    let h = Home::new("envrm-link");
    let real = h.home.join("dotfiles-zshrc");
    std::fs::write(&real, "export ANTHROPIC_API_KEY=a\nalias g=git\n").unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::os::unix::fs::symlink(&real, h.path(Place::Zshrc)).unwrap();
    h.remove("ANTHROPIC_API_KEY@zshrc").unwrap();
    assert!(std::fs::symlink_metadata(h.path(Place::Zshrc)).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(&real).unwrap(), b"alias g=git\n");
    assert_eq!(std::fs::metadata(&real).unwrap().permissions().mode() & 0o777, 0o644);
}
