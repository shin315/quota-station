// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension (`quota_station::ext`), filled by the
//! test extension in `common` and left empty by `NoExtension`.

mod common;

use common::TestExtension;
use quota_station::ext::{shadowed, Extension, NoExtension};

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
