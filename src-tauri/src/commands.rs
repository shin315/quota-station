//! What the windows can ask for. Only [`switch_claude`] acts outside the
//! app here: it switches Claude Code's login on this Mac to one of the
//! app's accounts when the owner confirms (switching Antigravity is in
//! `agy_switch`, the accounts' own actions in `claude`), and keeps the
//! switch in the history.
//! The pause commands write the rotation's pause flag; the rest write only
//! the app's own settings file.

use quota_station_core::history;
use quota_station_core::runner::{self, RunError};
use quota_station_core::sessions::{self, Sessions};
use quota_station_core::settings::{Menubar, MenubarWindow, Theme};
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

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    windows::hide_popover(&app);
}

#[tauri::command]
pub fn popover_resize(app: AppHandle, height: f64) {
    windows::resize_popover(&app, height);
}

/// Sets the quota-axi path; an empty value goes back to finding it.
/// Refuses a path with no file, and keeps the old one.
#[tauri::command]
pub fn set_quota_axi_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let chosen = if path.trim().is_empty() {
            None
        } else {
            let p = runner::expand_path(&path, &inner.home);
            if !p.is_file() {
                return Err("notFound".into());
            }
            Some(p)
        };
        let mut settings = inner.settings.clone();
        settings.quota_axi_path = chosen;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    state.request_read();
    reader::publish(&app, &state);
    Ok(state.lock().model())
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
    Ok(state.lock().model())
}

/// Lets quota-axi ask macOS for Keychain access once, then reads again.
/// Runs only when the user presses the button; macOS shows its own prompt.
#[tauri::command]
pub async fn grant_keychain(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let shared = state.inner().clone();
    let path = {
        let mut inner = shared.lock();
        if inner.granting {
            return Err("busy".into());
        }
        inner.granting = true;
        (inner.quota_axi(), inner.claude_paths().quota_axi_env())
    };
    reader::publish(&app, &shared);
    let result = tauri::async_runtime::spawn_blocking(move || match path {
        (Some(p), env) => runner::grant_keychain(&p, &env),
        (None, _) => Err(RunError::NotConfigured),
    })
    .await
    .unwrap_or_else(|e| Err(RunError::Spawn(e.to_string())));
    let outcome = {
        let mut inner = shared.lock();
        inner.granting = false;
        match result {
            Ok(report) => {
                inner.store_claude(report);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
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
