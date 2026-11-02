// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's hooks for an extension (`quota_station_core::ext`).

mod common;

use std::path::{Path, PathBuf};

use common::accounts::{credential, in_hours, oauth, FakeAnthropic, Mac};
use common::ext::{
    pool_reads, test_kind, TestHooks, TestPolicy, TEST_HISTORY, TEST_REFUSE, TEST_SOURCE,
};
use quota_station_core::alerts::{self, Facts, Kind, Memory, Switches};
use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::store::Store;
use quota_station_core::claude::{cli, import, state, switch};
use quota_station_core::ext::{self, Fields, Hooks, NoHooks};
use quota_station_core::history::{self, Seen, Source};
use quota_station_core::icon;
use quota_station_core::labels::Labels;
use quota_station_core::quota::claude::own_now;
use quota_station_core::quota::Response;
use quota_station_core::settings::{PoolAlert, Settings};
use quota_station_core::snapshot::PoolKind;
use quota_station_core::stats::report::Timeline;
use quota_station_core::switching::{switch_claude, Tools};
use quota_station_core::Snapshot;
use serde_json::{json, Map, Value};
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
        labels: None,
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

/// Settings with the test hooks' names on (`testLabels`).
fn labelled() -> Settings {
    let mut s = Settings::default();
    s.ext.insert("testLabels".into(), Value::Bool(true));
    s
}

/// Claude accounts 1 to 4, account 2 in use and dry, account 3 the one with
/// the most left.
fn four_accounts() -> Response {
    report(vec![slot(1, 5.0, false), slot(2, 5.0, true), slot(3, 12.0, false), slot(4, 8.0, false)])
}

/// The extension names the accounts it gives a name on the menu bar's icon
/// and on the cards, with the settings in force; an account it gives no
/// name keeps the app's.
#[test]
fn the_extensions_labels_name_the_accounts_where_the_app_shows_them() {
    let settings = labelled();
    let labels = TestHooks.account_labels(&settings).expect("on in these settings");
    let labels: Option<&dyn Labels> = Some(labels.as_ref());
    let r = four_accounts();
    // The icon, with the accounts of the snapshot's Claude section.
    let s = Snapshot::build(Some(&r), None);
    let names: Vec<String> =
        s.claude.accounts.iter().map(|a| icon::claude_name(&s.claude, a, labels)).collect();
    assert_eq!(names, ["Test 1 of 4", "Tài khoản 2", "Test 3 of 4", "Tài khoản 4"]);
    let mut agy = s.claude.accounts[0].clone();
    agy.number = 2;
    assert_eq!(icon::agy_name(&agy, labels), "Test agy 2");
    agy.email = Some("lan@example.com".into());
    assert_eq!(icon::agy_name(&agy, labels), "lan", "a name shown wins");
    // The cards: alert 1 at a pass, with the extension's title.
    let names = alerts::claude_names(Some(&r), labels);
    assert_eq!(
        names.values().collect::<Vec<_>>(),
        ["Test 1 of 4", "Tài khoản 2", "Test 3 of 4", "Tài khoản 4"]
    );
    let mut memory = Memory::default();
    let f = Facts { labels, ..facts(&r, None) };
    alerts::evaluate(&mut memory, &f, &Switches::default(), &TestHooks);
    let card = &memory.recent[0].alert;
    assert_eq!(card.title, "Test pool running dry");
    assert_eq!(card.body, "Tài khoản 2 đã dùng 95%. Test 1 of 4 có lại hạn mức lúc 10:29.");
    assert_eq!(card.actions[0].label, "Sang Test 3 of 4 (đã dùng 88%)");
    // "Gửi thử": the app's kinds, and the extension's kind gets the same names.
    let sample = |kind: &Kind| alerts::sample(kind, 0, UtcOffset::UTC, labels, &TestHooks).unwrap();
    let pool = sample(&Kind::Pool);
    assert_eq!(pool.title, "Test pool running dry");
    assert_eq!(pool.body, "Thử: Test one đã dùng 92%. Test two có lại hạn mức lúc 02:00.");
    assert_eq!(pool.actions[0].label, "Sang Test three (đã dùng 81%)");
    assert!(sample(&Kind::Overwrite)
        .body
        .contains("đổi về Test agy 1. Bạn đã chuyển sang Test agy 2"));
    assert_eq!(sample(&test_kind()).title, "Test sample for Test one at 00:00");
    #[cfg(feature = "antigravity-accounts")]
    assert_eq!(alerts::agy_refused(2, "x", labels).title, "Chưa chuyển lại được sang Test agy 2");
}

