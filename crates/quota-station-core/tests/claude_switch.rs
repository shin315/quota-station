// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Switching Claude Code's login to another account the app keeps, against
//! a fake Keychain in a temporary home. Every login is made up.

mod common;

use std::path::Path;

use quota_station_core::claude::switch::{self, SwitchError};
use quota_station_core::claude::{cc, usage};
use serde_json::Value;

use common::accounts::{credential, in_hours, oauth, Mac};

/// Team accounts 2 and 3 in the app, Claude Code on 2.
fn two_accounts(tag: &str) -> (Mac, String, String) {
    let mac = Mac::new(tag);
    let (c2, c3) =
        (credential("acc-2", "ref-2", in_hours(5.0)), credential("acc-3", "ref-3", in_hours(5.0)));
    mac.add(2, "lan@team.example", "org-team", &c2);
    mac.add(3, "lan.b@team.example", "org-team", &c3);
    mac.log_in(&c2, &oauth("lan@team.example", "uuid-2", "org-team"));
    (mac, c2, c3)
}

fn access(credential: &str) -> String {
    cc::oauth_part(credential).unwrap()["accessToken"].as_str().unwrap().to_string()
}

fn config(mac: &Mac) -> Value {
    serde_json::from_str(&std::fs::read_to_string(mac.paths.claude_json()).unwrap()).unwrap()
}

#[test]
fn a_switch_puts_the_account_in_claude_codes_login() {
    let (mac, _, _) = two_accounts("switch");
    let before = config(&mac);
    let done = switch::switch(&mac.store, 3).unwrap();
    assert_eq!((done.from, done.to, done.already), (Some(2), 3, false));
    assert_eq!(access(&mac.live_credential().unwrap()), "acc-3");
    assert_eq!(mac.live_email().as_deref(), Some("lan.b@team.example"));
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    // Only `oauthAccount` changed in `~/.claude.json`.
    let (mut a, mut b) =
        (config(&mac).as_object().unwrap().clone(), before.as_object().unwrap().clone());
    a.remove("oauthAccount");
    b.remove("oauthAccount");
    assert_eq!(a, b);
}

#[test]
fn the_outgoing_login_is_kept_as_claude_code_last_refreshed_it() {
    let (mac, _, _) = two_accounts("switch-keep");
    let refreshed = credential("acc-2b", "ref-2b", in_hours(8.0));
    mac.put_item("Claude Code-credentials", &mac.user(), &refreshed);
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(mac.store.credential(2).unwrap(), Some(refreshed));
}

#[test]
fn claude_codes_own_secrets_stay_with_the_live_login() {
    let (mac, _, _) = two_accounts("switch-shared");
    let mut live: Value =
        serde_json::from_str(&credential("acc-2", "ref-2", in_hours(5.0))).unwrap();
    live["mcpOAuth"] = serde_json::json!({ "linear": { "accessToken": "mcp-made-up" } });
    mac.put_item("Claude Code-credentials", &mac.user(), &live.to_string());
    switch::switch(&mac.store, 3).unwrap();
    let now: Value = serde_json::from_str(&mac.live_credential().unwrap()).unwrap();
    assert_eq!(now["claudeAiOauth"]["accessToken"], "acc-3");
    assert_eq!(now["mcpOAuth"], live["mcpOAuth"]);
}

#[test]
fn the_login_is_written_while_claude_codes_locks_are_held() {
    let (mac, _, _) = two_accounts("switch-locks");
    mac.keychain.watch(&mac.home.join(".claude.lock"));
    switch::switch(&mac.store, 3).unwrap();
    let watched = mac.keychain.watched();
    assert!(watched.iter().any(|l| l == "held Claude Code-credentials"), "{watched:?}");
    assert!(!watched.iter().any(|l| l == "free Claude Code-credentials"), "{watched:?}");
    assert!(!mac.home.join(".claude.lock").exists());
    assert!(!mac.home.join(".claude/.oauth_refresh.lock").exists());
    assert!(!mac.home.join(".claude/.storage-write.lock").exists());
}

