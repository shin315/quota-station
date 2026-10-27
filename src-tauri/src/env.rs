// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The variables that override a login (M14, I): Cài đặt's card, the Claude
//! tab's banner and one alert per variable, from
//! `quota_station_core::agents::env_check`.
//!
//! The check runs after a read when the last one is older than
//! [`EVERY`], and when "Kiểm lại" is pressed or Cài đặt opens. It knows
//! names and places only; the app never edits a shell file and never
//! unsets anything. "Bỏ qua" is kept in the app's folder.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use quota_station_core::agents::env_check::{self, Check, Found, Place, VARS};
use quota_station_core::alerts;
use quota_station_core::local_time;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::open;
use crate::reader;
use crate::state::{AppState, Inner};

/// A check after a read, at most this often.
const EVERY: Duration = Duration::from_secs(10 * 60);

/// What the app holds between checks.
#[derive(Debug, Default)]
pub struct Live {
    pub check: Option<Check>,
    /// RFC 3339.
    pub at: Option<String>,
    pub checking: bool,
    pub tried: Option<Instant>,
    /// "Bỏ qua", as saved; `None` until the first check loads it.
    pub ignored: Option<BTreeSet<String>>,
}

/// Cài đặt's card and the Claude tab's banner.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvView {
    /// RFC 3339; none before the first check.
    pub checked_at: Option<String>,
    pub checking: bool,
    /// The eleven names, and the six places' labels, for "Xem danh sách".
    pub vars: Vec<&'static str>,
    pub places: Vec<String>,
    /// Set and not ignored.
    pub found: Vec<Found>,
    /// How many "Bỏ qua" hides.
    pub ignored: usize,
    pub unread: Vec<Place>,
}

pub fn view(inner: &Inner) -> EnvView {
    let live = &inner.env;
    let ignored = live.ignored.clone().unwrap_or_default();
    let (found, hidden) = match &live.check {
        Some(c) => {
            let shown: Vec<Found> =
                c.found.iter().filter(|f| !ignored.contains(&f.id)).cloned().collect();
            let hidden = c.found.len() - shown.len();
            (shown, hidden)
        }
        None => (Vec::new(), 0),
    };
    EnvView {
        checked_at: live.at.clone(),
        checking: live.checking,
        vars: VARS.iter().map(|v| v.name).collect(),
        places: Place::system().iter().map(|p| p.label()).collect(),
        found,
        ignored: hidden,
        unread: live.check.as_ref().map(|c| c.unread.clone()).unwrap_or_default(),
    }
}

/// One check: the files and launchd, then the alerts for what is new.
fn run(state: &AppState) {
    let (home, dir) = {
        let inner = state.lock();
        (inner.home.clone(), inner.app_dir())
    };
    let check = env_check::check(&home, Path::new(env_check::LAUNCHCTL));
    let now = OffsetDateTime::now_utc();
    {
        let mut inner = state.lock();
        let mut ignored = inner.env.ignored.take().unwrap_or_else(|| env_check::load_ignored(&dir));
        let before = ignored.len();
        let shown = env_check::shown(&check, &mut ignored);
        if ignored.len() != before {
            let _ = env_check::save_ignored(&dir, &ignored);
        }
        if !alerts::env_alerts(&mut inner.alerts, &shown, now.unix_timestamp()).is_empty() {
            let _ = alerts::save(&dir, &inner.alerts);
        }
        inner.env.ignored = Some(ignored);
        inner.env.check = Some(check);
        inner.env.at = now.format(&Rfc3339).ok();
    }
}

fn start(app: &AppHandle, state: &AppState) -> bool {
    {
        let mut inner = state.lock();
        if inner.env.checking {
            return false;
        }
        inner.env.checking = true;
        inner.env.tried = Some(Instant::now());
    }
    reader::publish(app, state);
    true
}

fn end(app: &AppHandle, state: &AppState) {
    state.lock().env.checking = false;
    reader::publish(app, state);
}

/// After each read: when the last check is older than [`EVERY`].
pub fn every(app: &AppHandle, state: &AppState) {
    {
        let inner = state.lock();
        if inner.env.checking || inner.env.tried.is_some_and(|t| t.elapsed() < EVERY) {
            return;
        }
    }
    if !start(app, state) {
        return;
    }
    let (a, s) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        let s2 = s.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || run(&s2)).await;
        end(&a, &s);
    });
}