/// The extension gives no names for settings it says not to, and without
/// an extension there are none: the accounts go by number everywhere.
#[test]
fn without_an_extensions_labels_the_accounts_go_by_number() {
    assert!(TestHooks.account_labels(&Settings::default()).is_none());
    assert!(NoHooks.account_labels(&labelled()).is_none());
    assert!(ext::none().account_labels(&labelled()).is_none());
    let r = four_accounts();
    let names = alerts::claude_names(Some(&r), None);
    assert_eq!(
        names.values().collect::<Vec<_>>(),
        ["Tài khoản 1", "Tài khoản 2", "Tài khoản 3", "Tài khoản 4"]
    );
    let mut memory = Memory::default();
    alerts::evaluate(&mut memory, &facts(&r, None), &Switches::default(), &NoHooks);
    let card = &memory.recent[0].alert;
    assert_eq!(card.title, "Hạn mức Claude sắp cạn");
    assert_eq!(card.body, "Tài khoản 2 đã dùng 95%. Tài khoản 1 có lại hạn mức lúc 10:29.");
    let pool = alerts::sample(&Kind::Pool, 0, UtcOffset::UTC, None, &TestHooks).unwrap();
    assert_eq!(pool.title, "Hạn mức Claude sắp cạn");
    assert_eq!(pool.body, "Thử: Tài khoản 1 đã dùng 92%. Tài khoản 2 có lại hạn mức lúc 02:00.");
    let overwrite = alerts::sample(&Kind::Overwrite, 0, UtcOffset::UTC, None, &TestHooks).unwrap();
    assert!(overwrite.body.contains("đổi về Antigravity 1. Bạn đã chuyển sang Antigravity 2"));
}

/// `value`'s keys, in the order they are written.
fn keys_of(value: &impl serde::Serialize) -> Vec<String> {
    let text = serde_json::to_string(value).unwrap();
    serde_json::from_str::<Fields>(&text).unwrap().iter().map(|(k, _)| k.to_string()).collect()
}

/// [`four_accounts`], whose list has two keys of the extension's own:
/// `testPool` (42) and `testOther`.
fn four_accounts_with_keys() -> Response {
    let mut r = four_accounts();
    let list = r.primary_mut("claude").unwrap().claude_accounts.as_mut().unwrap();
    list.ext.insert("testPool", json!(42.0));
    list.ext.insert("testOther", json!("x"));
    r
}

/// The app's own keys of an account, of a pool and of a section.
const ACCOUNT_KEYS: [&str; 7] =
    ["number", "active", "reading", "fetchedAt", "source", "lines", "windows"];
const POOL_KEYS: [&str; 7] =
    ["kind", "percent", "line", "scope", "nextRefillAt", "exhausted", "reason"];
const SECTION_KEYS: [&str; 8] =
    ["status", "reason", "accounts", "pool", "generatedAt", "newestReadingAt", "stale", "source"];

