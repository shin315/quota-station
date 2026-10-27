// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The guided sessions on a Windows PC (plan 4.3): a test build, made with
//! `QS_TEST_SESSION` set (CI's Windows installer), opens a small window of
//! steps beside the app. The owner only follows them: he never counts or
//! reports what he saw. The app keeps the evidence itself, in its folder's
//! `session/`: each step with the result of its self-checks, the events of
//! the windows, the tray and the toasts while it ran, pictures of its own
//! windows and of the tray corner, and the tray icon as drawn. The last
//! step writes all of it, with the diagnostic info, `settings.json` and the
//! logs, to `diag-<date>.zip` ([`crate::diag`]) for the owner to drag into
//! a chat.
//!
//! Any other build has none of it: [`ON`] is false, the guide never opens,
//! and every hook here returns at once.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use time::OffsetDateTime;

use crate::state::{AppState, Inner};
use crate::{diag, open, release, windows};

/// A test build for a guided session.
pub const ON: bool = option_env!("QS_TEST_SESSION").is_some();

/// The guide's window.
pub const GUIDE: &str = "guide";
const GUIDE_SIZE: (f64, f64) = (400.0, 600.0);

/// Pictures in one step, at most; the zip keeps its own limit too.
const SHOTS_PER_STEP: u32 = 8;

/// The session while the app runs.
struct Live {
    /// The session's folder: `session/` in the app's folder.
    dir: PathBuf,
    progress: Progress,
    /// What the step on screen wants pictures of (`popover`, `main`, `toast`).
    captures: Vec<String>,
    shots: u32,
    /// The pictures taken in the session so far, which number the next.
    seq: usize,
    /// When this copy of the app started, Unix seconds.
    started: i64,
    /// Claude Code's login file when the step began, to tell a switch wrote it.
    login_at_step: Option<SystemTime>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn live() -> std::sync::MutexGuard<'static, Option<Live>> {
    LIVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Where the guide is, kept in `progress.json` so a restart (or a sign-out
/// and in) comes back to the same step.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Progress {
    step: Option<String>,
    /// When the step on screen began, Unix seconds.
    step_at: i64,
    done: Vec<String>,
    /// Every start of the app during the session, Unix seconds.
    starts: Vec<i64>,
    /// The zip last written.
    zip: Option<PathBuf>,
}

impl Progress {
    fn load(dir: &Path) -> Progress {
        std::fs::read_to_string(dir.join("progress.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, dir: &Path) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("progress.json"), text);
        }
    }
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn stamp() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Claude Code's login file in the owner's default folder.
fn login_file(home: &Path) -> PathBuf {
    home.join(".claude").join(".credentials.json")
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Starts the session of a test build: its folder, the start, and the
/// guide's window. Nothing in any other build.
pub fn start(app: &AppHandle) {
    if !ON {
        return;
    }
    let (dir, home) = {
        let state = app.state::<AppState>();
        let mut inner = state.lock();
        // No account's name or email in the windows, the tray's tooltip or
        // the toasts while the app takes pictures of them.
        inner.settings.hide_names = true;
        (inner.app_dir().path().join("session"), inner.home.clone())
    };
    for sub in ["shots", "tray"] {
        let _ = std::fs::create_dir_all(dir.join(sub));
    }
    let mut progress = Progress::load(&dir);
    let started = now();
    progress.starts.push(started);
    progress.save(&dir);
    let seq = std::fs::read_dir(dir.join("shots")).map(|r| r.count()).unwrap_or(0);
    *live() = Some(Live {
        dir,
        progress,
        captures: Vec::new(),
        shots: 0,
        seq,
        started,
        login_at_step: modified(&login_file(&home)),
    });
    note("start", json!({ "version": app.package_info().version.to_string() }));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        note("panic", json!({ "what": info.to_string() }));
        previous(info);
    }));
    open_guide(app);
}

/// Shows the guide, making its window the first time: at the top left of
/// the work area, above the other windows, and kept out of the pictures.
pub fn open_guide(app: &AppHandle) {
    if !ON {
        return;
    }
    if let Some(window) = app.get_webview_window(GUIDE) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let mut builder = WebviewWindowBuilder::new(app, GUIDE, WebviewUrl::App("index.html".into()))
        .title("Phiên thử · Quota Station")
        .inner_size(GUIDE_SIZE.0, GUIDE_SIZE.1)
        .min_inner_size(340.0, 420.0)
        .always_on_top(true);
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        builder = builder.position(
            f64::from(work.position.x) / scale + 16.0,
            f64::from(work.position.y) / scale + 16.0,
        );
    }
    if let Ok(window) = builder.build() {
        #[cfg(windows)]
        crate::shot::exclude(&window);
        let _ = window.set_focus();
    }
}

