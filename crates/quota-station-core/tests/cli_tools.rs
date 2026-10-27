// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Cập nhật Agent: how each install of Claude Code, Codex and agy is told
//! apart, the command each gets, the sessions that hold Codex back, the
//! newest versions from a local fake of each feed, and the update itself
//! run against fake `brew`, `claude`, `codex` and `agy` scripts. No test
//! runs a real update or reaches a real feed.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::http::FakeHttp;
use common::os::FakeProcesses;
use common::temp_dir;
use quota_station_core::alerts::{self, Effect, Kind, Memory};
use quota_station_core::cli_tools::{
    self, check, diagnose, layout, row, running, Failure, Feeds, Found, HandsOff, Install, Kept,
    Last, Refused, Tool,
};
use quota_station_core::os::Process;

fn found(tool: Tool, path: &Path, target: &Path, home: &Path, version: &str) -> Found {
    Found {
        tool,
        path: path.into(),
        target: target.into(),
        version: Some(version.into()),
        layout: layout(tool, path, target, home),
    }
}

fn process(command: &str) -> Process {
    Process { pid: 1, tty: Some("ttys001".into()), command: command.into() }
}

// ===== How it was installed =====

#[test]
fn claude_from_its_installer_updates_with_claude_update() {
    let home = temp_dir("cli-home");
    let path = home.join(".local/bin/claude");
    let l = layout(Tool::Claude, &path, &home.join(".local/share/claude/versions/2.1.311"), &home);
    assert_eq!(l.install, Some(Install::Native));
    assert_eq!(l.hands_off, None);
    assert_eq!(l.command.as_deref(), Some("claude update"));
    assert_eq!(l.update, Some(vec![path.to_string_lossy().into_owned(), "update".into()]));
}

#[test]
fn a_homebrew_cask_updates_with_the_brew_beside_its_caskroom() {
    let home = temp_dir("cli-home");
    let prefix = home.join("brew-prefix");
    let target = prefix.join("Caskroom/codex/0.163.0/codex");
    let l = layout(Tool::Codex, &prefix.join("bin/codex"), &target, &home);
    assert_eq!(l.install, Some(Install::Homebrew));
    assert_eq!(l.cask.as_deref(), Some("codex"));
    assert_eq!(l.command.as_deref(), Some("brew upgrade --cask codex"));
    let brew = prefix.join("bin").join("brew").to_string_lossy().into_owned();
    assert_eq!(l.update, Some(vec![brew, "upgrade".into(), "--cask".into(), "codex".into()]));
    assert_eq!(l.root, Some(prefix.join("Caskroom/codex")));

    let claude = prefix.join("Caskroom/claude-code@latest/2.1.311/claude");
    let l = layout(Tool::Claude, &prefix.join("bin/claude"), &claude, &home);
    assert_eq!(l.command.as_deref(), Some("brew upgrade --cask claude-code@latest"));
}

#[test]
fn codex_from_npm_updates_with_the_npm_of_its_prefix() {
    let home = temp_dir("cli-home");
    let prefix = home.join(".nvm/versions/node/v24.18.0");
    let target = prefix.join("lib/node_modules/@openai/codex/bin/codex.js");
    let l = layout(Tool::Codex, &prefix.join("bin/codex"), &target, &home);
    assert_eq!(l.install, Some(Install::Npm));
    assert_eq!(l.command.as_deref(), Some("npm install -g @openai/codex@latest"));
    assert_eq!(l.update.unwrap()[0], prefix.join("bin").join("npm").to_string_lossy());
    assert_eq!(l.root, Some(prefix.join("lib/node_modules/@openai/codex")));
}

#[test]
fn codex_from_its_install_script_updates_with_codex_update() {
    let home = temp_dir("cli-home");
    let target = home.join(".codex/packages/standalone/releases/0.163.0/codex");
    let l = layout(Tool::Codex, &home.join(".local/bin/codex"), &target, &home);
    assert_eq!(l.install, Some(Install::Standalone));
    assert_eq!(l.command.as_deref(), Some("codex update"));
}

