//! Switching Claude Code's login to another account the app keeps, against
//! a fake Keychain in a temporary home. Every login is made up.

mod common;

use std::process::Command;

use quota_station_core::claude::cc;
use quota_station_core::claude::switch::{self, SwitchError};
use serde_json::Value;

use common::accounts::{credential, in_hours, oauth, Mac};

/// Team accounts 2 and 3 in the app, Claude Code on 2.
fn two_accounts(tag: &str) -> (Mac, String, String) {
    let mac = Mac::new(tag);
    let (c2, c3) = (credential("acc-2", "ref-2", in_hours(5.0)), credential("acc-3", "ref-3", in_hours(5.0)));
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
    let (mut a, mut b) = (config(&mac).as_object().unwrap().clone(), before.as_object().unwrap().clone());
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
    let mut live: Value = serde_json::from_str(&credential("acc-2", "ref-2", in_hours(5.0))).unwrap();
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
    std::fs::write(mac.dir.join("watch"), mac.home.join(".claude.lock").display().to_string()).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    let watched = std::fs::read_to_string(mac.dir.join("watched")).unwrap();
    assert!(watched.lines().any(|l| l == "held Claude Code-credentials"), "{watched}");
    assert!(!watched.lines().any(|l| l == "free Claude Code-credentials"), "{watched}");
    assert!(!mac.home.join(".claude.lock").exists());
    assert!(!mac.home.join(".claude/.oauth_refresh.lock").exists());
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
    let old = Command::new("/usr/bin/touch").args(["-t", "202001010000"]).arg(&lock).status().unwrap();
    assert!(old.success());
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
    std::fs::write(mac.dir.join("drop-add"), "Claude Code-credentials").unwrap();
    let err = switch::switch(&mac.store, 3).unwrap_err();
    assert!(matches!(err, SwitchError::NotConfirmed { restored: true }), "{err:?}");
    assert_eq!(mac.live_credential(), Some(c2));
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
}

#[test]
fn a_keychain_that_refuses_the_write_leaves_claude_code_as_it_was() {
    let (mac, c2, _) = two_accounts("switch-refused");
    let before = std::fs::read_to_string(mac.paths.claude_json()).unwrap();
    std::fs::write(mac.dir.join("fail-add"), "").unwrap();
    assert!(switch::switch(&mac.store, 3).is_err());
    assert_eq!(mac.live_credential(), Some(c2));
    assert_eq!(std::fs::read_to_string(mac.paths.claude_json()).unwrap(), before);
}

#[test]
fn switching_to_the_account_in_use_writes_nothing() {
    let (mac, _, _) = two_accounts("switch-same");
    let done = switch::switch(&mac.store, 2).unwrap();
    assert!(done.already);
    assert!(!mac.security_calls().lines().any(|l| l.starts_with("add-generic-password") && l.ends_with("-s Claude Code-credentials")));
}
