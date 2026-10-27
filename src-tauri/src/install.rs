// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app itself (M7): its version, launching at login, and
//! "Cập nhật từ mã nguồn", which pulls the clone set in settings and runs
//! its `scripts/install.sh` over the running copy, then opens the new copy
//! when the owner asks. The update from source is one of the author's
//! tools: only a build with the `author-tools` feature has it.

#[cfg(feature = "author-tools")]
use std::path::{Path, PathBuf};
#[cfg(feature = "author-tools")]
use std::process::{Command, Stdio};

use quota_station_core::login_item::{self, LoginItems, Status};
#[cfg(feature = "author-tools")]
use quota_station_core::runner;
#[cfg(feature = "author-tools")]
use quota_station_core::update::{self, Outcome, SourceError, Step, Tools, UpdateError};
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
    #[cfg(feature = "author-tools")]
    pub updating: Option<Step>,
    #[cfg(feature = "author-tools")]
    pub outcome: Option<Outcome>,
    #[cfg(feature = "author-tools")]
    pub error: Option<UpdateError>,
}

impl Default for InstallState {
    fn default() -> Self {
        InstallState {
            login: Status::Off,
            #[cfg(feature = "author-tools")]
            updating: None,
            #[cfg(feature = "author-tools")]
            outcome: None,
            #[cfg(feature = "author-tools")]
            error: None,
        }
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
    /// The update from source; only in a build with the author's tools.
    #[cfg(feature = "author-tools")]
    #[serde(flatten)]
    pub source: SourceView,
}

/// "Mã nguồn" and "Cập nhật từ mã nguồn" in Cài đặt › Nâng cao.
#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    /// `~/…`; `None` when not set.
    pub source: Option<String>,
    /// `notSet`, `ok`, `missing` or `notSource`.
    pub source_status: &'static str,
    /// This copy runs from a `.app`, so an update has somewhere to go.
    pub bundled: bool,
    pub updating: Option<Step>,
    pub outcome: Option<Outcome>,
    pub error: Option<UpdateError>,
    /// The last install's output, `~/…`.
    pub log: String,
}

fn short(commit: &str) -> String {
    let (hash, rest) = commit.split_once('-').map_or((commit, None), |(h, r)| (h, Some(r)));
    let hash: String = hash.chars().take(7).collect();
    rest.map_or(hash.clone(), |r| format!("{hash}-{r}"))
}

#[cfg(feature = "author-tools")]
fn log_file(inner: &Inner) -> PathBuf {
    inner.app_dir().path().join("update/install.log")
}

pub fn view(inner: &Inner) -> AppView {
    AppView {
        version: inner.app_version.clone(),
        commit: COMMIT.map(short),
        launch_at_login: inner.install.login,
        #[cfg(feature = "author-tools")]
        source: source_view(inner),
    }
}

#[cfg(feature = "author-tools")]
fn source_view(inner: &Inner) -> SourceView {
    let source = inner.settings.source_dir.as_deref();
    let source_status = match source.map(update::check_source) {
        None => "notSet",
        Some(Ok(())) => "ok",
        Some(Err(e)) => e.code(),
    };
    SourceView {
        source: source.map(|p| runner::display_path(p, &inner.home)),
        source_status,
        bundled: running_dest().is_some(),
        updating: inner.install.updating,
        outcome: inner.install.outcome.clone(),
        error: inner.install.error.clone(),
        log: runner::display_path(&log_file(inner), &inner.home),
    }
}

/// The folder holding the `.app` this copy runs from.
#[cfg(feature = "author-tools")]
fn running_dest() -> Option<PathBuf> {
    std::env::current_exe().ok().as_deref().and_then(update::install_dest)
}

/// The `.app` this copy runs from.
#[cfg(feature = "author-tools")]
fn running_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    update::install_dest(&exe)?;
    exe.ancestors().nth(3).map(Path::to_path_buf)
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

/// The clone to update from; empty forgets it. Rejects with `missing` or
/// `notSource`.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub fn set_source_dir(
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
            update::check_source(&p).map_err(|e: SourceError| e.code().to_string())?;
            Some(p)
        };
        let mut settings = inner.settings.clone();
        settings.source_dir = chosen;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// "Cập nhật từ mã nguồn": `git pull --ff-only` in the clone, then its
/// install script into the folder this copy runs from, unless this copy was
/// built from that commit already. The outcome comes in the model. Rejects
/// with `busy`, `noSource` or `unbundled`.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub async fn update_from_source(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Model, String> {
    let shared = state.inner().clone();
    let dest = running_dest().ok_or("unbundled")?;
    let (source, log) = {
        let mut inner = shared.lock();
        if inner.install.updating.is_some() {
            return Err("busy".into());
        }
        let source = inner.settings.source_dir.clone().ok_or("noSource")?;
        inner.install.updating = Some(Step::Pulling);
        inner.install.outcome = None;
        inner.install.error = None;
        (source, log_file(&inner))
    };
    reader::publish(&app, &shared);
    let (step_app, step_state) = (app.clone(), shared.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        let on_step = |step: Step| {
            step_state.lock().install.updating = Some(step);
            reader::publish(&step_app, &step_state);
        };
        update::update(&Tools::system(), &source, &dest, COMMIT, Some(&log), &on_step)
    })
    .await
    .unwrap_or_else(|e| Err(UpdateError::Spawn { detail: e.to_string() }));
    {
        let mut inner = shared.lock();
        inner.install.updating = None;
        match result {
            Ok(outcome) => inner.install.outcome = Some(outcome),
            Err(error) => inner.install.error = Some(error),
        }
    }
    reader::publish(&app, &shared);
    let model = shared.lock().model();
    Ok(model)
}

/// "Mở lại" after an update: quits, and a small shell waits for this
/// process to end before opening the new copy (opening it while this one
/// runs would only bring this one forward). Rejects with `notInstalled`,
/// `rulesDirty` (the rules editor holds changes) or `unbundled`.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub fn relaunch(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let inner = state.lock();
        if !matches!(inner.install.outcome, Some(Outcome::Installed { .. })) {
            return Err("notInstalled".into());
        }
        if inner.rules_dirty {
            return Err("rulesDirty".into());
        }
    }
    let bundle = running_bundle().ok_or("unbundled")?;
    let wait = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open "$2""#;
    Command::new("/bin/sh")
        .args(["-c", wait, "sh"])
        .arg(std::process::id().to_string())
        .arg(&bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}