/// Notes an event of the session: the step on screen, the time, what
/// happened. Nothing outside a session.
pub fn note(kind: &str, data: Value) {
    let guard = live();
    let Some(live) = guard.as_ref() else { return };
    let line = json!({
        "at": stamp(),
        "step": live.progress.step,
        "kind": kind,
        "data": data,
    });
    if let Ok(mut file) =
        std::fs::OpenOptions::new().create(true).append(true).open(live.dir.join("events.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Takes a picture of `what` after `delay`, when the step on screen wants
/// such pictures. Nothing outside a session.
pub fn shoot(app: &AppHandle, what: &str, delay: Duration) {
    let wanted = live().as_ref().is_some_and(|l| l.captures.iter().any(|c| c == what));
    if wanted {
        take(app, what, delay);
    }
}

/// Takes a picture of `what` after `delay`, as the guide asked.
fn take(app: &AppHandle, what: &str, delay: Duration) {
    let name = {
        let mut guard = live();
        let Some(live) = guard.as_mut() else { return };
        if live.shots >= SHOTS_PER_STEP {
            return;
        }
        live.shots += 1;
        live.seq += 1;
        let step = live.progress.step.clone().unwrap_or_else(|| "none".into());
        live.dir.join("shots").join(format!("{:03}-{step}-{what}.jpg", live.seq))
    };
    let (app, what) = (app.clone(), what.to_string());
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        picture(&app, &what, &name);
    });
}

/// The picture itself: the part of the screen `what` names.
#[cfg(windows)]
fn picture(app: &AppHandle, what: &str, file: &Path) {
    let Some((x, y, w, h)) = crate::shot::area(app, what) else {
        note("shot", json!({ "what": what, "taken": false, "why": "nothing to take" }));
        return;
    };
    match crate::shot::take(x, y, w, h) {
        Some(jpeg) => {
            let _ = std::fs::write(file, jpeg);
            let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
            note("shot", json!({ "what": what, "taken": true, "file": name, "w": w, "h": h }));
        }
        None => note("shot", json!({ "what": what, "taken": false, "why": "capture failed" })),
    }
}

/// No pictures off Windows: the session is Windows's.
#[cfg(not(windows))]
fn picture(_: &AppHandle, what: &str, _: &Path) {
    note("shot", json!({ "what": what, "taken": false, "why": "not Windows" }));
}

/// A setting the owner changed in the main window: a picture of it, when
/// the step on screen wants one.
pub fn changed(app: &AppHandle) {
    shoot(app, "main", Duration::from_millis(900));
}

/// The tray icon as drawn, kept once per size, taskbar and number.
#[cfg(windows)]
pub fn tray_drawn(title: &str, size: u32, light: bool, rgba: &[u8]) {
    let file = {
        let guard = live();
        let Some(live) = guard.as_ref() else { return };
        let number: String = title.chars().filter(char::is_ascii_digit).collect();
        let number = if number.is_empty() { "glyph".to_string() } else { number };
        let taskbar = if light { "light" } else { "dark" };
        live.dir.join("tray").join(format!("{size}px-{taskbar}-{number}.png"))
    };
    let number = file.file_stem().map(|n| n.to_string_lossy().into_owned());
    note("tray", json!({ "size": size, "light": light, "icon": number }));
    if !file.exists() {
        let _ = std::fs::write(&file, diag::png(size, size, rgba));
    }
}

// ===== What the guide asks =====

/// What the guide needs to know: where it is, and the facts that decide
/// which steps this PC can do.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub on: bool,
    pub step: Option<String>,
    pub done: Vec<String>,
    /// The app started again after the step on screen began: a sign-out
    /// and in, or a quit and start.
    pub restarted: bool,
    /// The zip last written: its name and where it lies.
    pub zip: Option<ZipView>,
    /// `claude.exe`, `claude.cmd`, or none found.
    pub claude_program: Option<String>,
    /// This copy of the app, for a command the owner pastes in a terminal.
    pub exe: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipView {
    pub name: String,
    /// The folder it is in, as File Explorer names it.
    pub folder: String,
}

fn zip_view(path: &Path) -> Option<ZipView> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let folder = path.parent()?.file_name()?.to_string_lossy().into_owned();
    Some(ZipView { name, folder })
}

