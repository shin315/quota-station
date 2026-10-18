//! What the windows can ask for. None of these switch an account or write
//! anywhere but the app's own settings file.

use quota_station_core::runner::{self, RunError};
use tauri::{AppHandle, State};

use crate::reader;
use crate::state::{AppState, Model};
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
pub fn open_settings(app: AppHandle) {
    windows::open_settings(&app);
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
        inner.quota_axi()
    };
    reader::publish(&app, &shared);
    let result = tauri::async_runtime::spawn_blocking(move || match path {
        Some(p) => runner::grant_keychain(&p),
        None => Err(RunError::NotConfigured),
    })
    .await
    .unwrap_or_else(|e| Err(RunError::Spawn(e.to_string())));
    let outcome = {
        let mut inner = shared.lock();
        inner.granting = false;
        match result {
            Ok(report) => {
                inner.store_grant(report);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    };
    reader::publish(&app, &shared);
    outcome.map(|()| shared.lock().model())
}