#[test]
fn claude_codes_write_lock_held_stops_the_switch() {
    let (mac, c2, _) = two_accounts("switch-storage-write");
    let lock = mac.home.join(".claude/.storage-write.lock");
    std::fs::create_dir_all(&lock).unwrap();
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert_eq!(err.kind(), "busy");
    assert_eq!(mac.live_credential(), Some(c2));
    assert!(lock.is_dir());
    // Held past its 15 seconds, it was left behind.
    filetime::set_file_mtime(&lock, filetime::FileTime::from_unix_time(1_577_836_800, 0)).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(switch::active_slot(&mac.store), Some(3));
}

#[test]
fn a_lock_claude_code_holds_stops_the_switch() {
    let (mac, c2, _) = two_accounts("switch-busy");
    std::fs::create_dir(mac.home.join(".claude.lock")).unwrap();
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert_eq!(err.kind(), "busy");
    assert_eq!(mac.live_credential(), Some(c2));
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
    // Its lock is Claude Code's: the app leaves it.
    assert!(mac.home.join(".claude.lock").is_dir());
}

#[test]
fn a_lock_left_behind_is_taken_over() {
    let (mac, _, _) = two_accounts("switch-stale");
    let lock = mac.home.join(".claude.lock");
    std::fs::create_dir(&lock).unwrap();
    filetime::set_file_mtime(&lock, filetime::FileTime::from_unix_time(1_577_836_800, 0)).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(switch::active_slot(&mac.store), Some(3));
}

#[test]
fn a_live_login_the_app_does_not_keep_is_never_overwritten() {
    let (mac, _, _) = two_accounts("switch-unmanaged");
    let theirs = credential("acc-x", "ref-x", in_hours(5.0));
    mac.log_in(&theirs, &oauth("someone@else.example", "uuid-x", "org-x"));
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert!(matches!(err, SwitchError::Unmanaged));
    assert_eq!(mac.live_credential(), Some(theirs));
    assert_eq!(mac.live_email().as_deref(), Some("someone@else.example"));
}

#[test]
fn a_login_that_does_not_read_back_is_put_back() {
    let (mac, c2, _) = two_accounts("switch-unconfirmed");
    mac.keychain.drop_add("Claude Code-credentials");
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert!(matches!(err, SwitchError::NotConfirmed { restored: true, .. }), "{err:?}");
    assert_eq!(mac.live_credential(), Some(c2));
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
}

#[test]
fn a_keychain_that_refuses_the_write_leaves_claude_code_as_it_was() {
    let (mac, c2, _) = two_accounts("switch-refused");
    let before = std::fs::read_to_string(mac.paths.claude_json()).unwrap();
    mac.keychain.fail_add(true);
    assert!(switch::switch(&mac.store, 3).is_err());
    assert_eq!(mac.live_credential(), Some(c2));
    assert_eq!(std::fs::read_to_string(mac.paths.claude_json()).unwrap(), before);
}

#[test]
fn switching_to_the_account_in_use_writes_nothing() {
    let (mac, _, _) = two_accounts("switch-same");
    let done = switch::switch(&mac.store, 2).unwrap();
    assert!(done.already);
    assert!(!mac.security_calls().lines().any(
        |l| l.starts_with("add-generic-password") && l.ends_with("-s Claude Code-credentials")
    ));
}

#[test]
fn a_login_file_that_cannot_be_written_puts_the_keychain_back() {
    let (mac, c2, _) = two_accounts("switch-file-refused");
    let file = mac.home.join(".claude/.credentials.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, &c2).unwrap();
    locked(&file, true);
    let result = switch::switch(&mac.store, 3);
    locked(&file, false);
    assert!(matches!(result, Err(SwitchError::NotConfirmed { restored: true, .. })), "{result:?}");
    assert_eq!(mac.live_credential(), Some(c2.clone()));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), c2);
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