#[tauri::command]
pub fn session_get(state: State<'_, AppState>) -> SessionView {
    let claude_program = state
        .lock()
        .claude_program
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let guard = live();
    let Some(live) = guard.as_ref() else {
        return SessionView {
            on: false,
            step: None,
            done: Vec::new(),
            restarted: false,
            zip: None,
            claude_program,
            exe: None,
        };
    };
    let p = &live.progress;
    SessionView {
        on: true,
        step: p.step.clone(),
        done: p.done.clone(),
        restarted: p.step.is_some() && live.started > p.step_at,
        zip: p.zip.as_deref().and_then(zip_view),
        claude_program,
        exe: std::env::current_exe().ok().map(|p| p.display().to_string()),
    }
}

/// The guide shows step `id`: events from now on are its own, and the
/// pictures it wants are taken as the windows show.
#[tauri::command]
pub fn session_step(state: State<'_, AppState>, id: String, captures: Vec<String>) {
    let home = state.lock().home.clone();
    {
        let mut guard = live();
        let Some(live) = guard.as_mut() else { return };
        if live.progress.step.as_deref() != Some(&id) {
            live.progress.step = Some(id.clone());
            live.progress.step_at = now();
            live.login_at_step = modified(&login_file(&home));
        }
        live.captures = captures;
        live.shots = 0;
        live.progress.save(&live.dir);
    }
    note("step", json!({ "id": id }));
}

/// The owner finished step `id` (or the guide skipped it): its self-checks
/// run now, and the step, the facts the guide read from the model and the
/// checks' results go to `steps.jsonl`.
#[tauri::command]
pub fn session_done(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    skipped: bool,
    facts: Value,
    checks: Vec<String>,
) {
    let results: serde_json::Map<String, Value> =
        checks.iter().map(|c| (c.clone(), check(&app, &state, c))).collect();
    let mut guard = live();
    let Some(live) = guard.as_mut() else { return };
    let line = json!({
        "id": id,
        "skipped": skipped,
        "startedAt": live.progress.step_at,
        "endedAt": now(),
        "facts": facts,
        "checks": results,
    });
    if let Ok(mut file) =
        std::fs::OpenOptions::new().create(true).append(true).open(live.dir.join("steps.jsonl"))
    {
        let _ = writeln!(file, "{line}");
    }
    if !live.progress.done.contains(&id) {
        live.progress.done.push(id);
    }
    live.progress.save(&live.dir);
}

/// "Chụp sau 3 giây": a picture of `what` (`tray`, `main`, `popover`,
/// `toast`) a moment later, once the owner has hovered or opened it.
#[tauri::command]
pub fn session_shot(app: AppHandle, what: String, delay_ms: u64) {
    if matches!(what.as_str(), "tray" | "main" | "popover" | "toast") {
        take(&app, &what, Duration::from_millis(delay_ms.min(10_000)));
    }
}

/// What a page saw itself: its fonts, a contrast theme, a reload.
#[tauri::command]
pub fn session_note(kind: String, data: Value) {
    let kind: String =
        kind.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(32).collect();
    note(&format!("page-{kind}"), data);
}

/// A Windows Settings page or a file the guide points the owner to.
#[tauri::command]
pub fn session_open(
    app: AppHandle,
    state: State<'_, AppState>,
    what: String,
) -> Result<(), String> {
    let page = match what.as_str() {
        "colors" => "ms-settings:colors",
        "display" => "ms-settings:display",
        "taskbar" => "ms-settings:taskbar",
        "contrast" => "ms-settings:easeofaccess-highcontrast",
        "startup" => "ms-settings:startupapps",
        "notifications" => "ms-settings:notifications",
        "codexConfig" => {
            let file = {
                let inner = state.lock();
                quota_station_core::codex::home(&inner.home, inner.settings.codex_home.as_deref())
                    .join("config.toml")
            };
            note("open", json!({ "what": what }));
            return open::text(&app, &file);
        }
        _ => return Err("unknown".into()),
    };
    note("open", json!({ "what": what }));
    open::url(&app, page)
}

