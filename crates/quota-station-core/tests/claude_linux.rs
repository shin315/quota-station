// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts on Linux, with the secret store the command line
//! runs with there (`linux::Secrets`) in a temporary home: the accounts'
//! logins as files only their owner reads, Claude Code's login in
//! `~/.claude/.credentials.json` alone, one run of the rotation against
//! a fake Anthropic, and its `systemd --user` timer on a fake `systemctl`.
//! Every login is made up.

#![cfg(target_os = "linux")]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use quota_station_core::advanced::cli::CliHooks;
use quota_station_core::advanced::held::HeldHooks;
use quota_station_core::advanced::switch::SwitchHooks;
use quota_station_core::claude::cli::{self, Cli};
use quota_station_core::claude::keychain::user_account;
use quota_station_core::claude::rotate::{self, Policy, Tools};
use quota_station_core::claude::store::{self, Account, Kind, Store};
use quota_station_core::claude::systemd::{self, Systemd};
use quota_station_core::claude::{self, cc, session, switch, usage, Paths};
use quota_station_core::ext::Hooks;
use quota_station_core::linux::{self, NoItems, Plain, Secrets};
use quota_station_core::os::{Scheduler, SecretStore, Split};
use quota_station_core::sealed::SealedFiles;
use serde_json::{json, Value};

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::FakeScheduler;

const ITEM: &str = "Claude Code-credentials";

/// A Linux home in a temporary folder, its store on Linux's secret store.
struct Pc {
    home: PathBuf,
    paths: Paths,
    store: Store,
    secrets: PathBuf,
}

impl Pc {
    fn new(tag: &str) -> Pc {
        let home = common::temp_dir(tag);
        let secrets = linux::secrets_dir(&home);
        let mut paths = Paths::for_home(&home);
        paths.lock_wait = std::time::Duration::ZERO;
        let security: Secrets =
            Split { claude_code: NoItems, own: SealedFiles::new(&secrets, Plain) };
        let store = Store::new(paths.clone(), security).with_hooks(&HeldHooks);
        Pc { home, paths, store, secrets }
    }

    fn login_file(&self) -> PathBuf {
        self.paths.claude_home().join(".credentials.json")
    }

    fn add(&self, slot: u32, email: &str, org: &str, credential: &str) {
        let mut list = self.store.load().unwrap();
        list.put(Account {
            slot,
            alias: None,
            ext: Default::default(),
            kind: Kind::Login,
            account_uuid: format!("uuid-{slot}"),
            organization_uuid: org.into(),
            identity: store::identity(email, org),
            added: "2026-11-10T00:00:00Z".into(),
        });
        self.store.save(&list).unwrap();
        self.store.set_credential(slot, credential).unwrap();
        self.store.set_oauth_account(slot, &oauth(email, &format!("uuid-{slot}"), org)).unwrap();
    }