/// Makes `file` one nothing can replace: immutable on macOS, read-only on
/// Windows.
fn locked(file: &Path, on: bool) {
    #[cfg(unix)]
    {
        let flag = if on { "uchg" } else { "nouchg" };
        let status = std::process::Command::new("/usr/bin/chflags").arg(flag).arg(file).status();
        assert!(status.unwrap().success());
    }
    #[cfg(windows)]
    {
        let mut permissions = std::fs::metadata(file).unwrap().permissions();
        permissions.set_readonly(on);
        std::fs::set_permissions(file, permissions).unwrap();
    }
}

/// `~/.claude.json` without `oauthAccount`, as after a reset.
fn forget_who(mac: &Mac) {
    std::fs::write(mac.paths.claude_json(), "{\n  \"numStartups\": 3\n}\n").unwrap();
}

#[test]
fn a_live_login_without_its_account_is_told_by_its_refresh_token() {
    let (mac, _, _) = two_accounts("switch-no-who");
    forget_who(&mac);
    let refreshed = credential("acc-2b", "ref-2", in_hours(8.0));
    mac.put_item("Claude Code-credentials", &mac.user(), &refreshed);
    let done = switch::switch(&mac.store, 3).unwrap();
    assert_eq!(done.from, Some(2));
    assert_eq!(mac.store.credential(2).unwrap(), Some(refreshed));
    assert_eq!(access(&mac.live_credential().unwrap()), "acc-3");
}

#[test]
fn a_live_login_without_its_account_the_app_does_not_keep_is_never_overwritten() {
    let (mac, _, _) = two_accounts("switch-no-who-unknown");
    forget_who(&mac);
    let theirs = credential("acc-x", "ref-x", in_hours(5.0));
    mac.put_item("Claude Code-credentials", &mac.user(), &theirs);
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert!(matches!(err, SwitchError::Unmanaged), "{err:?}");
    assert_eq!(mac.live_credential(), Some(theirs));
}

#[test]
fn the_outgoing_login_kept_by_a_switch_is_read_again() {
    let (mac, _, _) = two_accounts("switch-relogin");
    let identity = mac.store.load().unwrap().get(2).unwrap().identity.clone();
    let mut cache = usage::load(&mac.store);
    cache.accounts.insert(
        "2".into(),
        usage::Row {
            identity,
            relogin: true,
            failures: 3,
            backoff_until: Some(4_000_000_000.0),
            ..usage::Row::default()
        },
    );
    usage::save(&mac.store, &cache).unwrap();
    mac.put_item(
        "Claude Code-credentials",
        &mac.user(),
        &credential("acc-2b", "ref-2b", in_hours(8.0)),
    );
    switch::switch(&mac.store, 3).unwrap();
    let row = usage::load(&mac.store).row(2).cloned().unwrap();
    assert_eq!((row.relogin, row.failures, row.backoff_until), (false, 0, None));
}

#[test]
fn an_account_left_without_its_email_keeps_its_own_account_details() {
    let (mac, _, _) = two_accounts("switch-no-email");
    std::fs::write(
        mac.paths.claude_json(),
        "{\n  \"oauthAccount\": {\n    \"accountUuid\": \"uuid-2\"\n  }\n}\n",
    )
    .unwrap();
    assert_eq!(switch::switch(&mac.store, 3).unwrap().from, Some(2));
    let kept = mac.store.oauth_account(2).unwrap().unwrap();
    assert_eq!(kept["emailAddress"], "lan@team.example");
    assert!(!switch::switch(&mac.store, 2).unwrap().already);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}

#[test]
fn a_switch_to_the_login_in_use_puts_its_account_back_in_the_config() {
    let (mac, c2, _) = two_accounts("switch-same-no-who");
    forget_who(&mac);
    let done = switch::switch(&mac.store, 2).unwrap();
    assert_eq!((done.from, done.already), (Some(2), false));
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert_eq!(mac.live_credential(), Some(c2));
}
