// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Agent › Cập nhật (Cập nhật Agent): Claude Code, the Codex CLI and the
//! Antigravity CLI. Once a day, while "Tự động kiểm tra" is on, or when the
//! owner presses "Kiểm tra ngay", the app reads each tool's version and asks
//! where it is published for the newest; a newer one is told at the bell.
//! A tool is updated only when the owner presses its "Cập nhật", with the
//! tool's own command; Codex waits while one of its sessions is open.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use quota_station_core::alerts;
use quota_station_core::cli_tools::{self, Feeds, Found, Tool, ToolView};
use quota_station_core::os::ProcessList;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner};

/// A check that did not start (or ran) is not tried again before this.
const RETRY: Duration = Duration::from_secs(60 * 60);

/// What the app found last, the check running now and the update running now.
#[derive(Debug, Default)]
pub struct Live {
    pub checking: bool,
    pub tried: Option<Instant>,
    /// `None` until the tools were first looked for.
    pub found: Option<Vec<Found>>,
    pub running: BTreeMap<Tool, u32>,
    pub updating: Option<Tool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliToolsView {
    /// "Tự động kiểm tra".
    pub daily: bool,
    pub checking: bool,
    /// Unix seconds of the last check that ended.
    pub checked_at: Option<i64>,
    /// No feed answered the last check.
    pub offline: bool,
    pub tools: Vec<ToolView>,
}

pub fn view(inner: &Inner) -> CliToolsView {
    let kept = cli_tools::load(&inner.app_dir());
    let live = &inner.cli_tools;
    let found = live.found.as_deref().unwrap_or_default();
    CliToolsView {
        daily: !inner.settings.cli_checks_off,
        checking: live.checking,
        checked_at: kept.checked_at,
        offline: kept.offline,
        tools: Tool::ALL
            .into_iter()
            .map(|t| {
                cli_tools::row(
                    t,
                    found.iter().find(|f| f.tool == t),
                    &kept,
                    live.running.get(&t).copied().unwrap_or(0),
                    live.updating == Some(t),
                    &inner.home,
                )
            })
            .collect(),
    }
}

/// Reads each tool's version and counts its sessions; asks the feeds too
/// when `fetch`, and tells the bell about versions not told before.
fn look(app: &AppHandle, state: &AppState, fetch: bool) {
    let (home, dir) = {
        let inner = state.lock();
        (inner.home.clone(), inner.app_dir())
    };
    let found = cli_tools::find_all(&home);
    let all = quota_station_core::os::processes().all().unwrap_or_default();
    let running = found.iter().map(|f| (f.tool, cli_tools::running(f, &all))).collect();
    if fetch {
        let before = cli_tools::load(&dir);
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let after = cli_tools::check(&Feeds::live(), &found, &home, &before, now);
        let _ = cli_tools::save(&dir, &after);
        let newer: Vec<(String, String, String)> = found
            .iter()
            .filter_map(|f| {
                let r = cli_tools::row(f.tool, Some(f), &after, 0, false, &home);
                match (r.newer, r.version, r.latest) {
                    (true, Some(from), Some(to)) => Some((f.tool.label().to_string(), from, to)),
                    _ => None,
                }
            })
            .collect();
        let mut inner = state.lock();
        if alerts::cli_alert(&mut inner.alerts, &newer, now).is_some() {
            let _ = alerts::save(&dir, &inner.alerts);
        }
    }
    {
        let mut inner = state.lock();
        inner.cli_tools.found = Some(found);
        inner.cli_tools.running = running;
    }
    reader::publish(app, state);
}

fn start(app: &AppHandle, state: &AppState) -> bool {
    {
        let mut inner = state.lock();
        if inner.cli_tools.checking {
            return false;
        }
        inner.cli_tools.checking = true;
        inner.cli_tools.tried = Some(Instant::now());
    }
    reader::publish(app, state);
    true
}

fn end(app: &AppHandle, state: &AppState) {
    state.lock().cli_tools.checking = false;
    reader::publish(app, state);
}

/// "Kiểm tra ngay", and "Kiểm lại" on a row.
#[tauri::command]
pub async fn cli_tools_check(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    if !start(&app, &shared) {
        return Ok(());
    }
    let (a, s) = (app.clone(), shared.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &s, true)).await;
    end(&app, &shared);
    Ok(())
}

