// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the windows can ask for. Only [`switch_claude`] acts outside the
//! app here: it switches Claude Code's login on this Mac to one of the
//! app's accounts when the owner confirms (switching Antigravity is in
//! `agy_switch`, the accounts' own actions in `claude`), and keeps the
//! switch in the history.
//! The rest write only the app's own settings file.

use quota_station_core::history;
use quota_station_core::quota;
use quota_station_core::sessions::{self, Sessions};
use quota_station_core::settings::{Language, Menubar, MenubarWindow, Theme, ThemePack};
use quota_station_core::switching::{self, Outcome};
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Model, Tab};
use crate::windows;

#[tauri::command]
pub fn get_model(state: State<'_, AppState>) -> Model {
    state.lock().model()
}

/// Asks for a read now. Returns the count of finished reads; the read asked
/// for is done once `Model.reads` passes it.
#[tauri::command]
pub fn refresh(state: State<'_, AppState>) -> u64 {
    let reads = state.lock().reads;
    state.request_read();
    reads
}

/// Opens the main window on the tab named `tab`, one of the app's own or
/// of the extension's; any other name opens nothing.
#[tauri::command]
pub fn open_window(app: AppHandle, state: State<'_, AppState>, tab: String) -> Result<(), String> {
    let named = state.lock().tab(&tab);
    windows::open_main(&app, named.ok_or_else(|| format!("unknown tab `{tab}`"))?);
    Ok(())
}

/// The tab the main window should show when it first loads.
#[tauri::command]
pub fn main_tab(state: State<'_, AppState>) -> Tab {
    state.lock().main_tab.clone()
}

/// The group Cài đặt shows.
#[tauri::command]
pub fn settings_group(state: State<'_, AppState>) -> String {
    state.lock().settings_group.clone()
}

/// The group picked in Cài đặt, kept while the app runs.
#[tauri::command]
pub fn set_settings_group(app: AppHandle, state: State<'_, AppState>, group: String) {
    state.lock().settings_group = group;
    crate::session::changed(&app);
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    windows::hide_popover(&app);
}

#[tauri::command]
pub fn popover_resize(app: AppHandle, height: f64) {
    windows::resize_popover(&app, height);
}