#[test]
fn agy_from_its_install_script_updates_with_agy_update() {
    let home = temp_dir("cli-home");
    let path = home.join(".local/bin/agy");
    let l = layout(Tool::Agy, &path, &path, &home);
    assert_eq!(l.install, Some(Install::Script));
    assert_eq!(l.command.as_deref(), Some("agy update"));
    assert_eq!(l.root, Some(home.join(".local/bin")));
}

#[test]
fn claude_from_npm_is_left_to_the_owner_with_claude_install() {
    let home = temp_dir("cli-home");
    let target = home.join("prefix/lib/node_modules/@anthropic-ai/claude-code/cli.js");
    let l = layout(Tool::Claude, &home.join("prefix/bin/claude"), &target, &home);
    assert_eq!(l.hands_off, Some(HandsOff::Npm));
    assert_eq!(l.command.as_deref(), Some("claude install"));
    assert_eq!(l.update, None);
    let local = layout(
        Tool::Claude,
        &home.join(".claude/local/claude"),
        &home.join(".claude/local/claude"),
        &home,
    );
    assert_eq!(local.hands_off, Some(HandsOff::Npm));
}

#[test]
fn a_copy_inside_an_app_is_that_apps_to_update() {
    let home = temp_dir("cli-home");
    let target = home.join("Applications/Codex.app/Contents/Resources/codex");
    let l = layout(Tool::Codex, &target, &target, &home);
    assert_eq!(l.hands_off, Some(HandsOff::Bundled));
    assert_eq!(l.bundled_by.as_deref(), Some("Codex.app"));
    assert_eq!((l.update, l.command), (None, None));
}

#[test]
fn a_layout_the_app_does_not_know_gets_no_command() {
    let home = temp_dir("cli-home");
    let claude = home.join("bin/claude");
    assert_eq!(layout(Tool::Claude, &claude, &claude, &home).hands_off, Some(HandsOff::Unknown));
    // bun's global folder is not npm's `lib/node_modules`.
    let bun = home.join(".bun/install/global/node_modules/@openai/codex/bin/codex.js");
    let l = layout(Tool::Codex, &home.join(".bun/bin/codex"), &bun, &home);
    assert_eq!((l.hands_off, l.update), (Some(HandsOff::Unknown), None));
}

#[test]
fn a_folder_that_needs_an_administrator_leaves_the_command_to_terminal() {
    let home = temp_dir("cli-home");
    let target = PathBuf::from("/usr/local/lib/node_modules/@openai/codex/bin/codex.js");
    let l = cli_tools::unwritable(layout(
        Tool::Codex,
        Path::new("/usr/local/bin/codex"),
        &target,
        &home,
    ));
    assert_eq!(l.hands_off, Some(HandsOff::Unwritable));
    assert_eq!(l.command.as_deref(), Some("sudo npm install -g @openai/codex@latest"));
    assert_eq!(l.update, None);
}

#[cfg(unix)]
#[test]
fn inspecting_a_tool_reads_its_version_and_whether_its_folder_is_writable() {
    use std::os::unix::fs::PermissionsExt;
    let home = temp_dir("cli-home");
    let agy = common::script(&home.join(".local/bin/agy"), "echo 1.4.7\n");
    let f = cli_tools::inspect(Tool::Agy, &agy, &home);
    assert_eq!(f.version.as_deref(), Some("1.4.7"));
    assert_eq!(f.layout.install, Some(Install::Script));
    assert_eq!(f.layout.hands_off, None);

    let locked = home.join("locked");
    let agy = common::script(&locked.join("agy"), "echo 1.4.7\n");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
    let f = cli_tools::inspect(Tool::Agy, &agy, &home);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(f.layout.hands_off, Some(HandsOff::Unwritable));
}

// ===== Its sessions =====