/// The extension's part of the snapshot: its keys sit right after the
/// app's key it put them after (even one the app leaves out), or at the
/// end; it may count a pool its own way; and it reads the keys of its own
/// the report's list had, which the section keeps without sending them.
#[test]
fn the_extensions_part_of_the_snapshot_sits_among_the_apps_keys() {
    let r = four_accounts_with_keys();
    let mut s = Snapshot::build(Some(&r), None);
    assert_eq!(s.claude.list_ext.get("testPool"), Some(&json!(42.0)));
    assert_eq!(keys_of(&s.claude), SECTION_KEYS, "the list's keys are never sent");
    TestHooks.decorate_snapshot(&mut s, Some(&r));

    let first = &s.claude.accounts[0];
    assert_eq!(
        keys_of(first),
        [
            "number",
            "testNumber",
            "active",
            "reading",
            "fetchedAt",
            "source",
            "lines",
            "windows",
            "testEnd"
        ]
    );
    let v = serde_json::to_value(&s).unwrap();
    let numbers: Vec<&Value> = (0..4).map(|i| &v["claude"]["accounts"][i]["testNumber"]).collect();
    assert_eq!(numbers, [&json!(10), &json!(20), &json!(30), &json!(40)]);
    let mut named = first.clone();
    named.email = Some("lan@example.com".into());
    assert_eq!(keys_of(&named)[7..], ["windows", "email", "testEnd"]);

    let pool = &s.claude.pool;
    assert_eq!(pool.kind, PoolKind::Ext("testPool"));
    assert_eq!(
        keys_of(pool),
        ["kind", "testWhole", "percent", "line", "scope", "nextRefillAt", "exhausted", "reason"]
    );
    assert_eq!(
        (&v["claude"]["pool"]["kind"], &v["claude"]["pool"]["percent"]),
        (&json!("testPool"), &json!(42.0))
    );

    let mut section = SECTION_KEYS.map(String::from).to_vec();
    section.insert(7, "testListKeys".into());
    assert_eq!(keys_of(&s.claude), section);
    assert_eq!(v["claude"]["testListKeys"], json!(["testPool", "testOther"]));
    assert_eq!(keys_of(&s.agy), SECTION_KEYS, "the extension adds what it says only");

    // A key put after one the object never has comes at the end, as one
    // put after none does; a key put again leaves its place for the new one.
    let mut a = first.clone();
    a.ext.insert_after("noSuchKey", "testStray", json!(1));
    a.ext.insert_after("active", "testNumber", json!(0));
    assert_eq!(
        keys_of(&a),
        [
            "number",
            "active",
            "testNumber",
            "reading",
            "fetchedAt",
            "source",
            "lines",
            "windows",
            "testEnd",
            "testStray"
        ]
    );
    assert_eq!(a.ext.remove("testStray"), Some(json!(1)));
    assert_eq!(a.ext.get("testStray"), None);
}

/// Without an extension's part the snapshot is as the app builds it: the
/// app's keys only, and its own count of the pool.
#[test]
fn without_an_extension_the_snapshot_is_as_the_app_builds_it() {
    let r = four_accounts_with_keys();
    let built = Snapshot::build(Some(&r), None);
    for hooks in [&NoHooks as &dyn Hooks, ext::none()] {
        let mut s = built.clone();
        hooks.decorate_snapshot(&mut s, Some(&r));
        assert_eq!(s, built);
        assert_eq!(serde_json::to_string(&s).unwrap(), serde_json::to_string(&built).unwrap());
    }
    assert_eq!(keys_of(&built.claude.accounts[0]), ACCOUNT_KEYS);
    assert_eq!(keys_of(&built.claude.pool), POOL_KEYS);
    assert_eq!(built.claude.pool.kind, PoolKind::Active);
    assert!(built.claude.accounts.iter().all(|a| a.ext.is_empty()));
}

// The switch history.

/// 2026-01-16 07:20 UTC, for the history.
const T: i64 = 1_768_548_000;

/// An app folder whose journal has the owner's switch from 1 to 2 at `T`
/// and a change seen from 2 to 3 at `T + 100`, and a home whose
/// [`TEST_HISTORY`] has three lines of the test hooks: a switch to 4 at
/// `T`, a line to 5 at `T + 50` that is not a switch, and a switch to 6 at
/// `T + 200`.
fn history_files(tag: &str) -> (AppDir, PathBuf) {
    let root = fresh_dir(tag);
    let app = AppDir::new(root.join("app"));
    let home = root.join("home");
    let journal = format!(
        "{}\n{}\n",
        format_args!(
            r#"{{"at":{T},"app":"claude","from":"1","to":"2","what":{{"kind":"manual","result":"switched"}}}}"#
        ),
        format_args!(
            r#"{{"at":{},"app":"claude","from":"2","to":"3","what":{{"kind":"observed"}}}}"#,
            T + 100
        ),
    );
    std::fs::create_dir_all(history::path(&app).parent().unwrap()).unwrap();
    std::fs::write(history::path(&app), journal).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    let lines = json!([
        { "at": T, "to": 4, "switch": true, "note": "same second" },
        { "at": T + 50, "to": 5, "switch": false, "note": "not a switch" },
        { "at": T + 200, "to": 6, "switch": true, "note": "newest" },
    ]);
    std::fs::write(home.join(TEST_HISTORY), lines.to_string()).unwrap();
    (app, home)
}

