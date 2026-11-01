// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app itself (M7): its version and launching at login.

use quota_station_core::login_item::{self, LoginItems, Status};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::login_item::MainApp;
use crate::reader;
use crate::state::{AppState, Inner, Model};

/// The commit this copy was built from, set by `scripts/install.sh`; `None`
/// for a build made another way. Ends in `-dirty` when the clone had
/// changes.
pub const COMMIT: Option<&str> = option_env!("QS_COMMIT");

/// What the app knows of itself between commands.
#[derive(Debug)]
pub struct InstallState {
    /// As macOS last reported it: at start, after the switch and when the
    /// settings open.
    pub login: Status,
}

impl Default for InstallState {
    fn default() -> Self {
        InstallState { login: Status::Off }
    }
}

/// The app's own row in settings.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    pub version: String,
    /// The first 7 characters of [`COMMIT`], with `-dirty` kept.
    pub commit: Option<String>,
    pub launch_at_login: Status,
}

fn short(commit: &str) -> String {
    let (hash, rest) = commit.split_once('-').map_or((commit, None), |(h, r)| (h, Some(r)));
    let hash: String = hash.chars().take(7).collect();
    rest.map_or(hash.clone(), |r| format!("{hash}-{r}"))
}

pub fn view(inner: &Inner) -> AppView {
    AppView {
        version: inner.app_version.clone(),
        commit: COMMIT.map(short),
        launch_at_login: inner.install.login,
    }
}

/// Asks macOS whether the app launches at login; registers nothing.
pub fn read_login(state: &AppState) {
    let status = MainApp.status();
    state.lock().install.login = status;
}

/// The settings tab opened: the owner may have changed Login Items in
/// System Settings meanwhile.
#[tauri::command]
pub fn settings_opened(app: AppHandle, state: State<'_, AppState>) {
    read_login(state.inner());
    reader::publish(&app, &state);
}

/// "Mở khi đăng nhập". Rejects with `unavailable` outside a packaged app,
/// or macOS's own message.
#[tauri::command]
pub fn set_launch_at_login(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<Model, String> {
    let result = login_item::set(&MainApp, on);
    crate::session::note("login-item", serde_json::json!({ "on": on, "ok": result.is_ok() }));
    read_login(state.inner());
    reader::publish(&app, &state);
    result.map(|_| state.lock().model()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_login_items(app: AppHandle) {
    #[cfg(windows)]
    crate::login_item::open_settings(&app);
    #[cfg(not(windows))]
    {
        let _ = app;
        crate::login_item::open_settings();
    }
}