    /// Claude Code logged in as `credential`: its file and `oauthAccount`.
    fn log_in(&self, credential: &str, who: &Value) {
        std::fs::create_dir_all(self.paths.claude_home()).unwrap();
        std::fs::write(self.login_file(), credential).unwrap();
        std::fs::set_permissions(self.login_file(), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let config = json!({ "numStartups": 3, "oauthAccount": who });
        std::fs::write(self.paths.claude_json(), serde_json::to_string_pretty(&config).unwrap())
            .unwrap();
    }

    fn live_access(&self) -> String {
        access(&std::fs::read_to_string(self.login_file()).unwrap())
    }

    fn live_email(&self) -> Option<String> {
        let v: Value =
            serde_json::from_str(&std::fs::read_to_string(self.paths.claude_json()).ok()?).ok()?;
        v["oauthAccount"]["emailAddress"].as_str().map(String::from)
    }
}

fn access(credential: &str) -> String {
    cc::oauth_part(credential).unwrap()["accessToken"].as_str().unwrap().to_string()
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// Team accounts 2 and 3, Claude Code on 2.
fn two_accounts(tag: &str) -> (Pc, String) {
    let pc = Pc::new(tag);
    let (c2, c3) =
        (credential("acc-2", "ref-2", in_hours(5.0)), credential("acc-3", "ref-3", in_hours(5.0)));
    pc.add(2, "lan@team.example", "org-team", &c2);
    pc.add(3, "lan.b@team.example", "org-team", &c3);
    pc.log_in(&c2, &oauth("lan@team.example", "uuid-2", "org-team"));
    (pc, c2)
}

#[test]
fn the_app_keeps_its_files_under_dot_config_and_its_secrets_under_dot_local_share() {
    let home = Path::new("/home/lan");
    assert_eq!(claude::app_dir(home), home.join(".config/com.shin315.quota-station"));
    assert_eq!(Paths::for_home(home).dir, home.join(".config/com.shin315.quota-station/claude"));
    assert_eq!(
        linux::secrets_dir(home),
        home.join(".local/share/com.shin315.quota-station/secrets")
    );
}

#[test]
fn each_accounts_login_is_a_file_only_its_owner_reads() {
    let (pc, c2) = two_accounts("linux-slots");
    assert_eq!(pc.store.credential(2).unwrap(), Some(c2));
    let files: Vec<PathBuf> = walk(&pc.secrets).into_iter().filter(|p| p.is_file()).collect();
    assert!(!files.is_empty());
    for file in &files {
        assert_eq!(mode(file), 0o600, "{}", file.display());
        assert_eq!(mode(file.parent().unwrap()), 0o700, "{}", file.display());
    }
    // A new login replaces the old whole, through a file renamed over it:
    // nothing is left beside it.
    let c2b = credential("acc-2b", "ref-2b", in_hours(5.0));
    pc.store.set_credential(2, &c2b).unwrap();
    assert_eq!(pc.store.credential(2).unwrap(), Some(c2b));
    let after: Vec<PathBuf> = walk(&pc.secrets).into_iter().filter(|p| p.is_file()).collect();
    assert_eq!(after.len(), files.len(), "{after:?}");
    assert!(after.iter().all(|p| !p.to_string_lossy().ends_with(".tmp")), "{after:?}");
}

#[test]
fn claude_codes_items_never_exist_and_are_never_written() {
    let (pc, _) = two_accounts("linux-no-items");
    assert_eq!(pc.store.security.get(ITEM, &user_account()).unwrap(), None);
    assert!(!pc.store.security.exists(ITEM, &user_account()).unwrap());
    assert!(NoItems.set(ITEM, &user_account(), "x").is_err());
    assert!(!pc.secrets.join(ITEM).exists());
}

#[test]
fn the_live_login_is_read_from_its_file() {
    let (pc, c2) = two_accounts("linux-read");
    assert_eq!(cc::read_credential(&pc.paths, &*pc.store.security).unwrap(), Some(c2));
    assert_eq!(switch::active_slot(&pc.store), Some(2));
}

#[test]
fn a_switch_writes_the_file_claude_code_reads() {
    let (pc, _) = two_accounts("linux-switch");
    // The live login's MCP secrets belong to this computer and stay.
    let live = json!({
        "claudeAiOauth": cc::oauth_part(&credential("acc-2", "ref-2", in_hours(5.0))).unwrap(),
        "mcpOAuth": { "server": { "accessToken": "mcp-1" } },
    });
    std::fs::write(pc.login_file(), live.to_string()).unwrap();
    let done = switch::switch(&pc.store, 3).unwrap();
    assert!(!done.already);
    assert_eq!(pc.live_access(), "acc-3");
    assert_eq!(mode(&pc.login_file()), 0o600);
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(pc.login_file()).unwrap()).unwrap();
    assert_eq!(written["mcpOAuth"]["server"]["accessToken"], "mcp-1");
    assert_eq!(pc.live_email().as_deref(), Some("lan.b@team.example"));
    assert_eq!(switch::active_slot(&pc.store), Some(3));
    assert!(!pc.secrets.join(ITEM).exists());
    // Back again: account 2's login is the one the app kept.
    switch::switch(&pc.store, 2).unwrap();
    assert_eq!(pc.live_access(), "acc-2");
    assert_eq!(switch::active_slot(&pc.store), Some(2));
}

#[test]
fn a_switch_makes_the_file_when_claude_code_has_no_login_yet() {
    let (pc, _) = two_accounts("linux-none");
    std::fs::remove_file(pc.login_file()).unwrap();
    switch::switch(&pc.store, 3).unwrap();
    assert_eq!(pc.live_access(), "acc-3");
    assert_eq!(mode(&pc.login_file()), 0o600);
}

#[test]
fn an_accounts_own_folder_gets_its_login_in_its_file() {
    let (pc, _) = two_accounts("linux-session");
    let dir = session::prepare(&pc.store, 3).unwrap();
    assert_eq!(access(&std::fs::read_to_string(dir.join(".credentials.json")).unwrap()), "acc-3");
    assert_eq!(mode(&dir.join(".credentials.json")), 0o600);
    // Claude Code's own login is left alone.
    assert_eq!(pc.live_access(), "acc-2");
}

/// Personal 1, Team 2 and 3, Claude Code on 2, and no rotation file: the
/// threshold rule, as on a Mac.
fn rotation(tag: &str) -> (Pc, FakeAnthropic, Tools) {
    let (pc, _) = two_accounts(tag);
    pc.add(1, "lan@example.com", "org-personal", &credential("acc-1", "ref-1", in_hours(5.0)));
    let anthropic = FakeAnthropic::start();
    let scheduler: Arc<FakeScheduler> = FakeScheduler::new();
    let tools = Tools {
        scheduler,
        claude: None,
        pause_file: quota_station_core::pause::path(&pc.home),
        endpoints: anthropic.endpoints(),
    };
    (pc, anthropic, tools)
}

fn iso(hours: f64) -> String {
    let t = time::OffsetDateTime::now_utc() + time::Duration::seconds((hours * 3600.0) as i64);
    t.format(&time::format_description::well_known::Rfc3339).unwrap()
}

#[test]
fn the_rotation_switches_at_ninety_percent_as_on_a_mac() {
    let (pc, anthropic, tools) = rotation("linux-rotate");
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    assert_eq!(rotate::policy(&pc.store), Policy::Auto { at: 90.0 });
    // `status --refresh`'s reading, then `rotate --dry-run`'s decision.
    usage::read_due(&pc.store, &tools.endpoints, false, true);
    let now = time::OffsetDateTime::now_utc();
    let planned = rotate::dry_run(&pc.store, &tools, &rotate::policy(&pc.store), now);
    assert_eq!(planned.line, "switch 1 (full 5h: slot 2 at 96%, 0% used)");
    // A dry run changes nothing.
    assert_eq!(pc.live_access(), "acc-2");
    let decided = rotate::run(&pc.store, &tools);
    assert_eq!(decided.line, planned.line);
    assert_eq!(pc.live_access(), "acc-1");
    assert_eq!(pc.live_email().as_deref(), Some("lan@example.com"));
    assert_eq!(switch::active_slot(&pc.store), Some(1));
}

#[test]
fn below_ninety_percent_the_rotation_stays() {
    let (pc, anthropic, tools) = rotation("linux-stay");
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 89.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    assert_eq!(rotate::run(&pc.store, &tools).line, "stay");
    assert_eq!(pc.live_access(), "acc-2");
}

/// A fake `systemctl`: each call's words go to `calls`; a user manager
/// answers while `up` is there, and the timer is active while `active` is.
fn fake_systemctl(dir: &Path) -> Systemd {
    let body = r#"d=$(dirname "$0")
echo "$*" >> "$d/calls"
[ -e "$d/up" ] || { echo "Failed to connect to bus: No medium found" >&2; exit 1; }
case "$2" in
  is-system-running) echo running ;;
  is-active) if [ -e "$d/active" ]; then echo active; else echo inactive; exit 3; fi ;;
  restart) touch "$d/active" ;;
  stop) rm -f "$d/active" ;;