#[test]
fn sessions_count_this_install_and_not_other_copies() {
    let home = temp_dir("cli-home");
    let prefix = home.join("brew-prefix");
    let target = prefix.join("Caskroom/codex/0.163.0/codex");
    let f = found(Tool::Codex, &prefix.join("bin/codex"), &target, &home, "0.163.0");
    let other = home.join(".openclaw/npm/node_modules/@openai/codex-darwin-arm64/bin/codex");
    let processes = [
        process("codex --resume"),
        process(&format!("{} app-server --managed-daemon", prefix.join("bin/codex").display())),
        process(&format!("{} exec", target.display())),
        process(&format!("{} app-server", other.display())),
        process("/Applications/Codex.app/Contents/Resources/codex app-server"),
        process("node /usr/local/bin/codex"),
        process("claude --resume"),
        process("./codex"),
    ];
    assert_eq!(running(&f, &processes), 3);
}

// ===== The newest version =====

/// Each tool, installed one way, under a temporary home.
fn installs(home: &Path) -> Vec<Found> {
    let native = home.join(".local/share/claude/versions/2.1.311");
    let cask = home.join("brew/Caskroom/codex/0.163.0/codex");
    let agy = home.join(".local/bin/agy");
    vec![
        found(Tool::Claude, &home.join(".local/bin/claude"), &native, home, "2.1.311"),
        found(Tool::Codex, &home.join("brew/bin/codex"), &cask, home, "0.163.0"),
        found(Tool::Agy, &agy, &agy, home, "1.4.7"),
    ]
}

