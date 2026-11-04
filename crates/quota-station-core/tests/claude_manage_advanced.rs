// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's side of the Claude accounts' management and of the
//! `quota-station claude` command line (the core's `claude_manage` tests
//! the app's own): an account held out of the rotation, the rotation's
//! commands and its LaunchAgent, against a fake Keychain and a fake
//! launchd in a temporary home. Every login is made up.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use quota_station_core::advanced::cli::CliHooks;
use quota_station_core::advanced::switch::SwitchHooks;
use quota_station_core::claude::cli::{self, Cli, Tools};
use quota_station_core::claude::manage::{self, ManageError};
use quota_station_core::claude::switch;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;
use quota_station_core::ext::Hooks;

/// Accounts 2 and 3, Claude Code on 2.
fn mac(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    mac.add(2, "lan@team.example", "org-team", &credential("acc-2", "ref-2", in_hours(5.0)));
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(
        &mac.store.credential(2).unwrap().unwrap(),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    mac
}

#[test]
fn an_account_gets_an_alias_and_leaves_the_rotation() {
    use quota_station_core::advanced::held;

    let mac = mac("manage-alias-rotation");
    manage::set_alias(&mac.store, 3, "  Team phụ ").unwrap();
    held::set_enabled(&mac.store, 3, false).unwrap();
    let three = mac.store.load().unwrap().get(3).cloned().unwrap();
    assert_eq!(three.alias.as_deref(), Some("Team phụ"));
    assert!(held::disabled(&three));
    manage::set_alias(&mac.store, 3, "").unwrap();
    held::set_enabled(&mac.store, 3, true).unwrap();
    let three = mac.store.load().unwrap().get(3).cloned().unwrap();
    assert_eq!((three.alias.clone(), held::disabled(&three)), (None, false));
    assert!(matches!(held::set_enabled(&mac.store, 9, true), Err(ManageError::UnknownSlot)));
}

// The command line.

fn cli(mac: &Mac, claude: Option<std::path::PathBuf>) -> Cli {
    use_scheduler(&mac.home, mac.scheduler.clone());
    Cli {
        store: mac.store.clone(),
        tools: Tools { claude, endpoints: FakeAnthropic::start().endpoints() },
    }
}

/// The fake launchd each test's Mac runs the advanced part's hooks with, by
/// its home.
static SCHEDULERS: Mutex<Vec<(PathBuf, Arc<common::advanced::FakeScheduler>)>> =
    Mutex::new(Vec::new());

/// The advanced part's hooks for the Mac at `home` run with `scheduler`.
fn use_scheduler(home: &Path, scheduler: Arc<common::advanced::FakeScheduler>) {
    let mut all = SCHEDULERS.lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|(h, _)| h != home);
    all.push((home.to_path_buf(), scheduler));
}

/// The hooks the command line runs with in this build: the advanced part's
/// commands, whose LaunchAgent runs `quota-station` in the test's folder of
/// fakes, with the Mac's fake launchd and a pause flag under its home.
fn hooks(cli: &Cli) -> CliHooks {
    let home = &cli.store.paths.home;
    let all = SCHEDULERS.lock().unwrap_or_else(|e| e.into_inner());
    let scheduler = all.iter().find(|(h, _)| h == home).expect("the Mac's launchd").1.clone();
    CliHooks {
        exe: home.join("fakes/bin/quota-station"),
        switch: SwitchHooks { scheduler, pause_file: Some(quota_station_core::pause::path(home)) },
    }
}

fn call(cli: &Cli, args: &[&str]) -> (i32, String, String) {
    call_with(cli, &hooks(cli), args)
}

