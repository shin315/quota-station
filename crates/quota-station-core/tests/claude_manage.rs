// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Adding, removing and changing the app's Claude accounts, and the
//! `quota-station claude` command line, against a fake Keychain and
//! `claude` in a temporary home. Every login is made up.
// Off Unix the tests that run a fake `claude` script are left out, and
// with them the uses of some imports.
#![cfg_attr(not(unix), allow(unused_imports))]

mod common;

use std::sync::atomic::AtomicBool;

use quota_station_core::claude::cli::{self, Cli, Tools};
use quota_station_core::claude::manage::{self, ManageError};
use quota_station_core::claude::store::Kind;
use quota_station_core::claude::{cc, session, switch, usage};
use serde_json::Value;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};
use common::ext::{TestHooks, TEST_USAGE};
use quota_station_core::ext::{Hooks, NoHooks};

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

/// A fake `claude auth login` that logs `email` in where `CLAUDE_CONFIG_DIR`
/// points, as Claude Code does: its folder's Keychain item and
/// `.claude.json`. With `fail`, it says why on stderr and exits 1. A shell
/// script, so its tests run on Unix only.
#[cfg(unix)]
fn fake_login(mac: &Mac, email: &str, fail: bool) -> std::path::PathBuf {
    let body = if fail {
        "echo 'OAuth error: the browser closed' >&2\nexit 1\n".to_string()
    } else {
        let dir = manage::login_folder(&mac.store);
        let service = cc::profile_service(&dir).unwrap();
        let login = credential("acc-new", "ref-new", in_hours(8.0));
        let config = serde_json::json!({ "oauthAccount": oauth(email, "uuid-new", "org-team") });
        let item = mac.dir.join("login-item");
        mac.keychain.written_by_file(&service, &mac.user(), &item);
        format!(
            "[ \"$1 $2\" = 'auth login' ] || exit 9\n[ \"$CLAUDE_CONFIG_DIR\" = '{dir}' ] || exit 8\n\
             printf '%s' '{login}' > '{item}'\nprintf '%s' '{config}' > \"$CLAUDE_CONFIG_DIR/.claude.json\"\n",
            dir = dir.display(),
            item = item.display(),
        )
    };
    common::script(&mac.dir.join("bin/claude"), &body)
}

#[cfg(unix)]
#[test]
fn a_new_login_comes_in_under_the_next_slot_without_touching_claude_code() {
    let mac = mac("manage-login");
    let live = mac.live_credential();
    let claude = fake_login(&mac, "lan.c@team.example", false);
    let added = manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    assert_eq!((added.slot, added.new), (4, true));
    assert_eq!(mac.store.oauth_account(4).unwrap().unwrap()["emailAddress"], "lan.c@team.example");
    assert_eq!(
        cc::refresh_token(&mac.store.credential(4).unwrap().unwrap()).as_deref(),
        Some("ref-new")
    );
    assert_eq!(mac.live_credential(), live);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    // The login folder and its Keychain item are gone.
    let dir = manage::login_folder(&mac.store);
    assert!(!dir.exists());
    assert_eq!(mac.item(&cc::profile_service(&dir).unwrap(), &mac.user()), None);
}

#[cfg(unix)]
#[test]
fn logging_in_an_account_the_app_has_renews_its_login() {
    let mac = mac("manage-relogin");
    let claude = fake_login(&mac, "lan.b@team.example", false);
    let added = manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    assert_eq!((added.slot, added.new), (3, false));
    assert_eq!(
        cc::refresh_token(&mac.store.credential(3).unwrap().unwrap()).as_deref(),
        Some("ref-new")
    );
    assert_eq!(mac.store.load().unwrap().accounts.len(), 2);
}

#[cfg(unix)]
#[test]
fn a_login_that_fails_adds_nothing() {
    let mac = mac("manage-fail");
    let claude = fake_login(&mac, "", true);
    let err = manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap_err();
    assert!(
        matches!(&err, ManageError::Login(why) if why == "OAuth error: the browser closed"),
        "{err:?}"
    );
    assert_eq!(mac.store.load().unwrap().accounts.len(), 2);
    assert!(!manage::login_folder(&mac.store).exists());
}