/// "Ghi gói chẩn đoán": `diag-<date>.zip` on the desktop (else in
/// Downloads, else in the app's folder), over one written earlier today.
#[tauri::command]
pub fn session_zip(app: AppHandle, state: State<'_, AppState>) -> Result<ZipView, String> {
    let dir = live().as_ref().map(|l| l.dir.clone()).ok_or("no session")?;
    note("zip", json!({}));
    let utc = OffsetDateTime::now_utc();
    let local = utc.to_offset(quota_station_core::local_time::offset_at(utc));
    let at = time::PrimitiveDateTime::new(local.date(), local.time());
    let (bytes, app_dir) = {
        let inner = state.lock();
        let home = inner.home.display().to_string();
        let app_dir = inner.app_dir().path().to_path_buf();
        let local_dir = inner.app_dir().local().path().to_path_buf();
        let bytes = diag::bundle(&diag::Sources {
            home: &home,
            at,
            diagnostics: release::diagnostics(&inner),
            settings: std::fs::read_to_string(&inner.settings_file).ok(),
            logs: diag::logs(&[app_dir.as_path(), local_dir.as_path()]),
            session: Some(&dir),
        });
        (bytes, app_dir)
    };
    let name = format!("diag-{}.zip", local.date());
    let path = [app.path().desktop_dir().ok(), app.path().download_dir().ok(), Some(app_dir)]
        .into_iter()
        .flatten()
        .map(|d| d.join(&name))
        .find(|p| std::fs::write(p, &bytes).is_ok())
        .ok_or("the zip could not be written")?;
    let view = zip_view(&path).ok_or("no name")?;
    if let Some(live) = live().as_mut() {
        live.progress.zip = Some(path);
        live.progress.save(&live.dir);
    }
    Ok(view)
}

