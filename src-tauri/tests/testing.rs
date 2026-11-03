// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's state for tests outside the crate (`quota_station::testing`),
//! used as an extension's own tests use it, with the test extension in
//! `common`.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{TestExtension, TestState};
use quota_station::ext::{Closing, Extension, NoExtension, Tab};
use quota_station::testing::{fixed_model_json, OnClose, TestApp};
use quota_station_core::ext::Fields;
use serde_json::{json, Value};

/// An app with `extension`, its settings file nowhere.
fn app<E: Extension>(extension: &'static E) -> TestApp {
    TestApp::new(PathBuf::new(), PathBuf::from("/nonexistent/settings.json"), "Mac", extension)
}

/// A folder of the test's own, which `fixed_model_json` removes.
fn folder(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("qs-testing-{name}-{}", std::process::id()))
}

/// The fixed model of a build with an extension carries the extension's
/// part, drawn from the settings file, the accounts' fields and the state it
/// keeps; without that part it is the fixed model of a build without one.
#[test]
fn the_fixed_model_carries_an_extensions_part_beside_the_apps_own() {
    let plain =
        fixed_model_json(&folder("plain"), &NoExtension, |_, _| {}, |_| Fields::new(), |_| {});
    let dir = folder("with");
    let mut handed = None;
    let mut json = fixed_model_json(
        &dir,
        &TestExtension,
        |home, _| handed = Some(home.to_path_buf()),
        |slot| {
            let mut fields = Fields::new();
            fields.insert("testSlot", slot.into());
            fields
        },
        |app| {
            let state = TestState { count: 3.into(), ..TestState::default() };
            app.keep(Arc::new(state));
        },
    );
    assert_eq!(handed, Some(dir.join("home")), "the settings are handed the home folder");
    assert!(!dir.exists(), "the folder is removed");

    let accounts = json["claudeAccounts"]["accounts"].as_array_mut().unwrap();
    for (account, slot) in accounts.iter_mut().zip(1..) {
        assert_eq!(account["testSlot"], slot);
        account.as_object_mut().unwrap().remove("testSlot");
    }
    let Value::Object(mut with) = json else { panic!("an object") };
    let part = with.remove("testExtension").unwrap();
    assert_eq!(part, json!({ "machine": "Mac phòng khách", "accounts": [1, 2, 3], "count": 3 }));
    let capabilities = with["capabilities"].as_object_mut().unwrap();
    assert_eq!(capabilities.remove("testExtension"), Some(true.into()));
    let accounts = with["claudeAccounts"].as_object_mut().unwrap();
    assert_eq!(accounts.remove("testCount"), Some(3.into()));
    let app = with["app"].as_object_mut().unwrap();
    assert_eq!(app.remove("testMachine"), Some("Mac phòng khách".into()));
    assert_eq!(Value::Object(with), plain);
}

/// The answer to a close follows the extension's hold: nothing held closes
/// at once, held work asks first, on the extension's tab for a quit.
/// Without an extension it closes, whatever is kept.
#[test]
fn the_answer_to_a_close_follows_the_extensions_hold() {
    let mut with = app(&TestExtension);
    assert_eq!(with.on_close(Closing::App), OnClose::Close, "nothing kept");
    let state = Arc::new(TestState::default());
    with.keep(state.clone());
    assert_eq!(with.on_close(Closing::MainWindow), OnClose::Close);
    *state.unkept.lock().unwrap() = true;
    assert_eq!(with.on_close(Closing::MainWindow), OnClose::Ask(None));
    assert_eq!(with.on_close(Closing::App), OnClose::Ask(Some(Tab::Ext("testExtension".into()))));
    assert_eq!(
        *state.closes.lock().unwrap(),
        [Closing::MainWindow, Closing::MainWindow, Closing::App]
    );

    let mut without = app(&NoExtension);
    without.keep(state);
    assert_eq!(without.on_close(Closing::MainWindow), OnClose::Close);
    assert_eq!(without.on_close(Closing::App), OnClose::Close);
}
