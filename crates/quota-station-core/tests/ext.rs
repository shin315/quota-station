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
