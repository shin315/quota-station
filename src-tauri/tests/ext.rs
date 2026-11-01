// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension (`quota_station::ext`), filled by the
//! test extension in `common` and left empty by `NoExtension`.

mod common;

use std::path::Path;
use std::sync::Arc;

use common::{NotKept, TestExtension, TestState};
use quota_station::ext::{
    shadowed, shadowed_tabs, Extension, Kept, ModelInput, NoExtension, ReaderPass,
};
use quota_station_core::app_dir::AppDir;
use quota_station_core::settings::Settings;
use serde_json::json;

#[test]
fn no_extension_adds_no_command() {
    assert!(NoExtension.command_names().is_empty());
    assert!(shadowed(&[]).is_empty());
}

#[test]
fn an_extension_lists_its_commands_as_the_windows_call_them() {
    let mut expected = vec!["test_extension_echo"];
    if cfg!(unix) {
        expected.push("test_extension_unix");
    }
    expected.push("get_model");
    assert_eq!(TestExtension.command_names(), expected);
}

#[test]
fn a_command_the_app_has_stays_the_apps() {
    assert_eq!(shadowed(&TestExtension.command_names()), vec!["get_model"]);
}

#[test]
fn the_state_an_extension_keeps_is_found_again_by_its_type() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState { machine: Some("Mac A".into()), ..TestState::default() });
    kept.keep(state.clone());
    let found = kept.get::<TestState>().expect("kept");
    assert!(Arc::ptr_eq(&found, &state), "the same state, not a copy");
    *found.count.lock().unwrap() += 1;
    assert_eq!(*state.count.lock().unwrap(), 1);
    assert!(kept.get::<NotKept>().is_none(), "only the types kept");

    // Kept again, the newer one is found.
    let newer = Arc::new(TestState::default());
    kept.keep(newer.clone());
    assert!(Arc::ptr_eq(&kept.get::<TestState>().unwrap(), &newer));
}

#[test]
fn without_an_extension_nothing_is_kept() {
    let kept = Kept::default();
    assert!(kept.get::<TestState>().is_none());
    assert!(kept.get::<NotKept>().is_none());
}

fn pass(kept: &Kept, app_dir: &AppDir, read_due: bool) -> bool {
    let pass = ReaderPass {
        kept,
        home: Path::new("/nonexistent/home"),
        app_dir,
        read_due,
        window_open: false,
    };
    match TestExtension.reader_pass(&pass) {
        Some(work) => {
            work()(kept);
            true
        }
        None => false,
    }
}

#[test]
fn an_extension_asks_the_reader_and_works_at_its_passes() {
    let app_dir = AppDir::new("/nonexistent/app");
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(!TestExtension.reader_asked(&kept));
    *state.asks.lock().unwrap() = true;
    assert!(TestExtension.reader_asked(&kept), "the reader goes now");

    // Alone between reads: the pass takes the ask, the work finishes.
    assert!(pass(&kept, &app_dir, false));
    assert!(!TestExtension.reader_asked(&kept));
    assert_eq!(*state.count.lock().unwrap(), 10);

    // Beside a read.
    assert!(pass(&kept, &app_dir, true));
    assert_eq!(*state.count.lock().unwrap(), 11);
    assert_eq!(*state.passes.lock().unwrap(), vec![(false, false), (true, false)]);
}

#[test]
fn without_an_extension_the_reader_neither_waits_for_it_nor_runs_its_work() {
    let app_dir = AppDir::new("/nonexistent/app");
    let kept = Kept::default();
    assert!(!NoExtension.reader_asked(&kept));
    for read_due in [false, true] {
        let pass = ReaderPass {
            kept: &kept,
            home: Path::new("/nonexistent/home"),
            app_dir: &app_dir,
            read_due,
            window_open: true,
        };
        assert!(NoExtension.reader_pass(&pass).is_none());
    }
    // An extension that keeps nothing yet does no work either.
    assert!(!pass(&kept, &app_dir, true));
}

fn model_input<'a>(kept: &'a Kept, settings: &'a Settings, app_dir: &'a AppDir) -> ModelInput<'a> {
    ModelInput {
        kept,
        settings,
        home: Path::new("/nonexistent/home"),
        app_dir,
        claude_accounts: &[],
    }
}

#[test]
fn an_extension_draws_its_part_of_the_model_from_what_the_app_hands_it() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings { machine_name: Some("Mac A".into()), ..Settings::default() };
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    *state.count.lock().unwrap() = 3;
    let part = TestExtension.model(&model_input(&kept, &settings, &app_dir));
    assert_eq!(
        serde_json::Value::Object(part),
        json!({ "testExtension": { "machine": "Mac A", "accounts": [], "count": 3 } })
    );
    assert_eq!(
        serde_json::Value::Object(TestExtension.capabilities()),
        json!({ "testExtension": true })
    );

    // Before it keeps its state, it draws what it has.
    let empty = Kept::default();
    let part = TestExtension.model(&model_input(&empty, &settings, &app_dir));
    assert_eq!(part["testExtension"]["count"], serde_json::Value::Null);
}

#[test]
fn without_an_extension_the_model_and_the_capabilities_gain_nothing() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings::default();
    let mut kept = Kept::default();
    kept.keep(Arc::new(TestState::default()));
    assert!(NoExtension.model(&model_input(&kept, &settings, &app_dir)).is_empty());
    assert!(NoExtension.capabilities().is_empty());
}

#[test]
fn an_extension_names_its_tabs_and_a_tab_the_app_has_stays_the_apps() {
    assert_eq!(TestExtension.tabs(), ["testExtension", "settings"]);
    assert_eq!(shadowed_tabs(TestExtension.tabs()), ["settings"]);
}

#[test]
fn without_an_extension_the_app_has_only_its_own_tabs() {
    assert!(NoExtension.tabs().is_empty());
    assert!(shadowed_tabs(&[]).is_empty());
}
