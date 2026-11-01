// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's hooks for an extension (`quota_station_core::ext`).

mod common;

use std::path::{Path, PathBuf};

use common::accounts::{credential, in_hours, oauth, Mac};
use common::ext::{pool_reads, TestHooks, TestPolicy};
use quota_station_core::alerts::{self, Facts, Kind, Memory, Switches};
use quota_station_core::ext::{self, Hooks, NoHooks};
use quota_station_core::quota::claude::own_now;
use quota_station_core::quota::Response;
use quota_station_core::settings::{PoolAlert, Settings};
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};

/// What every caller holds: hooks behind a shared reference, from any
/// thread.
fn shared(hooks: &dyn Hooks) -> &dyn Hooks {
    hooks
}

#[test]
fn hooks_pass_as_a_shared_reference_between_threads() {
    fn send_sync<T: Send + Sync + ?Sized>(_: &T) {}
    send_sync(shared(&NoHooks));
    send_sync(shared(&TestHooks));
    send_sync(ext::none());
}

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qs-ext-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn text(file: &Path) -> String {
    std::fs::read_to_string(file).unwrap()
}

#[test]
fn a_new_install_settles_the_extensions_settings_with_the_apps() {
    let dir = fresh_dir("new");
    let file = dir.join("settings.json");
    let s = Settings::open(&file, &TestHooks);
    assert!(!s.setup_seen);
    assert_eq!(s.ext.get("testSettled"), Some(&json!(false)));
    assert_eq!(s.ext.get("testFound"), Some(&json!(false)), "there was no file to read");
    let written = text(&file);
    assert!(written.contains(r#""testSettled": false"#), "{written}");
    assert!(written.contains(r#""setupSeen": false"#), "{written}");
    // What the app writes next never makes it look like an older install.
    std::fs::create_dir_all(dir.join("alerts")).unwrap();
    std::fs::write(dir.join("alerts/alerts.json"), "{}").unwrap();
    let again = Settings::open(&file, &TestHooks);
    assert_eq!(again.ext.get("testSettled"), Some(&json!(false)));
    assert_eq!(again.ext.get("testFound"), Some(&json!(true)));
    assert!(!again.setup_seen);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_extensions_key_settles_a_file_the_app_had_settled_before_it() {
    let dir = fresh_dir("had");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    // Settled for the app, not yet for the extension: a Mac that had the app.
    std::fs::write(&file, r#"{ "machineName": "Mac A", "setupSeen": false }"#).unwrap();
    let s = Settings::open(&file, &TestHooks);
    assert_eq!(s.ext.get("testSettled"), Some(&json!(true)));
    assert!(s.setup_seen, "settled again with the extension's key");
    assert_eq!(s.machine_name.as_deref(), Some("Mac A"));
    assert!(text(&file).contains(r#""testSettled": true"#));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_extension_reads_its_settings_on_every_open_and_they_are_written_back() {
    let dir = fresh_dir("read");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    let before =
        r#"{ "setupSeen": true, "testSettled": true, "testDropped": 1, "testList": [1, "two"] }"#;
    std::fs::write(&file, before).unwrap();
    let s = Settings::open(&file, &TestHooks);
    assert_eq!(s.ext.get("testFound"), Some(&json!(true)));
    assert_eq!(s.ext.get("testDropped"), None);
    assert_eq!(s.ext.get("testList"), Some(&json!([1, "two"])));
    assert_eq!(text(&file), before, "a settled file is not written on open");
    s.save(&file).unwrap();
    let written = text(&file);
    assert!(!written.contains("testDropped"), "{written}");
    assert_eq!(Settings::load(&file), s);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_extension_holds_back_the_alerts_it_says() {
    let mut s = Settings::default();
    assert_eq!(s.alert_switches(&TestHooks), s.alerts);
    s.ext.insert("testQuiet".into(), json!(true));
    let run = s.alert_switches(&TestHooks);
    assert!(!run.pool);
    assert_eq!(Switches { pool: true, ..run }, s.alerts, "and only those");
}

/// The switches of the extension's kinds of alert live in the settings
/// file's `alerts` beside the app's own: missing, each is at its default; a
/// file with a switch off its default keeps them all, written as the app's
/// are; and Cài đặt › Thông báo shows them with the app's.
#[test]
fn the_extensions_alert_switches_are_kept_in_the_settings_file_with_the_apps() {
    use common::ext::test_kind;
    let dir = fresh_dir("alerts");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    let alerts_of = |file: &Path| -> Option<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(&text(file)).unwrap().get("alerts").cloned()
    };
    let settled = r#"{ "setupSeen": true, "testSettled": true, "alerts": { "testKind": true } }"#;
    std::fs::write(&file, settled).unwrap();
    let mut s = Settings::open(&file, &TestHooks);
    assert!(s.alerts.on(&test_kind(), &TestHooks));
    assert_eq!(
        serde_json::to_value(s.alerts.shown(&TestHooks)).unwrap(),
        json!({ "pool": true, "testKind": true, "login": true, "overwrite": true })
    );
    s.save(&file).unwrap();
    assert_eq!(
        alerts_of(&file),
        Some(json!({ "pool": true, "testKind": true, "login": true, "overwrite": true }))
    );
    // Back to its default: no `alerts` written, as for the app's own.
    s.alerts.set(&test_kind(), false, &TestHooks);
    s.save(&file).unwrap();
    assert_eq!(alerts_of(&file), None);
    // One of the app's off: the extension's is written too, at its default.
    s.alerts.set(&quota_station_core::alerts::Kind::Login, false, &TestHooks);
    s.save(&file).unwrap();
    assert_eq!(
        alerts_of(&file),
        Some(json!({ "pool": true, "testKind": false, "login": false, "overwrite": true }))
    );
    assert_eq!(Settings::open(&file, &TestHooks).alerts, s.alerts);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without an extension, a switch in the settings file's `alerts` that is
/// none of the app's is kept as it was read and written back, never shown
/// and never on.
#[test]
fn without_an_extension_an_alert_switch_it_does_not_know_is_kept_and_never_on() {
    let dir = fresh_dir("alerts-none");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, r#"{ "setupSeen": true, "alerts": { "laterKind": true } }"#).unwrap();
    let s = Settings::open(&file, &NoHooks);
    let later = quota_station_core::alerts::Kind::named("laterKind");
    assert!(!s.alerts.on(&later, &NoHooks));
    assert_eq!(s.alerts.ext.get("laterKind"), Some(&json!(true)));
    assert_eq!(
        serde_json::to_value(s.alerts.shown(&NoHooks)).unwrap(),
        json!({ "pool": true, "login": true, "overwrite": true })
    );
    s.save(&file).unwrap();
    assert!(text(&file).contains(r#""laterKind": true"#), "{}", text(&file));
    let _ = std::fs::remove_dir_all(&dir);
}

/// No extension: the settings keys the app does not know pass through as
/// they were, no other key settles the file, and every alert switched on
/// runs.
#[test]
fn without_an_extension_the_settings_are_the_apps_own() {
    let dir = fresh_dir("none");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    let before = r#"{ "machineName": "Mac A", "setupSeen": true, "laterKey": { "on": true } }"#;
    std::fs::write(&file, before).unwrap();
    let s = Settings::open(&file, &NoHooks);
    assert_eq!(text(&file), before, "settled by setupSeen: not written");
    assert_eq!(s.ext.len(), 1);
    assert_eq!(s.ext.get("laterKey"), Some(&json!({ "on": true })));
    assert_eq!(Settings::open(&file, ext::none()), s);
    let changed = Settings { machine_name: Some("Mac B".into()), ..s };
    changed.save(&file).unwrap();
    assert!(text(&file).contains(r#""laterKey": {"#), "kept when written");
    assert_eq!(Settings::load(&file), changed);
    assert_eq!(changed.alert_switches(&NoHooks), changed.alerts);
    let _ = std::fs::remove_dir_all(&dir);

    // A new install writes the app's keys only.
    let dir = fresh_dir("none-new");
    let file = dir.join("settings.json");
    let s = Settings::open(&file, &NoHooks);
    assert!(s.ext.is_empty());
    assert_eq!(s, Settings { setup_seen: false, ..Settings::default() });
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text(&file)).unwrap(),
        json!({ "setupSeen": false })
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// One Claude account the app keeps, as the report gives it, with `left`
/// percent left.
fn slot(n: u32, left: f64, active: bool) -> Value {
    json!({
        "slot": n, "active": active, "status": "fresh", "ageSeconds": 60,
        "fetchedAt": "2026-01-16T07:19:00.000Z", "effectivePercentRemaining": left,
        "limitedBy": "five_hour", "resetsAt": "2026-01-16T10:29:00.000Z", "scopes": []
    })
}

/// A Claude report of the accounts the app keeps.
fn report(slots: Vec<Value>) -> Response {
    let active = slots.iter().find(|s| s["active"] == true).and_then(|s| s["slot"].as_u64());
    serde_json::from_value(json!({
        "generatedAt": "2026-01-16T07:20:00.000Z",
        "providers": [{
            "provider": "claude", "windows": [], "state": { "status": "fresh" },
            "claudeAccounts": { "activeSlot": active, "slots": slots }
        }]
    }))
    .expect("synthetic report parses")
}

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap()
}

/// A pass of the alert rules on `claude` with the pool's `policy`.
fn facts<'a>(
    claude: &'a Response,
    policy: Option<&'a dyn quota_station_core::quota::claude::PoolPolicy>,
) -> Facts<'a> {
    Facts {
        now: now(),
        offset: UtcOffset::UTC,
        claude: Some(claude),
        pool_policy: policy,
        #[cfg(feature = "antigravity-accounts")]
        agy: Vec::new(),
        #[cfg(feature = "antigravity-accounts")]
        overwrite: None,
        ext: None,
        pool: PoolAlert::default(),
    }
}

/// Team accounts 1 to 3 in the app, Claude Code on 2.
fn mac_with_accounts(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    for n in 1..=3 {
        let login = credential(&format!("acc-{n}"), &format!("ref-{n}"), in_hours(5.0));
        mac.add(n, &format!("lan{n}@team.example"), "org-team", &login);
    }
    mac.log_in(
        &credential("acc-2", "ref-2", in_hours(5.0)),
        &oauth("lan2@team.example", "uuid-2", "org-team"),
    );
    mac
}

/// The read of the accounts the app keeps asks the extension for its policy
/// of the pool where they are kept, and hands it the accounts as read.
#[test]
fn the_extensions_pool_policy_is_read_with_the_accounts_the_app_keeps() {
    let mac = mac_with_accounts("ext-pool-read");
    let report = own_now(&mac.store, &TestHooks, now()).expect("the app keeps accounts");
    assert_eq!(pool_reads(&mac.home), [(Some(2), vec![1, 2, 3])]);
    // The test policy writes no room: the row is the app's own.
    assert_eq!(own_now(&mac.store, &NoHooks, now()), Some(report));
    assert_eq!(pool_reads(&mac.home).len(), 1, "no policy without the extension");
    let _ = std::fs::remove_dir_all(&mac.home);
}

/// Alert 1's check counts the accounts the extension's policy counts: the
/// other accounts, the best of them, and whether Claude is dry follow it.
#[test]
fn alert_ones_check_counts_the_accounts_the_extensions_policy_counts() {
    let r = report(vec![
        slot(1, 10.0, false),
        slot(2, 5.0, true),
        slot(3, 12.0, false),
        slot(4, 95.0, false),
    ]);
    let policy = TestPolicy { home: PathBuf::new() };
    let check = |p| alerts::pool_check(&r, p, PoolAlert::default(), now(), UtcOffset::UTC).unwrap();
    let counted = check(Some(&policy));
    assert_eq!(counted.others, [(1, 10.0), (3, 12.0)], "the odd ones");
    assert_eq!((counted.active, counted.active_left), (Some(2), 5.0));
    assert!(counted.dry);
    assert_eq!(counted.best_other(), Some((3, 12.0)));

    // A pass of the rules with that policy sends alert 1.
    let mut memory = Memory::default();
    let ids =
        alerts::evaluate(&mut memory, &facts(&r, Some(&policy)), &Switches::default(), &TestHooks);
    assert_eq!(ids.len(), 1);
    assert_eq!(memory.recent[0].alert.kind, Kind::Pool);
    assert_eq!(memory.recent[0].alert.title, "Hạn mức Claude sắp cạn");
}

/// Alert 1's card says the extension's words once "dismiss" is pressed,
/// and the button still does what it does.
#[test]
fn alert_ones_card_has_the_extensions_words_for_its_pool() {
    let r = report(vec![slot(2, 5.0, true), slot(3, 12.0, false)]);
    for (hooks, title, body) in [
        (&TestHooks as &dyn Hooks, "Test pool noted", "Not again until the test pool has room."),
        (
            &NoHooks,
            "Đã bỏ qua thông báo này",
            "Không lặp lại cho cùng sự kiện. Quota Station báo lại khi có lại hạn mức.",
        ),
    ] {
        let mut memory = Memory::default();
        let ids = alerts::evaluate(&mut memory, &facts(&r, None), &Switches::default(), hooks);
        assert_eq!(memory.recent[0].alert.kind, Kind::Pool);
        let effect = alerts::answer(&mut memory, &ids[0], "dismiss", 0, UtcOffset::UTC, hooks);
        assert_eq!(effect, Ok(alerts::Effect::None));
        let said = memory.recent[0].answer.as_ref().unwrap().result.clone().unwrap();
        assert_eq!((said.title.as_str(), said.body.as_str()), (title, body));
    }
}

/// Without an extension there is no policy: the read writes no room and
/// every account counts in alert 1's check.
#[test]
fn without_an_extension_every_account_counts_in_the_pool() {
    let mac = Mac::new("ext-pool-none");
    assert!(NoHooks.claude_pool(&mac.paths).is_none());
    assert!(ext::none().claude_pool(&mac.paths).is_none());
    assert!(NoHooks.pool_answers().is_empty());
    let r = report(vec![
        slot(1, 10.0, false),
        slot(2, 5.0, true),
        slot(3, 12.0, false),
        slot(4, 95.0, false),
    ]);
    let all = alerts::pool_check(&r, None, PoolAlert::default(), now(), UtcOffset::UTC).unwrap();
    assert_eq!(all.others, [(1, 10.0), (3, 12.0), (4, 95.0)]);
    assert!(!all.dry);
    assert_eq!(all.best_other(), Some((4, 95.0)));
    let mut memory = Memory::default();
    let ids = alerts::evaluate(&mut memory, &facts(&r, None), &Switches::default(), &NoHooks);
    assert!(ids.is_empty(), "not dry");
    let _ = std::fs::remove_dir_all(&mac.home);
}