/// The extension's lines join the app's, newest first and after the app's
/// own at the same second, with their own source and reason as the windows
/// read them; its switches count in the stats' timeline, and any of its
/// lines explains a change of the account in use soon after it.
#[test]
fn the_extensions_lines_join_the_history() {
    let (app, home) = history_files("history-ext");
    let all = history::collect(&app, &home, &TestHooks, 50);
    let order: Vec<(Source, Option<u32>)> = all.iter().map(|e| (e.source, e.to)).collect();
    let ext = Source::Ext(TEST_SOURCE);
    assert_eq!(
        order,
        [
            (ext, Some(6)),
            (Source::Observed, Some(3)),
            (ext, Some(5)),
            (Source::App, Some(2)),
            (ext, Some(4)),
        ]
    );
    assert_eq!(
        serde_json::to_string(&all[0]).unwrap(),
        r#"{"at":"2026-01-16T07:23:20Z","app":"claude","source":"testSource","from":null,"to":6,"cause":{"kind":"testCause","note":"newest"}}"#
    );
    assert_eq!(history::collect(&app, &home, &TestHooks, 2).len(), 2);

    let timeline = Timeline::from_history(&all, None);
    assert_eq!(timeline.switches, [(T, 2), (T, 4), (T + 100, 3), (T + 200, 6)]);

    let mut seen = Seen { claude: Some(3), agy: None };
    history::track(&app, &home, &TestHooks, &mut seen, Some(6), None, T + 260);
    history::track(&app, &home, &TestHooks, &mut seen, Some(5), None, T + 300);
    assert_eq!(history::read(&app).len(), 2, "both changes have a line of the extension's");
    history::track(&app, &home, &TestHooks, &mut seen, Some(7), None, T + 400);
    let kept = history::read(&app);
    assert_eq!(kept.len(), 3);
    assert_eq!((kept[2].from.as_deref(), kept[2].to.as_str()), (Some("5"), "7"));
}

/// Without an extension the history is the app's own: no line of the
/// extension's, nothing read under the home, and a change only the
/// extension's lines would explain is kept as seen.
#[test]
fn without_an_extension_the_history_is_the_apps_own() {
    let (app, home) = history_files("history-none");
    for hooks in [ext::none(), &NoHooks as &dyn Hooks] {
        let all = history::collect(&app, &home, hooks, 50);
        let order: Vec<(Source, Option<u32>)> = all.iter().map(|e| (e.source, e.to)).collect();
        assert_eq!(order, [(Source::Observed, Some(3)), (Source::App, Some(2))]);
        assert_eq!(Timeline::from_history(&all, None).switches, [(T, 2), (T + 100, 3)]);
    }
    let mut seen = Seen { claude: Some(3), agy: None };
    history::track(&app, &home, ext::none(), &mut seen, Some(6), None, T + 260);
    let kept = history::read(&app);
    assert_eq!(kept.len(), 3);
    assert_eq!((kept[2].from.as_deref(), kept[2].to.as_str()), (Some("3"), "6"));
}

/// Accounts 2, 3 and 4, Claude Code on 2, and an expired login kept for 3.
fn switch_mac(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    for slot in [2, 3, 4] {
        let login = credential(&format!("acc-{slot}"), &format!("ref-{slot}"), in_hours(5.0));
        mac.add(slot, &format!("lan.{slot}@team.example"), "org-team", &login);
    }
    mac.log_in(
        &mac.store.credential(2).unwrap().unwrap(),
        &oauth("lan.2@team.example", "uuid-2", "org-team"),
    );
    mac
}

fn quiet() {}

