// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The login Claude Code itself holds for the account in use, once it has
//! reached the end of its life: what Claude Code leaves when it clears a
//! login (`expiresAt` 0, with `loginEnded` when the server said the refresh
//! token expired), and a login that still has its refresh token but is past
//! the end date Claude Code keeps for it (`refreshTokenExpiresAt`) with its
//! access token expired for 15 minutes. The first is dead from the login
//! alone; the second is dead only with both, and every other case is waited
//! on as before. The app never sends a refresh token to find out. Against a
//! fake Keychain and a fake Anthropic; every login is made up.

mod common;

use quota_station_core::claude::rotate::{self, Policy};
use quota_station_core::claude::{state, switch, usage};
use serde_json::{json, Value};
use time::OffsetDateTime;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;

const DAY: f64 = 24.0;

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

/// A login of account 2 whose access token expires at `access_at` and whose
/// refresh token ends at `ends` (a JSON value, or none).
fn login(access_at: i64, ends: Option<Value>) -> String {
    let mut v: Value = serde_json::from_str(&credential("acc-2", "ref-2", access_at)).unwrap();
    if let Some(ends) = ends {
        v["claudeAiOauth"]["refreshTokenExpiresAt"] = ends;
    }
    v.to_string()
}

/// What Claude Code writes over a login whose refresh the server refused,
/// with `ended` as its `loginEnded` when the server said it expired.
fn cleared(ended: Option<&str>) -> String {
    let mut v = json!({"claudeAiOauth": {
        "accessToken": "", "refreshToken": "", "expiresAt": 0,
        "scopes": ["user:inference"], "subscriptionType": "team"
    }});
    if let Some(e) = ended {
        v["claudeAiOauth"]["loginEnded"] = json!(e);
    }
    v.to_string()
}

fn row(mac: &Mac, slot: u32) -> usage::Row {
    usage::load(&mac.store).row(slot).cloned().unwrap()
}

fn status(mac: &Mac, slot: u32) -> &'static str {
    let out = state::build(
        &mac.store.load().unwrap(),
        &usage::load(&mac.store),
        switch::active_slot(&mac.store),
        None,
        OffsetDateTime::now_utc(),
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

/// A run that leaves the login in use: the line it logs, and that the log
/// says why before it says it switched.
fn leaves(tag: &str, live: &str, reason: &str) {
    let (mac, anthropic) = mac_with_live(tag, live);
    let decided = rotate::run(&mac.store, &tools(&mac, &anthropic));
    assert_eq!(decided.line, "switch 3 (slot 2 login ended, 20% used)");
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    let text = log(&mac);
    let why =
        text.lines().position(|l| l.contains(&format!(" slot 2 login {reason}, left for slot 3")));
    let switched = text.lines().position(|l| l.contains(" switch to slot 3 (slot 2 login ended"));
    assert!(why.is_some() && switched.is_some() && why < switched, "{text}");
    assert!(anthropic.to("/v1/oauth/token").is_empty(), "nothing is refreshed");
}

#[test]
fn what_claude_code_leaves_after_a_refused_refresh_is_dead_and_says_why() {
    for (tag, ended) in [("ended-cleared-time-limit", Some("time_limit")), ("ended-cleared", None)]
    {
        let live = cleared(ended);
        let (mac, anthropic) = mac_with_live(tag, &live);
        let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
        assert!(pass.failed.contains(&(2, "no_refresh_token".into())), "{pass:?}");
        let r = row(&mac, 2);
        assert!(r.relogin && r.live_dead, "{r:?}");
        assert_eq!(status(&mac, 2), "reloginRequired");
        assert!(anthropic.to("/v1/oauth/token").is_empty());
        assert_eq!(mac.live_credential().as_deref(), Some(live.as_str()), "left as it was");
    }
    leaves(
        "ended-cleared-run-time-limit",
        &cleared(Some("time_limit")),
        "reached its time limit (Claude Code ended it, no refresh token left)",
    );
    leaves("ended-cleared-run", &cleared(None), "cannot be renewed (no refresh token left)");
}

#[test]
fn a_login_past_its_end_date_with_its_access_token_long_expired_is_dead() {
    let live = login(in_hours(-2.0), Some(json!(in_hours(-3.0 * DAY))));
    let (mac, anthropic) = mac_with_live("ended-date-read", &live);
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "login_ended".into())), "{pass:?}");
    assert!(pass.read.contains(&3));
    let r = row(&mac, 2);
    assert!(r.relogin && r.live_dead, "{r:?}");
    assert_eq!(r.last_error.as_deref(), Some("login_ended"));
    assert!(r.live_expired_at.is_some());
    // The row says sign in again, not "old numbers" and not Claude Code's wait.
    assert_eq!(status(&mac, 2), "reloginRequired");
    // Nothing went to Anthropic for it: no refresh, no usage call.
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(anthropic.to("/api/oauth/usage").len(), 1, "only account 3 is read");
    assert_eq!(mac.live_credential().as_deref(), Some(live.as_str()), "left as it was");
    // The rotation leaves it, as for any dead login.
    let plan = rotate::decide(&mac.store, &auto(), OffsetDateTime::now_utc()).unwrap();
    assert_eq!(plan.decision, "switch 3 (slot 2 login ended, 20% used)");
}