/// After each read: the tools are looked for once after the app starts,
/// and their feeds asked once a day while "Tự động kiểm tra" is on (an hour
/// after a check that reached none).
pub fn daily(app: &AppHandle, state: &AppState) {
    let fetch = {
        let inner = state.lock();
        if inner.cli_tools.checking
            || inner.cli_tools.updating.is_some()
            || inner.cli_tools.tried.is_some_and(|t| t.elapsed() < RETRY)
        {
            return;
        }
        let due = !inner.settings.cli_checks_off
            && cli_tools::load(&inner.app_dir()).due(OffsetDateTime::now_utc().unix_timestamp());
        if !due && inner.cli_tools.found.is_some() {
            return;
        }
        due
    };
    if !start(app, state) {
        return;
    }
    let (a, s) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        let (a2, s2) = (a.clone(), s.clone());
        let _ = tauri::async_runtime::spawn_blocking(move || look(&a2, &s2, fetch)).await;
        end(&a, &s);
    });
}

/// The page opened: while "Tự động kiểm tra" is on, a check when the last
/// one is missing or due (a day old, an hour after one that reached no
/// feed), even within the hour after a check that did not start.
#[tauri::command]
pub async fn cli_tools_opened(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    {
        let inner = shared.lock();
        let due = cli_tools::load(&inner.app_dir()).due(OffsetDateTime::now_utc().unix_timestamp());
        if inner.settings.cli_checks_off || inner.cli_tools.updating.is_some() || !due {
            return Ok(());
        }
    }
    cli_tools_check(app, state).await
}

/// "Cập nhật" on a row: the tool's own command, never while another
/// update or a check runs, never for an install left to the owner, and for
/// Codex never while one of its sessions is open (the core counts them
/// again just before).
#[tauri::command]
pub async fn cli_tool_update(
    app: AppHandle,
    state: State<'_, AppState>,
    tool: String,
) -> Result<(), String> {
    let tool = Tool::parse(&tool).ok_or("unknown")?;
    let shared = state.inner().clone();
    let (home, dir) = {
        let mut inner = shared.lock();
        if inner.cli_tools.updating.is_some() || inner.cli_tools.checking {
            return Err("busy".into());
        }
        let home = inner.home.clone();
        let found = inner.cli_tools.found.as_ref().and_then(|f| f.iter().find(|f| f.tool == tool));
        if found.is_none() {
            return Err("notInstalled".into());
        }
        inner.cli_tools.updating = Some(tool);
        (home, inner.app_dir())
    };
    reader::publish(&app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = cli_tools::locate(tool, &home) else { return Err("notInstalled") };
        let found = cli_tools::inspect(tool, &path, &home);
        let log = dir.path().join(cli_tools::LOG_DIR).join(format!("{}.log", tool.program()));
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let processes = quota_station_core::os::processes();
        let last = cli_tools::update(
            &found,
            &processes as &dyn ProcessList,
            &log,
            &home,
            cli_tools::UPDATE_TIMEOUT,
            now,
        )
        .map_err(|_| "handsOff")?;
        let mut kept = cli_tools::load(&dir);
        kept.last.insert(tool, last);
        let _ = cli_tools::save(&dir, &kept);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r.map_err(String::from));
    shared.lock().cli_tools.updating = None;
    // The versions and sessions as they are now.
    let (a, s) = (app.clone(), shared.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &s, false)).await;
    done
}

/// Cập nhật Agent: "Tự động kiểm tra".
#[tauri::command]
pub fn set_cli_checks(
    app: AppHandle,
    state: State<'_, AppState>,
    daily: bool,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.cli_checks_off = !daily;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(())
}
