// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's hooks for an extension (`quota_station_core::ext`).

mod common;

use std::path::{Path, PathBuf};

use common::ext::TestHooks;
use quota_station_core::alerts::Switches;
use quota_station_core::ext::{self, Hooks, NoHooks};
use quota_station_core::settings::Settings;
use serde_json::json;

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
