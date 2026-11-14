// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A new sign-in of an account whose login died, as on two Macs where the
//! first one did not take: the account Claude Code is on (its own login must
//! get it), a switch away afterwards (the dead login must not replace it),
//! and a sign-in to another account than the one asked for. Against a fake
//! Keychain, a fake `claude auth login` and a fake Anthropic; every login is
//! made up. A shell script stands in for `claude`, so these run on Unix only.
#![cfg(unix)]

mod common;

use std::sync::atomic::AtomicBool;

use quota_station_core::claude::rotate::{self, Policy};
use quota_station_core::claude::{cc, manage, switch, usage};
use time::OffsetDateTime;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};

/// A fake `claude auth login` that signs `email` in with the refresh token
/// `ref-new`, as Claude Code does in its folder.
fn fake_login(mac: &Mac, email: &str) -> std::path::PathBuf {
    let dir = manage::login_folder(&mac.store);
    let service = cc::profile_service(&dir).unwrap();
    let login = credential("acc-new", "ref-new", in_hours(8.0));
    let config = serde_json::json!({ "oauthAccount": oauth(email, "uuid-new", "org-team") });
    let item = mac.dir.join("login-item");
    mac.keychain.written_by_file(&service, &mac.user(), &item);
    let body = format!(
        "[ \"$1 $2\" = 'auth login' ] || exit 9\n\
         printf '%s' '{login}' > '{item}'\nprintf '%s' '{config}' > \"$CLAUDE_CONFIG_DIR/.claude.json\"\n",
        item = item.display(),
    );
    common::script(&mac.dir.join("bin/claude"), &body)
}

/// Accounts 2 and 3, Claude Code on 2 with `live` (a login that died: it
/// expired 2 h ago and its refresh is refused).
fn dead_active(tag: &str, live: &str) -> (Mac, FakeAnthropic) {
    let mac = Mac::new(tag);
    let dead = credential("acc-dead", "ref-dead", in_hours(-2.0));
    mac.add(2, "lan@team.example", "org-team", &dead);
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(live, &oauth("lan@team.example", "uuid-2", "org-team"));
    let anthropic = FakeAnthropic::start();
    anthropic.usage("acc-new", 10.0, None, Some(5.0), None);
    anthropic.usage("acc-3", 20.0, None, Some(30.0), None);
    anthropic.refuses("ref-dead", "invalid_grant");
    (mac, anthropic)
}

fn row(mac: &Mac, slot: u32) -> usage::Row {
    usage::load(&mac.store).row(slot).cloned().unwrap()
}

fn refresh_token(credential: Option<String>) -> Option<String> {
    credential.as_deref().and_then(cc::refresh_token)
}

#[test]
fn a_relogin_of_the_active_account_reaches_claude_code() {
    let dead = credential("acc-dead", "ref-dead", in_hours(-2.0));
    let (mac, anthropic) = dead_active("relogin-active", &dead);
    let claude = fake_login(&mac, "lan@team.example");
    let added = manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    assert_eq!((added.slot, added.new, added.live), (2, false, Some(true)));
    assert_eq!(refresh_token(mac.store.credential(2).unwrap()).as_deref(), Some("ref-new"));
    // Claude Code's own login is the new one too, still on account 2.
    assert_eq!(refresh_token(mac.live_credential()).as_deref(), Some("ref-new"));
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.read.contains(&2) && pass.read.contains(&3), "{pass:?}");
    let r = row(&mac, 2);
    assert!(!r.relogin);
    assert_eq!(r.last_error, None);
    let auto =
        rotate::decide(&mac.store, &Policy::Auto { at: 90.0 }, OffsetDateTime::now_utc()).unwrap();
    assert_eq!(auto.decision, "stay");
}

#[test]
fn after_a_switch_away_the_same_relogin_reads_fresh() {
    let wiped = format!(
        r#"{{"claudeAiOauth":{{"accessToken":"","refreshToken":"","expiresAt":{}}}}}"#,
        in_hours(-2.0)
    );
    let (mac, anthropic) = dead_active("relogin-switch-wiped", &wiped);
    let claude = fake_login(&mac, "lan@team.example");
    manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    assert_eq!(refresh_token(mac.store.credential(2).unwrap()).as_deref(), Some("ref-new"));
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.read.contains(&2), "{pass:?}");
}

#[test]
fn a_switch_away_with_the_dead_tokens_still_live_keeps_the_relogin() {
    let dead = credential("acc-dead", "ref-dead", in_hours(-2.0));
    let (mac, anthropic) = dead_active("relogin-switch-dead", &dead);
    let claude = fake_login(&mac, "lan@team.example");
    manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    switch::switch(&mac.store, 3).unwrap();
    assert_eq!(refresh_token(mac.store.credential(2).unwrap()).as_deref(), Some("ref-new"));
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.read.contains(&2), "{pass:?}");
    assert!(!row(&mac, 2).relogin);
}

#[test]
fn a_relogin_as_another_account_renews_that_one_and_leaves_claude_code() {
    let dead = credential("acc-dead", "ref-dead", in_hours(-2.0));
    let (mac, _anthropic) = dead_active("relogin-wrong", &dead);
    let claude = fake_login(&mac, "lan.b@team.example");
    let added = manage::login(&mac.store, &claude, false, &AtomicBool::new(false)).unwrap();
    // Account 3 is not the one Claude Code is on: its login is not touched.
    assert_eq!((added.slot, added.new, added.live), (3, false, None));
    assert_eq!(refresh_token(mac.store.credential(2).unwrap()).as_deref(), Some("ref-dead"));
    assert_eq!(mac.live_credential(), Some(dead));
    assert_eq!(switch::active_slot(&mac.store), Some(2));
}
