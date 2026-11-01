// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's test hooks (`quota_station_core::ext::Hooks`): each core
//! socket's tests fill its hook here and check that the core calls it, and
//! that `NoHooks` leaves the core as it is.

use std::io::Write;

use quota_station_core::alerts::Switches;
use quota_station_core::claude::cli::Cli;
use quota_station_core::ext::Hooks;
use quota_station_core::settings::Settings;
use serde_json::{Map, Value};

/// Hooks a test can see the core call.
///
/// The settings file: they keep their own keys, all named `test…`. A file
/// is settled once it has `testSettled`, which a new install writes off and
/// a Mac that had the app on; every open notes whether a file was read
/// (`testFound`) and drops `testDropped`. While `testQuiet` is on, alert 1
/// does not run.
///
/// `quota-station claude …`: they add `test-word` (and would take `status`
/// too, were it ever asked), with a usage of their own.
#[derive(Debug, Default)]
pub struct TestHooks;

impl Hooks for TestHooks {
    fn settled_key(&self) -> Option<&'static str> {
        Some("testSettled")
    }

    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        ext.insert("testFound".into(), Value::Bool(found));
        ext.remove("testDropped");
    }

    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        ext.insert("testSettled".into(), Value::Bool(had_app));
    }

    fn alert_switches(&self, settings: &Settings, mut on: Switches) -> Switches {
        if settings.ext.get("testQuiet") == Some(&Value::Bool(true)) {
            on.pool = false;
        }
        on
    }

    /// `test-word ARGS…` prints `test ARGS…` and a note on stderr, and
    /// exits 3; `status` would exit 9; anything else is not theirs.
    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        let _ = cli;
        match args.first().map(String::as_str) {
            Some("test-word") => {
                let _ = writeln!(out, "test {}", args[1..].join(" "));
                let _ = writeln!(err, "noted");
                Some(3)
            }
            Some("status") => Some(9),
            _ => None,
        }
    }

    fn claude_usage(&self) -> Option<&'static str> {
        Some(TEST_USAGE)
    }
}

/// The usage the test hooks give `quota-station claude`.
pub const TEST_USAGE: &str = "usage: quota-station claude test-word ARGS...\n";