/// Sets the name shown for this Mac; an empty value goes back to the name
/// macOS gives it. Stays on this Mac: nothing is sent anywhere.
#[tauri::command]
pub fn set_machine_name(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.machine_name = quota_station_core::settings::machine_name(&name);
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// Shows or hides account names (emails, organizations) in both windows.
/// Hidden, they never reach the windows.
#[tauri::command]
pub fn set_show_names(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.hide_names = !on;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    // Shown again, the emails are read from the Keychain with the next read.
    if on {
        state.request_read();
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// Sets the service the menu bar title follows. Stays on this Mac.
#[tauri::command]
pub fn set_menubar(
    app: AppHandle,
    state: State<'_, AppState>,
    menubar: Menubar,
) -> Result<Model, String> {
    save_menubar(&app, &state, menubar)?;
    crate::session::changed(&app);
    Ok(state.lock().model())
}

/// The one way the menu bar's service is saved: from Cài đặt and from the
/// right-click menu.
pub fn save_menubar(app: &AppHandle, state: &AppState, menubar: Menubar) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.menubar = menubar;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(app, state);
    Ok(())
}

/// Sets the window whose share used the menu bar title shows. Stays on this
/// Mac.
#[tauri::command]
pub fn set_menubar_window(
    app: AppHandle,
    state: State<'_, AppState>,
    window: MenubarWindow,
) -> Result<Model, String> {
    save_menubar_window(&app, &state, window)?;
    crate::session::changed(&app);
    Ok(state.lock().model())
}

/// The one way the menu bar's window is saved: from Cài đặt and from the
/// right-click menu.
pub fn save_menubar_window(
    app: &AppHandle,
    state: &AppState,
    window: MenubarWindow,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.menubar_window = window;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(app, state);
    Ok(())
}

/// Sets the light or dark look of both windows, or back to macOS's.
#[tauri::command]
pub fn set_theme(
    app: AppHandle,
    state: State<'_, AppState>,
    theme: Theme,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.theme = theme;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    windows::apply_theme(&app, theme);
    reader::publish(&app, &state);
    crate::session::changed(&app);
    Ok(state.lock().model())
}

/// Sets the theme pack every window is drawn in.
#[tauri::command]
pub fn set_theme_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    pack: ThemePack,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.theme_pack = pack;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    crate::session::changed(&app);
    Ok(state.lock().model())
}

/// The first run's last step: it does not show again, and the app opens
/// where its owner chose, the main window or the popover under the menu bar.
#[tauri::command]
pub fn finish_setup(
    app: AppHandle,
    state: State<'_, AppState>,
    open: String,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.setup_seen = true;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    if open == "popover" {
        windows::close_main(&app);
        windows::show_popover(&app);
    }
    Ok(state.lock().model())
}

/// Sets the windows' language, or back to macOS's.
#[tauri::command]
pub fn set_language(
    app: AppHandle,
    state: State<'_, AppState>,
    language: Language,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.language = language;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    crate::session::changed(&app);
    Ok(state.lock().model())
}

/// Whether macOS reduces transparency: the page shows the solid ground.
#[tauri::command]
pub fn reduce_transparency() -> bool {
    windows::reduce_transparency()
}

/// Whether macOS increases contrast: the page draws stronger edges.
#[tauri::command]
pub fn increase_contrast() -> bool {
    windows::increase_contrast()
}

/// Reads Claude Code's Keychain item once with the owner at hand: macOS may
/// show its own prompt, and the read waits for the answer. Runs only when
/// the owner presses the button; afterwards the timed reads read the item
/// again.
#[tauri::command]
pub async fn grant_keychain(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let shared = state.inner().clone();
    let (store, dir) = {
        let mut inner = shared.lock();
        if inner.granting {
            return Err("busy".into());
        }
        inner.granting = true;
        (inner.claude_store(), inner.app_dir())
    };
    reader::publish(&app, &shared);
    let result = tauri::async_runtime::spawn_blocking(move || {
        reader::read_claude(&store, &dir, false, quota::claude::Keychain::Grant).0
    })
    .await
    .map_err(|e| e.to_string());
    let outcome = {
        let mut inner = shared.lock();
        inner.granting = false;
        result.map(|report| inner.store_claude(report))
    };
    reader::publish(&app, &shared);
    outcome.map(|()| shared.lock().model())
}

/// How many Claude Code sessions run on this Mac now, for the switch sheet.
/// `None` when the sessions folder cannot be read.
#[tauri::command]
pub fn claude_sessions(state: State<'_, AppState>) -> Option<Sessions> {
    let dir = sessions::dir(&state.lock().home);
    sessions::count(&dir).ok()
}

/// Switches Claude Code to the app's account `slot`, after the owner
/// confirmed it on the sheet.
#[tauri::command]
pub async fn switch_claude(
    app: AppHandle,
    state: State<'_, AppState>,
    slot: u32,
) -> Result<Outcome, String> {
    switch(app, state.inner().clone(), slot).await
}

/// Looks at the active account again five seconds after the switch and
/// reports what really happened.
async fn switch(app: AppHandle, shared: AppState, slot: u32) -> Result<Outcome, String> {
    let (tools, from, dir) = {
        let mut inner = shared.lock();
        if inner.switching.is_some() {
            return Err("busy".into());
        }
        let snapshot = inner.snapshot();
        let known = slot > 0
            && (snapshot.claude.accounts.iter().any(|a| a.number == slot)
                || inner.claude_accounts.iter().any(|a| a.slot == slot));
        if !known {
            return Err("unknownSlot".into());
        }
        let from = snapshot.claude.active().map(|a| a.number).filter(|n| *n > 0);
        inner.switching = Some(slot);
        (inner.switch_tools(), from, inner.app_dir())
    };
    reader::publish(&app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        switching::switch_claude(&tools, slot, &switching::settle)
    })
    .await;
    // Kept before the reading that shows it, so the change is this switch's.
    if let Ok(d) = &done {
        let landed = match &d.outcome {
            Outcome::Switched { .. } => Some("switched"),
            Outcome::Raced { .. } => Some("raced"),
            Outcome::Unconfirmed { .. } => Some("unconfirmed"),
            Outcome::Failed { .. } => None,
        };
        if let Some(result) = landed {
            let at = OffsetDateTime::now_utc().unix_timestamp();
            // The same lock as the history's look at the readings.
            let _one = shared.history.lock().unwrap_or_else(|e| e.into_inner());
            let _ = history::manual_claude(&dir, from, slot, result, at);
        }
    }
    {
        let mut inner = shared.lock();
        inner.switching = None;
        if let Ok(Some(report)) = done.as_ref().map(|d| d.report.clone()) {
            inner.store_claude(report);
        }
    }
    crate::alerting::track_history(&shared);
    reader::publish(&app, &shared);
    done.map(|d| d.outcome).map_err(|e| e.to_string())
}
