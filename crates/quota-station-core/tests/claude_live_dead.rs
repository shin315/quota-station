// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The login Claude Code itself holds, for the account in use: dead for good
//! (nothing left to renew it with) against only expired (Claude Code has not
//! run since, and renews it when it does). The app never refreshes that
//! login to tell them apart, so a dead one is known from the login alone,
//! and the rotation leaves it; an expired one that still has its refresh
//! token is waited on as before. Against a fake Keychain and a fake
//! Anthropic; every login is made up.

mod common;

use quota_station_core::claude::rotate::{self, Policy};
use quota_station_core::claude::{state, switch, usage};
use time::OffsetDateTime;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;

/// Accounts 2 and 3, Claude Code on 2 with `live`; account 3 has room.
fn mac_with_live(tag: &str, live: &str) -> (Mac, FakeAnthropic) {
    let mac = Mac::new(tag);
    let anthropic = FakeAnthropic::start();
    mac.add(2, "lan@team.example", "org-team", &credential("acc-2", "ref-2", in_hours(-2.0)));
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(live, &oauth("lan@team.example", "uuid-2", "org-team"));
    anthropic.usage("acc-3", 20.0, None, Some(30.0), None);
    (mac, anthropic)
}

/// What Claude Code leaves of a login it can no longer use: the tokens gone,
/// the expiry kept.
fn wiped() -> String {
    format!(
        r#"{{"claudeAiOauth":{{"accessToken":"","refreshToken":"","expiresAt":{}}}}}"#,
        in_hours(-2.0)
    )
}

/// An expired access token with its refresh token still there.
fn only_expired() -> String {
    credential("acc-2", "ref-2", in_hours(-2.0))
}

fn row(mac: &Mac, slot: u32) -> usage::Row {
    usage::load(&mac.store).row(slot).cloned().unwrap()
}

fn status(mac: &Mac, slot: u32) -> &'static str {
    let now = OffsetDateTime::now_utc();
    let out = state::build(
        &mac.store.load().unwrap(),
        &usage::load(&mac.store),
        switch::active_slot(&mac.store),
        None,
        now,
    );
    out.accounts.iter().find(|a| a.slot == slot).unwrap().status
}

fn auto() -> Policy {
    Policy::Auto { at: 90.0 }
}

fn log(mac: &Mac) -> String {
    std::fs::read_to_string(mac.paths.log()).unwrap_or_default()
}

fn tools(mac: &Mac, anthropic: &FakeAnthropic) -> rotate::Tools {
    rotate::Tools {
        scheduler: mac.scheduler.clone(),
        claude: None,
        pause_file: quota_station_core::pause::path(&mac.home),
        endpoints: anthropic.endpoints(),
    }
}

#[test]
fn a_login_with_nothing_to_renew_it_with_is_dead_and_needs_a_new_sign_in() {
    let live = wiped();
    let (mac, anthropic) = mac_with_live("live-dead-read", &live);
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "no_refresh_token".into())), "{pass:?}");
    assert!(pass.read.contains(&3));
    let r = row(&mac, 2);
    assert!(r.relogin && r.live_dead, "{r:?}");
    assert_eq!(r.last_error.as_deref(), Some("no_refresh_token"));
    // The row says sign in again, not "old numbers" or a wait.
    assert_eq!(status(&mac, 2), "reloginRequired");
    // Nothing was sent to Anthropic for it: no refresh, no usage call.
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(anthropic.to("/api/oauth/usage").len(), 1, "only account 3 is read");
    // The login in Claude Code is as it was.
    assert_eq!(mac.live_credential().as_deref(), Some(live.as_str()));
}