/// `quota-station claude ARGS…` with `hooks`: the exit code, stdout and
/// stderr.
fn claude_cli(mac: &Mac, hooks: &dyn Hooks, args: &[&str]) -> (i32, String, String) {
    let cli = cli::Cli {
        store: mac.store.clone(),
        tools: cli::Tools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
    };
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(&cli, hooks, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

/// What the extension does around the owner's manual switch: it reads its
/// own arguments of the ask, may refuse the ask or the switch before
/// anything is switched (in the app and in a terminal), sees where the
/// switch landed and what the second look found, puts its keys at the end
/// of the outcome (in its own order where it wrote them so) and keeps its
/// own on the journal's line, which the History tab shows its way.
#[test]
fn the_extension_acts_around_a_manual_switch() {
    let mac = switch_mac("switch-around");
    let tools = Tools { store: mac.store.clone() };
    let mut asked = Map::new();
    asked.insert("testBad".into(), json!(true));
    assert_eq!(TestHooks.manual_switch(&asked).err(), Some("testBadAsk".to_string()));

    std::fs::write(mac.home.join(TEST_REFUSE), "").unwrap();
    let around = TestHooks.manual_switch(&Map::new()).unwrap();
    let done = switch_claude(&tools, 3, around, &quiet, &TestHooks);
    assert_eq!(
        serde_json::to_string(&done.outcome).unwrap(),
        r#"{"kind":"failed","slot":3,"error":"testRefused","detail":"refused by the test","active":null}"#
    );
    assert_eq!((done.journal, done.report.is_none()), (Fields::new(), true));
    assert_eq!(switch::active_slot(&mac.store), Some(2), "nothing was switched");
    assert_eq!(
        claude_cli(&mac, &TestHooks, &["switch", "3"]),
        (1, String::new(), "quota-station claude switch: the test hooks refuse it\n".into())
    );
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    std::fs::remove_file(mac.home.join(TEST_REFUSE)).unwrap();

    asked = Map::new();
    asked.insert("slot".into(), json!(3));
    asked.insert("testNote".into(), json!("from the window"));
    let around = TestHooks.manual_switch(&asked).unwrap();
    let done = switch_claude(&tools, 3, around, &quiet, &TestHooks);
    assert_eq!(
        serde_json::to_string(&done.outcome).unwrap(),
        r#"{"kind":"switched","slot":3,"testNote":"from the window","testLanded":3,"testActive":3,"testOrder":{"zulu":1,"alpha":2}}"#
    );
    assert_eq!(serde_json::to_string(&done.journal).unwrap(), r#"{"testKept":3}"#);

    let other = || {
        switch::switch(&mac.store, 2).unwrap();
    };
    let around = TestHooks.manual_switch(&Map::new()).unwrap();
    let raced = switch_claude(&tools, 4, around, &other, &TestHooks);
    assert_eq!(
        serde_json::to_string(&raced.outcome).unwrap(),
        r#"{"kind":"raced","slot":4,"active":2,"testNote":null,"testLanded":4,"testActive":2,"testOrder":{"zulu":1,"alpha":2}}"#
    );

    let app = AppDir::new(mac.dir.join("app"));
    history::manual_claude(&app, Some(2), 3, done.journal, "switched", T).unwrap();
    let line = std::fs::read_to_string(history::path(&app)).unwrap();
    assert_eq!(
        line.trim_end(),
        format!(
            r#"{{"at":{T},"app":"claude","from":"2","to":"3","what":{{"kind":"manual","testKept":3,"result":"switched"}}}}"#
        )
    );
    let shown = |hooks: &dyn Hooks| {
        serde_json::to_string(&history::collect(&app, &mac.home, hooks, 1)[0].cause).unwrap()
    };
    assert_eq!(shown(&TestHooks), r#"{"kind":"manual","testShown":3,"result":"switched"}"#);
    // Without the extension its keys stay in the journal, and the History
    // tab shows the app's own.
    assert_eq!(shown(ext::none()), r#"{"kind":"manual","result":"switched"}"#);
}

/// Without an extension nothing is done around a manual switch: nothing is
/// asked, refused or added, the History tab shows a manual switch's line
/// without keys of the extension's, and a login's token is refreshed when
/// due.
#[test]
fn without_an_extension_a_manual_switch_is_the_apps_own() {
    let mut kept = Fields::new();
    kept.insert("testKept", json!(3));
    for hooks in [ext::none(), &NoHooks as &dyn Hooks] {
        let mut asked = Map::new();
        asked.insert("testBad".into(), json!(true));
        assert!(hooks.manual_switch(&asked).unwrap().is_none());
        assert_eq!(hooks.manual_line(&kept), Fields::new());
        assert!(hooks.may_refresh_logins());
    }
}

/// `quota-station claude status --refresh` refreshes a login's token only
/// while the extension lets it.
#[test]
fn the_extension_may_hold_a_logins_refresh_back() {
    for (hooks, refreshed) in [(&TestHooks as &dyn Hooks, false), (ext::none(), true)] {
        let mac = switch_mac(if refreshed { "refresh-none" } else { "refresh-held" });
        mac.store.set_credential(3, &credential("acc-3", "ref-3", in_hours(-1.0))).unwrap();
        let anthropic = FakeAnthropic::start();
        anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
        for access in ["acc-2", "acc-3b", "acc-4"] {
            anthropic.usage(access, 5.0, None, None, None);
        }
        let cli = cli::Cli {
            store: mac.store.clone(),
            tools: cli::Tools { claude: None, endpoints: anthropic.endpoints() },
        };
        let args = ["status".to_string(), "--refresh".to_string()];
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(cli::run(&cli, hooks, &args, &mut out, &mut err), 0);
        assert_eq!(anthropic.to("/v1/oauth/token").len(), usize::from(refreshed));
    }
}

// The Claude accounts' file.

/// The extension's keys on the Claude accounts: kept on each account the
/// file has (one without them gets them), written back right after its
/// alias, shown in the state file after the alias and before the name on
/// `claude status`, carried on each account's slot by the read of the
/// accounts after `active`, and a file whose keys the extension refuses is
/// not read.
#[test]
fn the_extension_keeps_its_keys_on_the_claude_accounts() {
    let mac = switch_mac("account-keys");
    let store = Store::new(mac.paths.clone(), mac.keychain.clone()).with_hooks(&TestHooks);
    assert!(store.account_keys().is_some());
    let mut list = store.load().unwrap();
    assert_eq!(list.get(2).unwrap().ext.get("testTag"), Some(&json!("none")));
    list.get_mut(3).unwrap().alias = Some("Ba".into());
    list.get_mut(3).unwrap().ext.insert("testTag", json!("blue"));
    store.save(&list).unwrap();
    let file = std::fs::read_to_string(mac.paths.accounts()).unwrap();
    let three =
        "\"slot\": 3,\n      \"alias\": \"Ba\",\n      \"testTag\": \"blue\",\n      \"kind\"";
    let two = "\"slot\": 2,\n      \"testTag\": \"none\",\n      \"kind\"";
    assert!(file.contains(three) && file.contains(two), "{file}");

    state::write(&store, Some(2), OffsetDateTime::from_unix_timestamp(T).unwrap()).unwrap();
    let written = std::fs::read_to_string(store.paths.state()).unwrap();
    let shown = "\"alias\": \"Ba\",\n      \"testState\": \"blue!\",\n      \"active\"";
    assert!(written.contains(shown), "{written}");
    let cli = cli::Cli {
        store: store.clone(),
        tools: cli::Tools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(cli::run(&cli, &NoHooks, &["status".to_string()], &mut out, &mut err), 0);
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().map(str::trim_end).collect();
    assert!(lines[0].ends_with("noData") && lines[1].ends_with("noData   tagged  Ba"), "{out}");

    let report = own_now(&store, &NoHooks, OffsetDateTime::from_unix_timestamp(T).unwrap());
    let report = report.unwrap();
    let slots = &report.primary("claude").unwrap().claude_accounts.as_ref().unwrap().slots;
    assert_eq!(slots[1].ext.get("testTag"), Some(&json!("blue")));
    assert_eq!(keys_of(&slots[1])[..4], ["slot", "active", "testTag", "status"]);

    let refused = file.replace("\"testTag\": \"blue\"", "\"testTag\": 5");
    std::fs::write(mac.paths.accounts(), refused).unwrap();
    assert_eq!(
        store.load().unwrap_err().to_string(),
        "accounts.json cannot be read: testTag is not text"
    );
}

/// Without an extension the file keeps only the app's keys: what it does
/// not know is read and left out, nothing is shown of it, and `claude
/// status` names the accounts alone.
#[test]
fn without_an_extension_the_claude_accounts_keep_only_the_apps_keys() {
    let mac = switch_mac("account-keys-none");
    for hooks in [ext::none(), &NoHooks as &dyn Hooks] {
        assert!(hooks.claude_account_keys().is_none());
        let store = Store::new(mac.paths.clone(), mac.keychain.clone()).with_hooks(hooks);
        assert!(store.account_keys().is_none());
        let text = std::fs::read_to_string(mac.paths.accounts()).unwrap();
        let tagged = text.replace("\"kind\"", "\"testTag\": \"blue\",\n      \"kind\"");
        std::fs::write(mac.paths.accounts(), tagged).unwrap();
        let list = store.load().unwrap();
        assert!(list.accounts.iter().all(|a| a.ext.is_empty()));
        store.save(&list).unwrap();
        let file = std::fs::read_to_string(mac.paths.accounts()).unwrap();
        assert!(!file.contains("testTag") && !file.contains("disabled"), "{file}");
        let state = quota_station_core::claude::state::build(
            &list,
            &Default::default(),
            None,
            None,
            OffsetDateTime::UNIX_EPOCH,
        );
        assert!(state.accounts.iter().all(|a| a.ext.is_empty()));
    }
}

// An import of claude-swap's accounts.

/// claude-swap's folder under `mac`'s home with one account, 1, its login
/// in claude-swap's Keychain item.
fn claude_swap_one(mac: &Mac) -> PathBuf {
    let dir = import::source(&mac.home);
    std::fs::create_dir_all(dir.join("configs")).unwrap();
    let email = "lan@example.com";
    let sequence = json!({ "accounts": { "1": { "email": email, "organizationUuid": "org-1" } } });
    std::fs::write(dir.join("sequence.json"), sequence.to_string()).unwrap();
    let config = json!({ "oauthAccount": oauth(email, "uuid-1", "org-1") });
    std::fs::write(dir.join(format!("configs/.claude-config-1-{email}.json")), config.to_string())
        .unwrap();
    let login = credential("acc-1", "ref-1", in_hours(5.0));
    mac.put_item("claude-swap", &format!("account-1-{email}"), &login);
    dir
}

/// What the extension brings over is asked once the accounts are in, for
/// the accounts the app keeps, and ends the import's outcome in the order
/// the extension put it, in the app and on `claude import`.
#[test]
fn the_extension_brings_its_own_over_at_the_end_of_an_import() {
    let mac = Mac::new("ext-import");
    let dir = claude_swap_one(&mac);
    let done = import::import(&mac.store, &dir, &TestHooks).unwrap();
    assert_eq!(done.added, vec![1]);
    assert_eq!(done.ext.get("testAccounts"), Some(&json!(1)));
    assert_eq!(done.ext.get("testHome"), Some(&json!(mac.home.display().to_string())));
    let own = ["added", "updated", "unchanged", "skipped"];
    assert_eq!(keys_of(&done), [&own[..], &["testAccounts", "testHome"]].concat());

    let (code, out, err) = claude_cli(&mac, &TestHooks, &["import"]);
    assert_eq!((code, err.as_str()), (0, ""));
    let printed: Fields = serde_json::from_str(&out).unwrap();
    let keys: Vec<&str> = printed.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, [&own[..], &["testAccounts", "testHome"]].concat());
    assert_eq!(printed.get("unchanged"), Some(&json!([1])));
}

/// Without an extension an import brings over only the app's own, and its
/// outcome has only the app's keys.
#[test]
fn without_an_extension_an_import_brings_only_the_apps_own() {
    for (n, hooks) in [ext::none(), &NoHooks as &dyn Hooks].into_iter().enumerate() {
        let mac = Mac::new(&format!("ext-import-none-{n}"));
        assert!(hooks.claude_imported(&mac.paths).is_empty());
        let dir = claude_swap_one(&mac);
        let done = import::import(&mac.store, &dir, hooks).unwrap();
        assert_eq!(done.added, vec![1]);
        assert!(done.ext.is_empty());
        assert_eq!(keys_of(&done), ["added", "updated", "unchanged", "skipped"]);
        let (code, out, _) = claude_cli(&mac, hooks, &["import"]);
        assert_eq!(code, 0);
        let printed: Fields = serde_json::from_str(&out).unwrap();
        let keys: Vec<&str> = printed.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, ["added", "updated", "unchanged", "skipped"]);
    }
}