#[test]
fn a_run_leaves_a_login_past_its_end_date_and_logs_why() {
    let live = login(in_hours(-2.0), Some(json!(in_hours(-3.0 * DAY))));
    leaves("ended-date-run", &live, "reached its end date (the refresh token expired)");
}

#[test]
fn the_account_left_for_a_login_past_its_end_date_still_asks_to_sign_in_again() {
    let live = login(in_hours(-2.0), Some(json!(in_hours(-3.0 * DAY))));
    let (mac, anthropic) = mac_with_live("ended-date-left", &live);
    // The app's copy is older than the login Claude Code held, so the switch keeps that one.
    mac.store.set_credential(2, &credential("acc-2", "ref-2", in_hours(-30.0))).unwrap();
    rotate::run(&mac.store, &tools(&mac, &anthropic));
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    assert_eq!(mac.store.credential(2).unwrap().as_deref(), Some(live.as_str()));
    let r = row(&mac, 2);
    assert!(r.relogin, "{r:?}");
    assert_eq!(status(&mac, 2), "reloginRequired");
    // Not read again, so its refresh token is not sent either.
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(!pass.read.contains(&2) && !pass.failed.iter().any(|(s, _)| *s == 2), "{pass:?}");
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn an_expired_login_before_its_end_date_is_only_waited_on() {
    let live = login(in_hours(-2.0), Some(json!(in_hours(20.0 * DAY))));
    let (mac, anthropic) = mac_with_live("ended-date-ahead", &live);
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "token_expired".into())), "{pass:?}");
    let r = row(&mac, 2);
    assert!(!r.relogin && !r.live_dead, "{r:?}");
    assert_eq!(status(&mac, 2), "liveExpired");
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    let plan = rotate::decide(&mac.store, &auto(), OffsetDateTime::now_utc()).unwrap();
    assert_eq!(plan.decision, "stay (slot 2 usage unknown)");
    let decided = rotate::run(&mac.store, &tools(&mac, &anthropic));
    assert_eq!(decided.line, "stay (slot 2 usage unknown)");
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(!log(&mac).contains("login reached"), "{}", log(&mac));
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn a_working_access_token_is_not_dead_for_a_date_that_has_passed() {
    // The access token lasts 5 more hours: Claude Code runs on it as it is.
    let live = login(in_hours(5.0), Some(json!(in_hours(-3.0 * DAY))));
    let (mac, anthropic) = mac_with_live("ended-date-working", &live);
    anthropic.usage("acc-2", 10.0, None, Some(5.0), None);
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.read.contains(&2), "{pass:?}");
    let r = row(&mac, 2);
    assert!(!r.relogin && !r.live_dead, "{r:?}");
    assert_eq!(status(&mac, 2), "fresh");
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn the_fifteen_minute_rule_holds_before_the_date_counts() {
    let past = || Some(json!(in_hours(-3.0 * DAY)));
    // Expired 5 minutes ago: Claude Code may be about to refresh it.
    let (mac, anthropic) = mac_with_live("ended-date-young", &login(in_hours(-5.0 / 60.0), past()));
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "token_expired".into())), "{pass:?}");
    assert!(!row(&mac, 2).live_dead);
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    // Expired 20 minutes ago: it did not.
    let (mac, anthropic) = mac_with_live("ended-date-old", &login(in_hours(-20.0 / 60.0), past()));
    let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
    assert!(pass.failed.contains(&(2, "login_ended".into())), "{pass:?}");
    assert!(row(&mac, 2).live_dead);
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}

#[test]
fn an_end_date_that_is_missing_or_not_a_date_changes_nothing() {
    let cases: [(&str, Option<Value>); 7] = [
        ("missing", None),
        ("string", Some(json!(in_hours(-3.0 * DAY).to_string()))),
        ("negative", Some(json!(-5))),
        ("zero", Some(json!(0))),
        ("seconds", Some(json!(in_hours(-3.0 * DAY) / 1000))),
        ("null", Some(Value::Null)),
        ("object", Some(json!({"at": in_hours(-3.0 * DAY)}))),
    ];
    for (name, ends) in cases {
        let live = login(in_hours(-2.0), ends);
        let (mac, anthropic) = mac_with_live(&format!("ended-date-{name}"), &live);
        let pass = usage::read_due(&mac.store, &anthropic.endpoints(), true, true);
        assert!(pass.failed.contains(&(2, "token_expired".into())), "{name}: {pass:?}");
        let r = row(&mac, 2);
        assert!(!r.relogin && !r.live_dead, "{name}: {r:?}");
        let plan = rotate::decide(&mac.store, &auto(), OffsetDateTime::now_utc()).unwrap();
        assert_eq!(plan.decision, "stay (slot 2 usage unknown)", "{name}");
        assert!(anthropic.to("/v1/oauth/token").is_empty(), "{name}");
    }
}

#[test]
fn a_new_sign_in_of_the_account_in_use_clears_a_login_past_its_end_date() {
    let live = login(in_hours(-2.0), Some(json!(in_hours(-3.0 * DAY))));
    let (mac, anthropic) = mac_with_live("ended-date-signed-in", &live);
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
    assert!(anthropic.to("/v1/oauth/token").is_empty());
}