#[test]
fn an_expired_login_that_still_has_its_refresh_token_is_only_waited_on() {
    let (mac, anthropic) = mac_with_live("live-expired-read", &only_expired());
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "token_expired".into())), "{pass:?}");
    let r = row(&mac, 2);
    assert!(!r.relogin && !r.live_dead, "{r:?}");
    assert_eq!(r.last_error.as_deref(), Some("token_expired"));
    assert!(r.live_expired_at.is_some());
    assert!(anthropic.to("/v1/oauth/token").is_empty(), "the live login is never refreshed");
    // The rotation holds, as before.
    let now = OffsetDateTime::now_utc();
    let plan = rotate::decide(&mac.store, &auto(), now).unwrap();
    assert_eq!(plan.decision, "stay (slot 2 usage unknown)");
}

#[test]
fn the_rotation_leaves_a_dead_login_in_use_and_no_longer_stays_on_unknown() {
    let (mac, anthropic) = mac_with_live("live-dead-decide", &wiped());
    usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    let now = OffsetDateTime::now_utc();
    let plan = rotate::decide(&mac.store, &auto(), now).unwrap();
    assert_eq!(plan.decision, "switch 3 (slot 2 login ended, 20% used)");
    assert_eq!(plan.target(), Some(3));
    // The window's preview says the same, with nothing read.
    let preview = rotate::dry_run(&mac.store, &tools(&mac, &anthropic), &auto(), now);
    assert_eq!(preview.line, plan.decision);
}

#[test]
fn a_run_switches_off_a_dead_login_and_logs_why() {
    let (mac, anthropic) = mac_with_live("live-dead-run", &wiped());
    let decided = rotate::run(&mac.store, &tools(&mac, &anthropic));
    assert_eq!(decided.line, "switch 3 (slot 2 login ended, 20% used)");
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    assert_eq!(mac.live_email().as_deref(), Some("lan.b@team.example"));
    let text = log(&mac);
    let reason = text.lines().position(|l| l.contains(" slot 2 login cannot be renewed"));
    let switched = text.lines().position(|l| l.contains(" switch to slot 3 (slot 2 login ended"));
    assert!(reason.is_some() && switched.is_some() && reason < switched, "{text}");
    // Account 2 stays in the list, asking to be signed in again.
    assert!(row(&mac, 2).relogin);
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn a_run_with_only_an_expired_login_in_use_stays_and_says_nothing_of_it() {
    let (mac, anthropic) = mac_with_live("live-expired-run", &only_expired());
    let decided = rotate::run(&mac.store, &tools(&mac, &anthropic));
    assert_eq!(decided.line, "stay (slot 2 usage unknown)");
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(!log(&mac).contains("switch to slot"), "{}", log(&mac));
    assert!(!log(&mac).contains("cannot be renewed"), "{}", log(&mac));
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn a_new_sign_in_of_the_dead_account_in_use_clears_it() {
    let (mac, anthropic) = mac_with_live("live-dead-signed-in", &wiped());
    usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(row(&mac, 2).live_dead);
    // Claude Code signed in again by itself (its own /login).
    mac.log_in(
        &credential("acc-new", "ref-new", in_hours(8.0)),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    anthropic.usage("acc-new", 10.0, None, Some(5.0), None);
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.read.contains(&2), "{pass:?}");
    let r = row(&mac, 2);
    assert!(!r.relogin && !r.live_dead, "{r:?}");
    assert_eq!(status(&mac, 2), "fresh");
    let plan = rotate::decide(&mac.store, &auto(), OffsetDateTime::now_utc()).unwrap();
    assert_eq!(plan.decision, "stay");
}

#[test]
fn a_stale_mark_on_an_account_not_in_use_never_makes_the_rotation_leave_the_one_in_use() {
    // Slot 3 is marked dead from an old read; Claude Code is on 2, healthy.
    let (mac, anthropic) =
        mac_with_live("live-dead-stale", &credential("acc-2", "ref-2", in_hours(5.0)));
    anthropic.usage("acc-2", 10.0, None, Some(5.0), None);
    usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    let mut cache = usage::load(&mac.store);
    cache.accounts.get_mut("3").unwrap().live_dead = true;
    usage::save(&mac.store, &cache).unwrap();
    let plan = rotate::decide(&mac.store, &auto(), OffsetDateTime::now_utc()).unwrap();
    assert_eq!(plan.decision, "stay");
}
