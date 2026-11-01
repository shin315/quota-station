// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A test extension, made as a crate outside the app makes one: only
//! through `quota_station::ext` and `quota_station::extension_commands!`.
//! Each socket's tests fill the socket here and check that the app calls it.
//! A test reads what needs no window runtime (the commands' names, never
//! their handler, never the host): a Windows test program cannot load
//! Tauri's runtime.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use quota_station::ext::{Extension, Hooks, Host};

/// The test extension's commands.
pub mod tools {
    /// A command no build of the app has.
    #[tauri::command]
    pub fn test_extension_echo(text: String) -> String {
        text
    }

    /// A command built on Unix only, as an extension may list one.
    #[cfg(unix)]
    #[tauri::command]
    pub fn test_extension_unix() -> bool {
        true
    }

    /// Named like one of the app's own commands, which stays the app's.
    #[tauri::command]
    pub fn get_model() -> bool {
        false
    }
}

/// The hooks the test extension gives the core.
pub struct TestHooks;

impl Hooks for TestHooks {}

/// The test extension's own state, which it keeps in the app when it
/// starts: what it read of the settings, and a count its commands keep.
#[derive(Debug, Default)]
pub struct TestState {
    pub machine: Option<String>,
    pub count: Mutex<u32>,
}

/// Another type, which the test extension never keeps.
#[derive(Debug, Default)]
pub struct NotKept;

/// The test extension: fills every socket there is with something a test
/// can see.
pub struct TestExtension;

impl Extension for TestExtension {
    fn hooks(&self) -> &dyn Hooks {
        &TestHooks
    }

    fn start(&self, host: Host) {
        let state = TestState { machine: host.settings().machine_name, ..TestState::default() };
        host.keep_state(Arc::new(state));
    }

    quota_station::extension_commands![
        tools::test_extension_echo,
        #[cfg(unix)]
        tools::test_extension_unix,
        tools::get_model,
    ];
}