/// `quota-station claude ARGS…` with `hooks`: the exit code, stdout and
/// stderr.
fn call_with(cli: &Cli, hooks: &dyn Hooks, args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(cli, hooks, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

#[test]
fn nothing_is_switched_or_turned_on_while_the_old_switcher_is_loaded() {
    let mac = mac("cli-legacy");
    mac.load_label("com.cswap.prefer-team");
    let cli = cli(&mac, None);
    let (code, _, err) = call(&cli, &["switch", "3"]);
    assert_eq!(code, 1);
    assert!(err.contains("com.cswap.prefer-team is loaded"), "{err}");
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert_eq!(call(&cli, &["rotation", "on"]).0, 1);
    assert!(!mac.loaded("com.shin315.quota-station.rotation"));
}

#[test]
fn nothing_is_switched_when_launchctl_cannot_tell() {
    let mac = mac("cli-launchctl-unknown");
    let cli = cli(&mac, None);
    use_scheduler(&mac.home, common::advanced::FakeScheduler::cannot_tell());
    assert_eq!(call(&cli, &["switch", "3"]).0, 1);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

#[test]
fn the_rotation_agent_is_loaded_and_unloaded() {
    let mac = mac("cli-agent");
    let cli = cli(&mac, None);
    assert_eq!(call(&cli, &["rotation", "on"]).0, 0);
    assert!(mac.loaded("com.shin315.quota-station.rotation"));
    assert_eq!(mac.scheduler.installed(), Some(mac.dir.join("bin/quota-station")));
    assert_eq!(call(&cli, &["rotation", "off"]).0, 0);
    assert!(!mac.loaded("com.shin315.quota-station.rotation"));
    assert_eq!(mac.scheduler.installed(), None);
}

#[test]
fn the_private_parts_usage_names_its_commands_for_help_and_for_a_wrong_word() {
    use quota_station_core::advanced::cli::USAGE;
    let mac = mac("cli-private-usage");
    let cli = cli(&mac, None);
    let usage = (2, String::new(), USAGE.to_string());
    assert_eq!(call(&cli, &["help"]), (0, USAGE.to_string(), String::new()));
    assert!(USAGE.contains("  rotate [--dry-run]    one run of the rotation (the LaunchAgent's)\n"));
    assert!(USAGE.ends_with("  remove N | alias N TEXT | enable N | disable N\n"));
    for args in [
        &["nonsense"][..],
        &["rotation"],
        &["rotation", "sideways"],
        &["enable"],
        &["disable", "0"],
        &["switch", "x"],
        &["remove"],
    ] {
        assert_eq!(call(&cli, args), usage, "{args:?}");
    }
    assert!(!mac.loaded("com.shin315.quota-station.rotation"));
}

#[test]
fn rotate_prints_its_decision_and_exits_0() {
    use quota_station_core::claude::rotate::NO_RULES;
    let mac = mac("cli-rotate");
    let cli = cli(&mac, None);
    let decided = (0, format!("{NO_RULES}\n"), String::new());
    assert_eq!(call(&cli, &["rotate", "--dry-run"]), decided);
    assert_eq!(call(&cli, &["rotate"]), decided);
    mac.load_label("com.cswap.prefer-team");
    let blocked = (0, "blocked (com.cswap.prefer-team is loaded)\n".to_string(), String::new());
    assert_eq!(call(&cli, &["rotate", "--dry-run"]), blocked);
    assert_eq!(call(&cli, &["rotate"]), blocked);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

#[test]
fn enable_and_disable_put_an_account_back_or_hold_it_out_and_exit_0() {
    let mac = mac("cli-enable");
    let cli = cli(&mac, None);
    let disabled = |slot| {
        quota_station_core::advanced::held::disabled(mac.store.load().unwrap().get(slot).unwrap())
    };
    assert_eq!(call(&cli, &["disable", "3"]), (0, String::new(), String::new()));
    assert!(disabled(3));
    assert_eq!(call(&cli, &["enable", "3"]), (0, String::new(), String::new()));
    assert!(!disabled(3));
    assert_eq!(
        call(&cli, &["enable", "9"]),
        (1, String::new(), "quota-station claude enable: no such account\n".to_string())
    );
    assert_eq!(
        call(&cli, &["disable", "9"]),
        (1, String::new(), "quota-station claude disable: no such account\n".to_string())
    );
}
