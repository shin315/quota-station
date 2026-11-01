// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension (`quota_station::ext`), filled by the
//! test extension in `common` and left empty by `NoExtension`.

mod common;

use std::sync::Arc;

use common::{NotKept, TestExtension, TestState};
use quota_station::ext::{shadowed, Extension, Kept, NoExtension};

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
