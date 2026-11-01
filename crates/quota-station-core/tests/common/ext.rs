// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's test hooks (`quota_station_core::ext::Hooks`): each core
//! socket's tests fill its hook here and check that the core calls it, and
//! that `NoHooks` leaves the core as it is.

use quota_station_core::alerts::Switches;
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
}
