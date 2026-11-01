// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The `quota-station` binary: the app itself is the `quota_station`
//! library (`lib.rs`).

// A window app on Windows: a release build opens no console of its own.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    quota_station::run()
}
