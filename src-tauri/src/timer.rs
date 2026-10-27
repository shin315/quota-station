// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation on Windows, which has no launchd: while the owner has it on
//! (`claude::timer`'s flag), the app runs it itself every three minutes, as
//! launchd runs `quota-station claude rotate` on macOS. A run refreshes the
//! kept accounts' tokens that are due and switches as the rules say. It
//! runs only while the app does; the next start runs once at once.

use std::thread;
use std::time::{Duration, Instant};

use quota_station_core::claude::{rotate, timer};
use quota_station_core::os::Scheduler;

use crate::state::AppState;

/// How often the timer looks whether a run is due, or was just turned on.
const TICK: Duration = Duration::from_secs(15);

pub fn spawn(state: AppState) {
    thread::Builder::new()
        .name("rotation-timer".into())
        .spawn(move || {
            let scheduler = quota_station_core::os::scheduler();
            let mut last: Option<Instant> = None;
            loop {
                let (store, tools) = {
                    let inner = state.lock();
                    let tools = rotate::Tools {
                        claude: inner.claude_program.clone(),
                        ..rotate::Tools::system(&inner.home)
                    };
                    (inner.claude_store(), tools)
                };
                let on = scheduler.loaded() == Some(true) && store.paths.in_use();
                if !on {
                    // Turned on later, it runs at once.
                    last = None;
                } else if timer::due(on, last.map(|t| t.elapsed())) {
                    last = Some(Instant::now());
                    rotate::run(&store, &tools);
                    crate::session::note("rotation-run", serde_json::json!({}));
                    state.request_switcher();
                }
                thread::sleep(TICK);
            }
        })
        .expect("the rotation timer starts");
}
