// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the windows can ask for. Only [`switch_claude`] acts outside the
//! app here: it switches Claude Code's login on this Mac to one of the
//! app's accounts when the owner confirms (switching Antigravity is in
//! `agy_switch`, the accounts' own actions in `claude`), and keeps the
//! switch in the history.
//! The pause commands write the rotation's pause flag; the rest write only
//! the app's own settings file.

use quota_station_core::history;
use quota_station_core::quota;
use quota_station_core::sessions::{self, Sessions};
#[cfg(feature = "author-tools")]
use quota_station_core::settings::Settings;
use quota_station_core::settings::{Language, Menubar, MenubarWindow, Theme, ThemePack};
use quota_station_core::switching::{self, Outcome};
use quota_station_core::{local_time, pause};
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

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

#[tauri::command]
pub fn open_window(app: AppHandle, tab: Tab) {
    windows::open_main(&app, tab);
}

/// The tab the main window should show when it first loads.
#[tauri::command]
pub fn main_tab(state: State<'_, AppState>) -> Tab {
    state.lock().main_tab
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

/// Cài đặt › Nâng cao: shows or hides what was made for the author's own
/// Macs (the rotation, the pause and the switcher, the other Macs, the update
/// from source, Firstmate). Hiding changes no file but the settings: a
/// rotation already running keeps running. Only in a build with the
/// `author-tools` feature.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub fn set_advanced(app: AppHandle, state: State<'_, AppState>, on: bool) -> Result<Model, String> {
    save_settings(&state, |s| s.advanced = on)?;
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// The Firstmate switch inside Nâng cao: its tab and alert 5.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub fn set_firstmate(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<Model, String> {
    save_settings(&state, |s| s.firstmate = on)?;
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

#[cfg(feature = "author-tools")]
fn save_settings(state: &AppState, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let mut inner = state.lock();
    let mut settings = inner.settings.clone();
    change(&mut settings);
    settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
    inner.settings = settings;
    Ok(())
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
/// confirmed it on the sheet. With `hold_until` (RFC 3339) the rotation is
/// paused until then. Looks at the active account again five seconds later
/// and reports what really happened.
#[tauri::command]
pub async fn switch_claude(
    app: AppHandle,
    state: State<'_, AppState>,
    slot: u32,
    hold_until: Option<String>,
) -> Result<Outcome, String> {
    let shared = state.inner().clone();
    let hold = match hold_until.as_deref() {
        Some(text) => Some(OffsetDateTime::parse(text, &Rfc3339).map_err(|_| "badTime")?),
        None => None,
    };
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
        switching::switch_claude(&tools, slot, hold, &switching::settle)
    })
    .await;
    // Kept before the reading that shows it, so the change is this switch's.
    if let Ok(d) = &done {
        let landed = match &d.outcome {
            Outcome::Switched { hold, .. } => Some(("switched", hold)),
            Outcome::Raced { hold, .. } => Some(("raced", hold)),
            Outcome::Unconfirmed { hold, .. } => Some(("unconfirmed", hold)),
            Outcome::Failed { .. } => None,
        };
        if let Some((result, hold)) = landed {
            let until = hold.as_ref().filter(|h| !h.failed).map(|h| h.until.clone());
            let at = OffsetDateTime::now_utc().unix_timestamp();
            // The same lock as the history's look at the readings.
            let _one = shared.history.lock().unwrap_or_else(|e| e.into_inner());
            let _ = history::manual_claude(&dir, from, slot, until, result, at);
        }
    }
    {
        let mut inner = shared.lock();
        inner.switching = None;
        if let Ok(Some(report)) = done.as_ref().map(|d| d.report.clone()) {
            inner.store_claude(report);
        }
        inner.reload_pause();
    }
    crate::alerting::track_history(&shared);
    shared.request_switcher();
    reader::publish(&app, &shared);
    done.map(|d| d.outcome).map_err(|e| e.to_string())
}

/// Pauses automatic switching until `until` (RFC 3339), or until the owner
/// resumes when it is `None`.
#[tauri::command]
pub fn pause_switching(
    app: AppHandle,
    state: State<'_, AppState>,
    until: Option<String>,
) -> Result<Model, String> {
    let now = OffsetDateTime::now_utc();
    let until = match until.as_deref() {
        Some(text) => {
            let t = OffsetDateTime::parse(text, &Rfc3339).map_err(|_| "badTime")?;
            if t <= now {
                return Err("past".into());
            }
            Some(local_time::minute(t))
        }
        None => None,
    };
    let flag = pause::Flag { until, reason: Some(pause::pause_reason(&local_time::clock(now))) };
    write_flag(&app, &state, Some(flag))
}

/// Moves the end of a timed pause `minutes` later, keeping why it started.
#[tauri::command]
pub fn extend_pause(
    app: AppHandle,
    state: State<'_, AppState>,
    minutes: u32,
) -> Result<Model, String> {
    let file = pause::path(&state.lock().home);
    let mut flag = pause::read(&file).map_err(|e| e.to_string())?.ok_or("notPaused")?;
    let until = flag.until.as_deref().and_then(local_time::instant).ok_or("noEnd")?;
    let from = until.max(OffsetDateTime::now_utc());
    flag.until = Some(local_time::minute(from + Duration::minutes(i64::from(minutes))));
    write_flag(&app, &state, Some(flag))
}

/// Lets the switcher switch again: removes the pause flag.
#[tauri::command]
pub fn resume_switching(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    write_flag(&app, &state, None)
}

fn write_flag(
    app: &AppHandle,
    state: &State<'_, AppState>,
    flag: Option<pause::Flag>,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let file = pause::path(&inner.home);
        match &flag {
            Some(flag) => pause::write(&file, flag),
            None => pause::remove(&file),
        }
        .map_err(|e| e.to_string())?;
        inner.reload_pause();
    }
    state.request_switcher();
    reader::publish(app, state);
    Ok(state.lock().model())
}

/// The editor has changes not saved: closing the main window asks first.
#[tauri::command]
pub fn rules_dirty(state: State<'_, AppState>, dirty: bool) {
    state.lock().rules_dirty = dirty;
}

/// Takes a pending `Thoát Quota Station` request: true once per request, for
/// the page to ask about unsaved rules (or quit when there are none).
#[tauri::command]
pub fn take_quit(state: State<'_, AppState>) -> bool {
    std::mem::take(&mut state.lock().quit_pending)
}

/// Closes the main window after the owner answered the unsaved-changes
/// dialog, or quits the app when the dialog came from `Thoát Quota Station`.
#[tauri::command]
pub fn close_main(app: AppHandle, state: State<'_, AppState>, quit: bool) {
    {
        let mut inner = state.lock();
        inner.rules_dirty = false;
        inner.quit_pending = false;
    }
    if quit {
        app.exit(0);
    } else {
        windows::close_main(&app);
    }
}