/// Shows the zip in File Explorer, selected, ready to drag.
#[tauri::command]
pub fn session_reveal(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = live().as_ref().and_then(|l| l.progress.zip.clone()).ok_or("no zip")?;
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

// ===== Self-checks =====

/// One self-check, read from the system and the app's folders. None reads
/// a secret: files are counted, their times and first bytes compared, and
/// Credential Manager's items only looked up by name.
fn check(app: &AppHandle, state: &AppState, id: &str) -> Value {
    let inner = state.lock();
    match id {
        "system" => system(&inner),
        "machine" => json!({
            "set": !inner.system_machine.is_empty(),
            "fromWindows": inner.system_machine == crate::machine_name(),
            "own": inner.settings.machine_name.is_some(),
        }),
        "claudeStore" => claude_store(&inner),
        "claudeSessions" => json!({
            "folders": count(&inner.app_dir().path().join("claude").join("sessions"), ".credentials.json"),
        }),
        "rotation" => rotation(inner.app_dir().path()),
        "port1455" => json!({
            "free": std::net::TcpListener::bind(("127.0.0.1", 1455)).is_ok(),
        }),
        "codexConfig" => {
            let home =
                quota_station_core::codex::home(&inner.home, inner.settings.codex_home.as_deref());
            json!({ "store": codex_store(&home.join("config.toml")) })
        }
        "codexAccounts" => {
            let local = inner.app_dir().local().path().join("codex").join("accounts");
            let roaming = inner.app_dir().path().join("codex").join("accounts");
            json!({
                "local": count(&local, "auth.json"),
                "roaming": if local == roaming { 0 } else { count(&roaming, "auth.json") },
                "ownerOnly": owner_only(&local),
            })
        }
        "secrets" => secrets(),
        "mainWindow" => {
            drop(inner);
            main_window(app)
        }
        other => platform_check(app, &inner, other),
    }
}

/// The diagnostic info, and whether its system line and paths are right.
fn system(inner: &Inner) -> Value {
    let text = release::diagnostics(inner);
    let second = text.lines().nth(1).unwrap_or_default();
    json!({
        "diagnostics": text,
        "systemLine": second.split(" · ").next(),
        "windowsLine": second.starts_with("Windows"),
        "noProfilePath": !text.to_ascii_lowercase().contains(r"\users\"),
        "packaged": crate::notify::bundled(),
        "webview": tauri::webview_version().ok(),
        "startedAgain": live().as_ref().map(|l| l.progress.starts.len()),
    })
}

/// Files named `name` one folder below `dir` (`sessions/<n>/…`).
fn count(dir: &Path, name: &str) -> usize {
    std::fs::read_dir(dir)
        .map(|r| r.flatten().filter(|e| e.path().join(name).is_file()).count())
        .unwrap_or(0)
}

/// Claude Code's login: in its file or in Credential Manager, written
/// during the step or not, and no lock of its left behind.
fn claude_store(inner: &Inner) -> Value {
    let claude = inner.home.join(".claude");
    let file = login_file(&inner.home);
    let at_step = live().as_ref().and_then(|l| l.login_at_step);
    let now = modified(&file);
    json!({
        "file": file.is_file(),
        "fileWritten": now.is_some() && now != at_step,
        "credentialManager": credman(),
        "storageLockLeft": claude.join(".storage-write.lock").exists(),
        "refreshLockLeft": claude.join(".oauth_refresh.lock").exists(),
        "kept": inner.claude_accounts.len(),
    })
}

/// The rotation's timer flag and its log: lines counted, the last one's
/// time, never their text.
fn rotation(app_dir: &Path) -> Value {
    let claude = app_dir.join("claude");
    let log = std::fs::read_to_string(claude.join("rotation.log")).unwrap_or_default();
    let last = log.lines().last().and_then(|l| l.split_whitespace().next());
    let last = last.filter(|w| w.len() <= 32 && w.starts_with(|c: char| c.is_ascii_digit()));
    json!({
        "timerOn": claude.join("rotation-timer").exists(),
        "logLines": log.lines().count(),
        "lastLine": last,
    })
}

/// The store `cli_auth_credentials_store` names in Codex's config: one of
/// the words Codex knows, else `other`; `None` when the line is not there.
fn codex_store(config: &Path) -> Option<&'static str> {
    let text = std::fs::read_to_string(config).ok()?;
    let line = text.lines().find(|l| l.trim_start().starts_with("cli_auth_credentials_store"))?;
    let value = line.split('=').nth(1).unwrap_or_default().trim().trim_matches(['"', '\'']);
    Some(match value {
        "file" => "file",
        "keyring" => "keyring",
        "auto" => "auto",
        _ => "other",
    })
}

/// The app's sealed secrets: how many, whether each starts as a DPAPI blob
/// does (so none holds its text), and whether the folder is its user's alone.
#[cfg(windows)]
fn secrets() -> Value {
    /// `dwVersion` 1 and DPAPI's provider GUID, at the start of every blob.
    const DPAPI: [u8; 20] = [
        0x01, 0x00, 0x00, 0x00, 0xd0, 0x8c, 0x9d, 0xdf, 0x01, 0x15, 0xd1, 0x11, 0x8c, 0x7a, 0x00,
        0xc0, 0x4f, 0xc2, 0x97, 0xeb,
    ];
    let dir = quota_station_core::win::secrets_dir();
    let mut files = 0;
    let mut sealed = 0;
    for service in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        for item in std::fs::read_dir(service.path()).into_iter().flatten().flatten() {
            if item.path().extension().is_some_and(|e| e == "bin") {
                files += 1;
                let head = std::fs::read(item.path()).unwrap_or_default();
                if head.starts_with(&DPAPI) {
                    sealed += 1;
                }
            }
        }
    }
    json!({ "files": files, "sealed": sealed, "ownerOnly": owner_only(&dir) })
}

#[cfg(not(windows))]
fn secrets() -> Value {
    json!({ "files": Value::Null })
}

#[cfg(windows)]
fn owner_only(dir: &Path) -> Option<bool> {
    quota_station_core::win::is_owner_only(dir).ok()
}

#[cfg(not(windows))]
fn owner_only(_: &Path) -> Option<bool> {
    None
}

