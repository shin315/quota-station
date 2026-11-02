// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A test extension, made as a crate outside the app makes one: only
//! through `quota_station::ext` and `quota_station::extension_commands!`.
//! Each socket's tests fill the socket here and check that the app calls it.
//! A test reads what needs no window runtime (the commands' names, never
//! their handler, never the host): a Windows test program cannot load
//! Tauri's runtime.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use quota_station::ext::{
    AlertInput, AlertRead, Badge, Closing, Extension, Hold, Hooks, Host, Kept, ModelInput,
    ReaderDone, ReaderPass, ReaderWork, Shape,
};
use quota_station_core::alerts::{Alert, ExtPass, ExtSource};
use serde_json::{json, Map, Value};
use time::OffsetDateTime;

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
/// starts: what it read of the settings, a count its commands and its work
/// at the reader's passes keep (and its part of the model shows), whether it
/// asks the reader to go now, the passes it saw (`read_due`,
/// `window_open`), and whether it holds work not kept yet (so a close
/// waits) and the closes it was asked about, the machine names it read
/// for its alerts and had back once the alert rules ran, and until when it
/// badges the menu bar icon.
#[derive(Debug, Default)]
pub struct TestState {
    pub alert_reads: Mutex<Vec<Option<String>>>,
    pub machine: Option<String>,
    pub count: Mutex<u32>,
    pub asks: Mutex<bool>,
    pub passes: Mutex<Vec<(bool, bool)>>,
    pub unkept: Mutex<bool>,
    pub closes: Mutex<Vec<Closing>>,
    pub switches: Mutex<Vec<PathBuf>>,
    pub badge_until: Mutex<Option<OffsetDateTime>>,
}

/// The test extension's badge on the menu bar icon: one square in the
/// middle of the gauge.
pub fn test_badge() -> Badge {
    Badge {
        shapes: vec![Shape { x: 7.5, y: 7.5, width: 3.0, height: 3.0, radius: 0.5 }],
        halo: 0.5,
        words: "Huy hiệu thử".into(),
    }
}

/// What the test extension read for its alerts: the machine's name, from
/// the settings. It draws no alert.
pub struct TestRead {
    pub machine: Option<String>,
}

impl ExtSource for TestRead {
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
        let _ = pass;
        Vec::new()
    }
}

impl AlertRead for TestRead {
    fn source(&self) -> &dyn ExtSource {
        self
    }

    fn done(self: Box<Self>, kept: &Kept) {
        if let Some(state) = kept.get::<TestState>() {
            state.alert_reads.lock().unwrap().push(self.machine);
        }
    }
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

    fn start_threads(&self, host: Host) {
        let _ = host.kept_state::<TestState>();
    }

    fn reader_asked(&self, kept: &Kept) -> bool {
        kept.get::<TestState>().is_some_and(|state| *state.asks.lock().unwrap())
    }

    fn reader_pass(&self, pass: &ReaderPass<'_>) -> Option<ReaderWork> {
        let state = pass.kept.get::<TestState>()?;
        *state.asks.lock().unwrap() = false;
        state.passes.lock().unwrap().push((pass.read_due, pass.window_open));
        // Alone between reads it counts 10, beside a read 1.
        let add = if pass.read_due { 1 } else { 10 };
        Some(Box::new(move || {
            let done: ReaderDone = Box::new(move |kept: &Kept| {
                *kept.get::<TestState>().expect("kept").count.lock().unwrap() += add;
            });
            done
        }))
    }

    /// `testExtension` in the model: what it read of the settings and the
    /// accounts, and its count, from the state it keeps.
    fn model(&self, input: &ModelInput<'_>) -> Map<String, Value> {
        let count = input.kept.get::<TestState>().map(|state| *state.count.lock().unwrap());
        let part = json!({
            "machine": input.settings.machine_name,
            "accounts": input.claude_accounts.iter().map(|a| a.slot).collect::<Vec<_>>(),
            "count": count,
        });
        Map::from_iter([("testExtension".to_string(), part)])
    }

    /// `testExtension` in what the build has.
    fn capabilities(&self) -> Map<String, Value> {
        Map::from_iter([("testExtension".to_string(), Value::Bool(true))])
    }

    /// Its own tab, and one named like an app's tab, which stays the app's.
    fn tabs(&self) -> &[&'static str] {
        &["testExtension", "settings"]
    }

    /// While it holds work not kept, the main window's close asks on the tab
    /// in view and a quit asks on its own tab.
    fn hold_close(&self, kept: &Kept, closing: Closing) -> Hold {
        let Some(state) = kept.get::<TestState>() else { return Hold::Close };
        state.closes.lock().unwrap().push(closing);
        if !*state.unkept.lock().unwrap() {
            return Hold::Close;
        }
        match closing {
            Closing::MainWindow => Hold::Ask { tab: None },
            Closing::App => Hold::Ask { tab: Some("testExtension") },
        }
    }

    /// It reads the machine's name for its alerts.
    fn alert_read(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>> {
        Some(Box::new(TestRead { machine: input.settings.machine_name.clone() }))
    }

    /// After a manual switch it notes the home it was handed and asks the
    /// reader for a pass, once it keeps its state.
    fn after_switch(&self, kept: &Kept, home: &Path) -> bool {
        let Some(state) = kept.get::<TestState>() else { return false };
        state.switches.lock().unwrap().push(home.to_path_buf());
        *state.asks.lock().unwrap() = true;
        true
    }

    /// Its badge on the icon, until the time its state names.
    fn icon_badge(&self, kept: &Kept, now: OffsetDateTime) -> Option<Badge> {
        let until = (*kept.get::<TestState>()?.badge_until.lock().unwrap())?;
        (now < until).then(test_badge)
    }

    /// `quota-station test-extension ARGS…` exits with as many as there
    /// are ARGS, plus 5; anything else is not its own.
    fn subcommand(&self, args: &[String]) -> Option<i32> {
        (args.first()? == "test-extension").then(|| 5 + args.len() as i32 - 1)
    }

    quota_station::extension_commands![
        tools::test_extension_echo,
        #[cfg(unix)]
        tools::test_extension_unix,
        tools::get_model,
    ];
}
