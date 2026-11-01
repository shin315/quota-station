// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The `quota-station` binary: the app itself is the `quota_station`
//! library (`lib.rs`), run with this crate's `tauri.conf.json` and the
//! extension this build has (`quota_station::ext`).

// A window app on Windows: a release build opens no console of its own.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let extension = quota_station::ext::NoExtension;
    quota_station::run(tauri::generate_context!(), extension)
}