/// Claude Code's item in Credential Manager, and whether it is in pieces.
#[cfg(windows)]
fn credman() -> Value {
    use quota_station_core::claude::credman;
    use quota_station_core::os::Credentials;
    let creds = quota_station_core::win::WinCred;
    let target = credman::target("Claude Code-credentials", "claude-code-user");
    let there = |t: &str| creds.read(t).map(|v| v.is_some()).unwrap_or(false);
    json!({ "item": there(&target), "pieces": there(&format!("{target}#m")) })
}

#[cfg(not(windows))]
fn credman() -> Value {
    Value::Null
}

/// The main window: there, framed by Windows, its size, and on screen.
fn main_window(app: &AppHandle) -> Value {
    let Some(window) = app.get_webview_window(windows::MAIN) else {
        return json!({ "open": false });
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size().ok().map(|s| s.to_logical::<f64>(scale));
    let at = window.outer_position().ok();
    let outer = window.outer_size().ok();
    let inside = match (at, outer, window.current_monitor().ok().flatten()) {
        (Some(at), Some(outer), Some(m)) => {
            let work = m.work_area();
            let (right, bottom) = (at.x + outer.width as i32, at.y + outer.height as i32);
            // Windows draws an invisible resize border of a few pixels outside.
            let slack = (8.0 * scale) as i32;
            Some(
                at.x >= work.position.x - slack
                    && at.y >= work.position.y - slack
                    && right <= work.position.x + work.size.width as i32 + slack
                    && bottom <= work.position.y + work.size.height as i32 + slack,
            )
        }
        _ => None,
    };
    json!({
        "open": true,
        "visible": window.is_visible().ok(),
        "decorated": window.is_decorated().ok(),
        "maximized": window.is_maximized().ok(),
        "size": size.map(|s| [s.width.round(), s.height.round()]),
        "scale": scale,
        "inside": inside,
    })
}

/// The checks only Windows answers: the tray as drawn, the material,
/// launching at login.
#[cfg(windows)]
fn platform_check(_app: &AppHandle, _inner: &Inner, id: &str) -> Value {
    match id {
        "tray" => json!({
            "size": crate::winsys::tray_icon_size(),
            "lightTaskbar": crate::winsys::light_taskbar(),
        }),
        "material" => json!({
            "material": crate::winsys::material(),
            "highContrast": crate::winsys::high_contrast(),
        }),
        "autostart" => crate::login_item::session_check(),
        _ => json!({ "unknown": id }),
    }
}

#[cfg(not(windows))]
fn platform_check(_app: &AppHandle, _inner: &Inner, id: &str) -> Value {
    json!({ "unknown": id, "why": "not Windows" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_test_build_has_a_session() {
        // CI's macOS job and every other build leave QS_TEST_SESSION unset.
        assert_eq!(ON, option_env!("QS_TEST_SESSION").is_some());
        if !ON {
            note("nothing", json!({}));
            assert!(live().is_none());
        }
    }

    #[test]
    fn codex_store_reads_only_the_word() {
        let dir = std::env::temp_dir().join(format!("qs-session-codex-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("config.toml");
        assert_eq!(codex_store(&file), None);
        std::fs::write(&file, "model = \"x\"\ncli_auth_credentials_store = \"keyring\"\n").unwrap();
        assert_eq!(codex_store(&file), Some("keyring"));
        std::fs::write(&file, "cli_auth_credentials_store = \"sk-ant-oat01-secret\"\n").unwrap();
        assert_eq!(codex_store(&file), Some("other"));
        std::fs::write(&file, "model = \"x\"\n").unwrap();
        assert_eq!(codex_store(&file), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_counts_lines_and_keeps_only_the_last_time() {
        let dir = std::env::temp_dir().join(format!("qs-session-rot-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("claude")).unwrap();
        std::fs::write(
            dir.join("claude").join("rotation.log"),
            "2026-10-28T09:00:00Z kept lan@example.com\n2026-10-28T09:03:00Z switched to 2\n",
        )
        .unwrap();
        let v = rotation(&dir);
        assert_eq!(v["logLines"], 2);
        assert_eq!(v["lastLine"], "2026-10-28T09:03:00Z");
        assert_eq!(v["timerOn"], false);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