#[cfg(unix)]
#[test]
fn a_cancelled_login_adds_nothing() {
    let mac = mac("manage-cancel");
    let claude = common::script(&mac.dir.join("bin/claude"), "exec sleep 600\n");
    let err = manage::login(&mac.store, &claude, false, &AtomicBool::new(true)).unwrap_err();
    assert!(matches!(err, ManageError::Cancelled));
    assert_eq!(mac.store.load().unwrap().accounts.len(), 2);
}

#[test]
fn the_login_claude_code_is_on_can_be_added_and_then_switched_away_from() {
    let mac = mac("manage-live");
    mac.log_in(
        &credential("acc-x", "ref-x", in_hours(5.0)),
        &oauth("lan.d@team.example", "uuid-x", "org-team"),
    );
    assert!(switch::switch(&mac.store, 3).is_err(), "an unknown login stops a switch");
    let added = manage::adopt_live(&mac.store).unwrap();
    assert_eq!((added.slot, added.new), (4, true));
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(switch::active_slot(&mac.store), Some(3));
}

#[test]
fn a_setup_token_is_kept_as_a_login_without_refresh() {
    let mac = mac("manage-token");
    let added = manage::add_token(&mac.store, "  sk-ant-oat01-made-up\n", None).unwrap();
    assert_eq!(added.slot, 4);
    let list = mac.store.load().unwrap();
    assert_eq!(list.get(4).unwrap().kind, Kind::SetupToken);
    let stored: Value = serde_json::from_str(&mac.store.credential(4).unwrap().unwrap()).unwrap();
    assert_eq!(stored["claudeAiOauth"]["accessToken"], "sk-ant-oat01-made-up");
    assert_eq!(
        mac.store.oauth_account(4).unwrap().unwrap()["emailAddress"],
        "setup-token-4@token.local"
    );
    for bad in ["", "two words", "line\nbreak"] {
        assert!(
            matches!(manage::add_token(&mac.store, bad, None), Err(ManageError::BadToken)),
            "{bad:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn removing_an_account_takes_its_secrets_folder_and_reading() {
    let mac = mac("manage-remove");
    let claude = common::script(&mac.dir.join("bin/claude"), "exit 0\n");
    session::run(&mac.store, &claude, 3, true, &[]).unwrap();
    let folder = session::folder(&mac.store, 3);
    let profile = cc::profile_service(&folder).unwrap();
    assert!(mac.item(&profile, &mac.user()).is_some());
    let anthropic = FakeAnthropic::start();
    anthropic.usage("acc-3", 5.0, None, None, None);
    usage::read_due(&mac.store, &anthropic.endpoints(), false, true);

    manage::remove(&mac.store, 3).unwrap();
    assert!(mac.store.load().unwrap().get(3).is_none());
    assert_eq!(mac.store.credential(3).unwrap(), None);
    assert_eq!(mac.store.oauth_account(3).unwrap(), None);
    assert_eq!(mac.item(&profile, &mac.user()), None);
    assert!(!folder.exists());
    assert!(usage::load(&mac.store).row(3).is_none());
}

#[test]
fn the_account_in_use_cannot_be_removed() {
    let mac = mac("manage-remove-active");
    assert!(matches!(manage::remove(&mac.store, 2), Err(ManageError::Active)));
    assert!(mac.store.load().unwrap().get(2).is_some());
}

#[test]
fn an_account_gets_an_alias() {
    let mac = mac("manage-alias");
    manage::set_alias(&mac.store, 3, "  Team phụ ").unwrap();
    let three = mac.store.load().unwrap().get(3).cloned().unwrap();
    assert_eq!(three.alias.as_deref(), Some("Team phụ"));
    manage::set_alias(&mac.store, 3, "").unwrap();
    let three = mac.store.load().unwrap().get(3).cloned().unwrap();
    assert_eq!(three.alias, None);
    assert!(matches!(manage::set_alias(&mac.store, 9, "x"), Err(ManageError::UnknownSlot)));
}

// The command line.

fn cli(mac: &Mac, claude: Option<std::path::PathBuf>) -> Cli {
    Cli {
        store: mac.store.clone(),
        tools: Tools { claude, endpoints: FakeAnthropic::start().endpoints() },
    }
}

/// The hooks the command line runs with in this build: none.
fn hooks(_: &Cli) -> quota_station_core::ext::NoHooks {
    quota_station_core::ext::NoHooks
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
fn an_extension_adds_commands_and_its_usage_and_the_apps_own_stay_the_apps() {
    let mac = mac("cli-ext");
    let cli = cli(&mac, None);
    let hooks = TestHooks;
    let usage = || (2, String::new(), TEST_USAGE.to_string());
    assert_eq!(
        call_with(&cli, &hooks, &["test-word", "a", "b"]),
        (3, "test a b\n".to_string(), "noted\n".to_string())
    );
    // The app's `status` is the app's: the extension is never asked.
    let (code, text, _) = call_with(&cli, &hooks, &["status", "--json"]);
    assert_eq!(code, 0);
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["activeSlot"], 2);
    // The usage is the extension's, for help and for what nobody knows.
    assert_eq!(call_with(&cli, &hooks, &["help"]), (0, TEST_USAGE.to_string(), String::new()));
    assert_eq!(call_with(&cli, &hooks, &["--help"]).1, TEST_USAGE);
    assert_eq!(call_with(&cli, &hooks, &[]).1, TEST_USAGE);
    assert_eq!(call_with(&cli, &hooks, &["nonsense"]), usage());
    assert_eq!(call_with(&cli, &hooks, &["switch", "x"]), usage());
    assert_eq!(call_with(&cli, &hooks, &["remove"]), usage());
}

#[test]
fn without_an_extension_a_command_the_app_has_not_prints_the_apps_usage_and_exits_2() {
    let mac = mac("cli-no-ext");
    let cli = cli(&mac, None);
    let usage = (2, String::new(), cli::USAGE.to_string());
    for args in [&["test-word", "a"][..], &["nonsense"], &["switch", "x"], &["alias"]] {
        assert_eq!(call_with(&cli, &NoHooks, args), usage, "{args:?}");
    }
    assert_eq!(call_with(&cli, &NoHooks, &["help"]), (0, cli::USAGE.to_string(), String::new()));
    assert_eq!(call_with(&cli, &NoHooks, &["-h"]).1, cli::USAGE);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

/// The app without an extension refuses the rotation's commands as it
/// refuses any command it does not have: the app's usage on stderr, exit 2,
/// nothing done.
#[test]
fn without_an_extension_the_rotations_commands_are_refused_with_the_apps_usage() {
    let mac = mac("cli-no-extension");
    let cli = cli(&mac, None);
    let before = mac.store.load().unwrap();
    let usage = (2, String::new(), cli::USAGE.to_string());
    for args in [
        &["rotate"][..],
        &["rotate", "--dry-run"],
        &["rotation", "on"],
        &["rotation", "off"],
        &["enable", "3"],
        &["disable", "3"],
    ] {
        assert_eq!(call_with(&cli, &NoHooks, args), usage, "{args:?}");
    }
    assert_eq!(mac.store.load().unwrap(), before);
    assert!(!cli::USAGE.contains("rotate"));
}

#[test]
fn status_names_no_email_unless_asked() {
    let mac = mac("cli-status");
    let cli = cli(&mac, None);
    let (code, text, _) = call(&cli, &["status", "--json"]);
    assert_eq!(code, 0);
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["activeSlot"], 2);
    assert_eq!(data["accounts"].as_array().unwrap().len(), 2);
    assert!(!text.contains("@team.example"));
    let (_, text, _) = call(&cli, &["status", "--json", "--emails"]);
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["accounts"][1]["email"], "lan.b@team.example");
}

#[cfg(unix)]
#[test]
fn run_gives_back_claudes_exit_code() {
    let mac = mac("cli-run");
    let claude = common::script(&mac.dir.join("bin/claude"), "[ \"$1\" = -p ] && exit 5\nexit 0\n");
    let cli = cli(&mac, Some(claude));
    assert_eq!(call(&cli, &["run", "3", "--", "-p", "ok"]).0, 5);
    assert_eq!(call(&cli, &["run", "2", "--require-session", "--", "-p", "ok"]).0, 1);
    assert_eq!(call(&cli, &["run", "x"]).0, 2);
}