esac
"#;
    Systemd { systemctl: common::script(&dir.join("systemctl"), body) }
}

fn calls(dir: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(dir.join("calls")).unwrap_or_default();
    text.lines().filter(|l| !l.contains("is-system-running")).map(String::from).collect()
}

/// `rotation on|off` as the command line runs them, on `systemctl`.
fn rotation_command(pc: &Pc, systemd: Systemd, word: &str) -> (i32, String) {
    let hooks = CliHooks {
        exe: PathBuf::from("/home/lan/.local/bin/quota-station"),
        switch: SwitchHooks { scheduler: Arc::new(systemd), pause_file: None },
    };
    let anthropic = FakeAnthropic::start();
    let cli = Cli {
        store: Store::new(pc.paths.clone(), NoItems),
        tools: cli::Tools { claude: None, endpoints: anthropic.endpoints() },
    };
    let args: Vec<String> = ["rotation", word].iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = hooks.claude_command(&cli, &args, &mut out, &mut err).unwrap();
    (code, String::from_utf8(err).unwrap())
}

#[test]
fn rotation_on_starts_a_systemd_user_timer_and_off_removes_it() {
    let pc = Pc::new("linux-timer");
    let fakes = pc.home.join("fakes");
    let systemd = fake_systemctl(&fakes);
    std::fs::write(fakes.join("up"), "").unwrap();
    assert_eq!(systemd.loaded(), Some(false));
    assert_eq!(rotation_command(&pc, systemd.clone(), "on"), (0, String::new()));
    let service = std::fs::read_to_string(systemd::service_path(&pc.home)).unwrap();
    assert!(
        service.contains("ExecStart=\"/home/lan/.local/bin/quota-station\" claude rotate"),
        "{service}"
    );
    assert_eq!(std::fs::read_to_string(systemd::timer_path(&pc.home)).unwrap(), systemd::timer());
    let timer = "com.shin315.quota-station.rotation.timer";
    assert_eq!(
        calls(&fakes),
        [
            "--user is-active com.shin315.quota-station.rotation.timer".to_string(),
            "--user daemon-reload".into(),
            format!("--user enable {timer}"),
            format!("--user restart {timer}"),
        ]
    );
    assert_eq!(systemd.loaded(), Some(true));
    assert_eq!(systemd.legacy_loaded(), None);

    std::fs::remove_file(fakes.join("calls")).unwrap();
    assert_eq!(rotation_command(&pc, systemd.clone(), "off"), (0, String::new()));
    assert!(!systemd::service_path(&pc.home).exists());
    assert!(!systemd::timer_path(&pc.home).exists());
    let off = calls(&fakes);
    assert!(off.contains(&format!("--user stop {timer}")), "{off:?}");
    assert!(off.contains(&format!("--user disable {timer}")), "{off:?}");
    assert_eq!(systemd.loaded(), Some(false));
    // Off again: nothing there, nothing to do.
    std::fs::remove_file(fakes.join("calls")).unwrap();
    assert_eq!(rotation_command(&pc, systemd.clone(), "off"), (0, String::new()));
    let again = calls(&fakes);
    assert!(!again.iter().any(|c| c.contains(" stop ") || c.contains(" disable ")), "{again:?}");
}

#[test]
fn without_a_user_manager_rotation_on_gives_the_cron_line_and_writes_nothing() {
    let pc = Pc::new("linux-no-systemd");
    let fakes = pc.home.join("fakes");
    let systemd = fake_systemctl(&fakes);
    assert_eq!(systemd.loaded(), None, "no answer cannot be told");
    let (code, err) = rotation_command(&pc, systemd, "on");
    assert_eq!(code, 1);
    assert!(err.contains("crontab -e"), "{err}");
    assert!(
        err.trim_end().ends_with(
            "*/3 * * * * '/home/lan/.local/bin/quota-station' claude rotate >/dev/null 2>&1"
        ),
        "{err}"
    );
    assert!(!systemd::unit_dir(&pc.home).exists());
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        }
        out.push(path);
    }
    out
}