/// "Kiểm lại", and Cài đặt opening.
#[tauri::command]
pub async fn env_check(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    if !start(&app, &shared) {
        return Ok(());
    }
    let s = shared.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || run(&s)).await;
    end(&app, &shared);
    Ok(())
}

/// "Mở file": the shell file the last check found `id` in, at its line.
/// Resolves to the editor that went to the line, or null when the default
/// text editor opened the file without it.
#[tauri::command]
pub fn env_open(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<String>, String> {
    let (home, line) = {
        let inner = state.lock();
        let found = inner.env.check.as_ref().and_then(|c| c.found.iter().find(|f| f.id == id));
        let found = found.ok_or("notFound")?;
        (inner.home.clone(), found.line)
    };
    let (_, place) = env_check::parse_id(&id).ok_or("notFound")?;
    let file = home.join(place.file().ok_or("noFile")?);
    #[cfg(windows)]
    if let Some(editor) = code_at(&file, line) {
        return Ok(Some(editor.to_string()));
    }
    let (args, editor) = env_check::open_args(&file, line, &env_check::app_folders(&home));
    match (editor, args.first()) {
        (Some(_), Some(url)) => open::url(&app, url).map_err(|_| "open".to_string())?,
        _ => open::text(&app, &file).map_err(|_| "open".to_string())?,
    }
    Ok(editor.map(str::to_string))
}

/// Windows: VS Code at the line, when `code` is on the `PATH`.
#[cfg(windows)]
fn code_at(file: &std::path::Path, line: Option<u32>) -> Option<&'static str> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let line = line?;
    let code = quota_station_core::runner::on_path("code")?;
    std::process::Command::new(code)
        .arg("--goto")
        .arg(format!("{}:{line}", file.display()))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;
    Some("Visual Studio Code")
}

/// Adds `ids` to "Bỏ qua", or takes them off; ignoring also answers the
/// variables' alerts nobody answered yet.
fn set_ignored(app: &AppHandle, state: &AppState, ids: &[String], on: bool) -> Result<(), String> {
    if ids.iter().any(|id| env_check::parse_id(id).is_none()) {
        return Err("notFound".into());
    }
    {
        let mut inner = state.lock();
        let dir = inner.app_dir();
        let mut ignored = inner.env.ignored.take().unwrap_or_else(|| env_check::load_ignored(&dir));
        for id in ids {
            if on {
                ignored.insert(id.clone());
            } else {
                ignored.remove(id);
            }
        }
        let saved = env_check::save_ignored(&dir, &ignored);
        inner.env.ignored = Some(ignored);
        saved.map_err(|e| e.to_string())?;
        if on {
            let now = OffsetDateTime::now_utc();
            let names: BTreeSet<&str> =
                ids.iter().filter_map(|id| id.split_once('@')).map(|(n, _)| n).collect();
            let mut answered = false;
            for name in names {
                answered |= !alerts::env_ignored(
                    &mut inner.alerts,
                    name,
                    now.unix_timestamp(),
                    local_time::offset_at(now),
                )
                .is_empty();
            }
            if answered {
                let _ = alerts::save(&dir, &inner.alerts);
            }
        }
    }
    reader::publish(app, state);
    Ok(())
}

/// "Bỏ qua" on one place.
#[tauri::command]
pub fn env_ignore(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    set_ignored(&app, &state, &[id], true)
}

/// "Hoàn tác" after "Bỏ qua".
#[tauri::command]
pub fn env_unignore(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    set_ignored(&app, &state, &[id], false)
}

/// "Bỏ qua" on an alert card: every place of `name` the last check found.
pub fn ignore_name(app: &AppHandle, state: &AppState, name: &str) -> Result<(), String> {
    let ids: Vec<String> = {
        let inner = state.lock();
        let found = inner.env.check.as_ref().map(|c| c.found.as_slice()).unwrap_or_default();
        found.iter().filter(|f| f.name == name).map(|f| f.id.clone()).collect()
    };
    set_ignored(app, state, &ids, true)
}