fn feeds() -> FakeHttp {
    let http = FakeHttp::start();
    http.answer("/claude/claude-code-releases/latest", 200, "2.1.312\n");
    http.answer("/claude/claude-code-releases/stable", 200, "2.1.308\n");
    http.answer("/brew/api/cask/codex.json", 200, r#"{"token":"codex","version":"0.164.0"}"#);
    http.answer(
        "/npm/@openai/codex/latest",
        200,
        r#"{"name":"@openai/codex","version":"0.165.0"}"#,
    );
    http.answer(
        "/github/repos/openai/codex/releases/latest",
        200,
        r#"{"tag_name":"rust-v0.166.0"}"#,
    );
    // agy's manifest for the system the tests run on: `darwin_arm64` on a Mac.
    let os = if cfg!(target_os = "macos") { "darwin" } else { std::env::consts::OS };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
    let agy = format!("/agy/manifests/{os}_{arch}.json");
    http.answer(
        &agy,
        200,
        r#"{"version":"1.4.8","url":"https://example.invalid/agy","sha512":"x"}"#,
    );
    http
}

#[test]
fn each_install_asks_where_it_is_published() {
    let home = temp_dir("cli-home");
    let http = feeds();
    let kept = check(&Feeds::at(&http.base), &installs(&home), &home, &Kept::default(), 1000);
    assert_eq!(kept.latest.get(&Tool::Claude).map(String::as_str), Some("2.1.312"));
    assert_eq!(kept.latest.get(&Tool::Codex).map(String::as_str), Some("0.164.0"));
    assert_eq!(kept.latest.get(&Tool::Agy).map(String::as_str), Some("1.4.8"));
    assert_eq!((kept.checked_at, kept.offline), (Some(1000), false));

    let npm = home.join("p/lib/node_modules/@openai/codex/bin/codex.js");
    let npm = found(Tool::Codex, &home.join("p/bin/codex"), &npm, &home, "0.163.0");
    let standalone = home.join(".codex/packages/standalone/releases/0.163.0/codex");
    let standalone =
        found(Tool::Codex, &home.join(".local/bin/codex"), &standalone, &home, "0.163.0");
    let feeds = Feeds::at(&http.base);
    assert_eq!(
        cli_tools::newest(&feeds, &npm, &home),
        cli_tools::Newest::Version("0.165.0".into())
    );
    assert_eq!(
        cli_tools::newest(&feeds, &standalone, &home),
        cli_tools::Newest::Version("0.166.0".into())
    );
}

#[test]
fn claude_follows_the_stable_channel_when_its_settings_say_so() {
    let home = temp_dir("cli-home");
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    std::fs::write(
        home.join(".claude/settings.json"),
        r#"{"autoUpdatesChannel":"stable","env":{"X":"1"}}"#,
    )
    .unwrap();
    let http = feeds();
    let kept = check(&Feeds::at(&http.base), &installs(&home)[..1], &home, &Kept::default(), 1000);
    assert_eq!(kept.latest.get(&Tool::Claude).map(String::as_str), Some("2.1.308"));
}

#[test]
fn an_install_left_to_the_owner_asks_no_feed() {
    let home = temp_dir("cli-home");
    let bundled = home.join("Applications/Codex.app/Contents/Resources/codex");
    let f = found(Tool::Codex, &bundled, &bundled, &home, "0.163.0");
    let http = feeds();
    assert_eq!(cli_tools::newest(&Feeds::at(&http.base), &f, &home), cli_tools::Newest::NoFeed);
    assert!(http.requests().is_empty());
}

#[test]
fn a_check_that_reaches_no_feed_keeps_what_the_last_one_found() {
    let home = temp_dir("cli-home");
    let http = feeds();
    http.answer_all(503, "down");
    let mut before = Kept::default();
    before.latest.insert(Tool::Codex, "0.164.0".into());
    before.last.insert(Tool::Agy, Last::done("1.4.6", "1.4.7", 900));
    let kept = check(&Feeds::at(&http.base), &installs(&home), &home, &before, 1000);
    assert!(kept.offline);
    assert_eq!(kept.latest.get(&Tool::Codex).map(String::as_str), Some("0.164.0"));
    assert_eq!(kept.latest.get(&Tool::Claude), None);
    assert_eq!(kept.last, before.last);
    // An hour later, not a day, the daily check tries again.
    assert!(!kept.due(1000 + 60 * 60 - 1));
    assert!(kept.due(1000 + 60 * 60));
}

#[test]
fn a_check_is_due_a_day_after_one_that_worked() {
    let kept = Kept { checked_at: Some(1000), ..Kept::default() };
    assert!(Kept::default().due(0));
    assert!(!kept.due(1000 + 24 * 60 * 60 - 1));
    assert!(kept.due(1000 + 24 * 60 * 60));
}

// ===== The row =====

#[test]
fn a_row_shows_the_version_the_newest_and_the_path_followed() {
    let home = temp_dir("cli-home");
    let f = &installs(&home)[1];
    let mut kept = Kept::default();
    kept.latest.insert(Tool::Codex, "0.164.0".into());
    let r = row(Tool::Codex, Some(f), &kept, 2, false, &home);
    assert!(r.newer);
    assert_eq!(r.running, 2);
    assert!(!r.safe_while_running);
    assert_eq!(r.command.as_deref(), Some("brew upgrade --cask codex"));
    assert_eq!(r.path.as_deref(), Some("~/brew/bin/codex → ~/brew/Caskroom/codex/0.163.0/codex"));

    let missing = row(Tool::Agy, None, &kept, 0, false, &home);
    assert_eq!((missing.version, missing.newer, missing.path), (None, false, None));
    assert!(missing.safe_while_running);
}

#[test]
fn only_codex_waits_for_its_sessions() {
    assert!(Tool::Claude.safe_while_running());
    assert!(Tool::Agy.safe_while_running());
    assert!(!Tool::Codex.safe_while_running());
}

// ===== The update =====

#[test]
fn an_install_left_to_the_owner_is_never_updated() {
    let home = temp_dir("cli-home");
    let bundled = home.join("Applications/Codex.app/Contents/Resources/codex");
    let f = found(Tool::Codex, &bundled, &bundled, &home, "0.163.0");
    let log = home.join("log/codex.log");
    let done =
        cli_tools::update(&f, &FakeProcesses::none(), &log, &home, Duration::from_secs(5), 1000);
    assert_eq!(done, Err(Refused::HandsOff));
    assert!(!log.exists());
}

#[test]
fn the_output_says_why_an_update_failed() {
    let home = PathBuf::from("/Users/lan");
    let (f, d) = diagnose("npm error code EACCES\nnpm error syscall mkdir\nnpm error Error: EACCES: permission denied, mkdir '/usr/local/lib/node_modules/@openai'\n", &home);
    assert_eq!((f, d.as_str()), (Failure::Permission, "/usr/local/lib/node_modules/@openai"));
    let (f, _) = diagnose("Error: A `brew upgrade --cask codex` process has already locked /opt/homebrew/Caskroom/codex.\n", &home);
    assert_eq!(f, Failure::Busy);
    let (f, _) =
        diagnose("another background updater process is already active (update.lock)", &home);
    assert_eq!(f, Failure::Busy);
    let (f, _) = diagnose("curl: (6) Could not resolve host: formulae.brew.sh\n", &home);
    assert_eq!(f, Failure::Offline);
    let (f, d) = diagnose(
        "Downloading…\nError: checksum mismatch in /Users/lan/Library/Caches/x\n\n",
        &home,
    );
    assert_eq!((f, d.as_str()), (Failure::Exit, "Error: checksum mismatch in ~/Library/Caches/x"));
}

#[test]
fn the_new_alert_names_each_tool_once_per_set_of_versions() {
    let mut memory = Memory::default();
    let one = [("Codex CLI".to_string(), "0.163.0".to_string(), "0.164.0".to_string())];
    let id = alerts::cli_alert(&mut memory, &one, 1000).unwrap();
    let card = &memory.record(&id).unwrap().alert;
    assert_eq!(card.kind, Kind::Cli);
    assert_eq!(card.title, "Codex CLI có bản 0.164.0");
    assert_eq!(card.body, "Codex CLI 0.163.0 → 0.164.0. App chỉ cập nhật khi bạn bấm.");
    assert_eq!(alerts::cli_alert(&mut memory, &one, 2000), None);

    let two = [one[0].clone(), ("Antigravity CLI".into(), "1.4.7".into(), "1.4.8".into())];
    let id = alerts::cli_alert(&mut memory, &two, 3000).unwrap();
    assert_eq!(memory.record(&id).unwrap().alert.title, "2 agent có bản mới");
    let effect = alerts::answer(&mut memory, &id, "cli:settings", 3100, time::UtcOffset::UTC);
    assert_eq!(effect, Ok(Effect::OpenCliSettings));
}

/// Fake `codex` reading its version from `version` beside a fake `brew`
/// that records its words and, unless told otherwise, writes the new one.
#[cfg(unix)]
struct Brew {
    home: PathBuf,
    found: Found,
    log: PathBuf,
}

#[cfg(unix)]
impl Brew {
    fn new(brew_body: &str) -> Brew {
        let home = temp_dir("cli-home");
        let prefix = home.join("brew");
        let cellar = prefix.join("Caskroom/codex/0.163.0");
        std::fs::create_dir_all(&cellar).unwrap();
        std::fs::write(home.join("version"), "0.163.0").unwrap();
        let codex = common::script(
            &cellar.join("codex"),
            &format!("printf 'codex-cli %s\\n' \"$(cat '{}')\"\n", home.join("version").display()),
        );
        let body = brew_body.replace("$HOME_DIR", &home.to_string_lossy());
        common::script(
            &prefix.join("bin/brew"),
            &format!("echo \"$@\" >> '{}/brew-ran'\n{body}", home.display()),
        );
        let found = cli_tools::inspect(Tool::Codex, &codex, &home);
        assert_eq!(found.layout.install, Some(Install::Homebrew));
        let log = home.join("logs/codex.log");
        Brew { home, found, log }
    }

    fn update(&self, processes: &FakeProcesses, timeout: Duration) -> Last {
        cli_tools::update(&self.found, processes, &self.log, &self.home, timeout, 5000).unwrap()
    }

    fn ran(&self) -> Option<String> {
        std::fs::read_to_string(self.home.join("brew-ran")).ok()
    }
}

#[cfg(unix)]
#[test]
fn an_update_runs_the_command_and_reads_the_new_version() {
    let b = Brew::new("echo '==> Upgrading codex'\nprintf 0.164.0 > \"$HOME_DIR/version\"\n");
    let last = b.update(&FakeProcesses::none(), Duration::from_secs(10));
    assert_eq!(last, Last::done("0.163.0", "0.164.0", 5000));
    assert_eq!(b.ran().as_deref(), Some("upgrade --cask codex\n"));
    let log = std::fs::read_to_string(&b.log).unwrap();
    assert!(log.starts_with("$ brew upgrade --cask codex\n"));
    assert!(log.contains("==> Upgrading codex") && log.contains("(exit 0)"));
}

#[cfg(unix)]
#[test]
fn codex_does_not_update_while_one_of_its_sessions_runs() {
    let b = Brew::new("printf 0.164.0 > \"$HOME_DIR/version\"\n");
    let open = FakeProcesses {
        all: Some(vec![process("codex --resume"), process("codex")]),
        ..FakeProcesses::none()
    };
    let last = b.update(&open, Duration::from_secs(10));
    assert_eq!(
        (last.ok, last.failure, last.detail.as_deref()),
        (false, Some(Failure::Started), Some("2"))
    );
    assert_eq!(b.ran(), None);
}

#[cfg(unix)]
#[test]
fn codex_does_not_update_when_its_sessions_cannot_be_told() {
    let b = Brew::new("printf 0.164.0 > \"$HOME_DIR/version\"\n");
    let unknown = FakeProcesses { all: None, ..FakeProcesses::none() };
    assert_eq!(b.update(&unknown, Duration::from_secs(10)).failure, Some(Failure::Sessions));
    assert_eq!(b.ran(), None);
}

#[cfg(unix)]
#[test]
fn a_failed_update_keeps_its_reason_and_its_log() {
    let b = Brew::new(
        "echo 'Error: Permission denied @ rb_sysopen - /opt/homebrew/Caskroom/codex' >&2\nexit 1\n",
    );
    let last = b.update(&FakeProcesses::none(), Duration::from_secs(10));
    assert_eq!((last.ok, last.failure), (false, Some(Failure::Permission)));
    assert_eq!(last.detail.as_deref(), Some("/opt/homebrew/Caskroom/codex"));
    assert_eq!(last.log.as_deref(), Some("~/logs/codex.log"));
    assert!(std::fs::read_to_string(&b.log).unwrap().contains("(exit 1)"));
}

#[cfg(unix)]
#[test]
fn an_update_that_leaves_the_version_says_so() {
    let b =
        Brew::new("echo 'Warning: Not upgrading codex, the latest version is already installed'\n");
    assert_eq!(
        b.update(&FakeProcesses::none(), Duration::from_secs(10)).failure,
        Some(Failure::Unchanged)
    );
}

#[cfg(unix)]
#[test]
fn an_update_past_its_time_limit_is_stopped() {
    let b = Brew::new("sleep 5\n");
    let last = b.update(&FakeProcesses::none(), Duration::from_millis(300));
    assert_eq!(last.failure, Some(Failure::Timeout));
    assert_eq!(std::fs::read_to_string(b.home.join("version")).unwrap(), "0.163.0");
}

#[cfg(unix)]
#[test]
fn claude_updates_while_its_sessions_run() {
    let home = temp_dir("cli-home");
    let versions = home.join(".local/share/claude/versions");
    std::fs::create_dir_all(&versions).unwrap();
    std::fs::write(home.join("version"), "2.1.311").unwrap();
    let v = home.join("version").display().to_string();
    common::script(
        &versions.join("2.1.311"),
        &format!("if [ \"$1\" = update ]; then printf 2.1.312 > '{v}'; echo updated; else printf '%s (Claude Code)\\n' \"$(cat '{v}')\"; fi\n"),
    );
    std::fs::create_dir_all(home.join(".local/bin")).unwrap();
    std::os::unix::fs::symlink(versions.join("2.1.311"), home.join(".local/bin/claude")).unwrap();
    let f = cli_tools::inspect(Tool::Claude, &home.join(".local/bin/claude"), &home);
    assert_eq!(f.layout.install, Some(Install::Native));
    let open = FakeProcesses {
        all: Some(vec![process("claude --resume"), process("claude")]),
        ..FakeProcesses::none()
    };
    let last = cli_tools::update(
        &f,
        &open,
        &home.join("claude.log"),
        &home,
        Duration::from_secs(10),
        7000,
    )
    .unwrap();
    assert_eq!(last, Last::done("2.1.311", "2.1.312", 7000));
}
