// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The history, the five alerts and what Firstmate sees (M6), after every
//! read: the history keeps a change of the account in use that nothing
//! explains, the alert rules run on the newest readings and each new alert
//! goes out as a macOS card with buttons, and Firstmate's state file is read
//! again. The buttons, on the macOS card or the same card in the bell's
//! panel (Thông báo), come back through [`press`].
//!
//! Reads only: the app's own files, the switcher's log, the rotation file,
//! Firstmate's `quota-pool.last` at the path set in settings, and, while an
//! overwrite waits for an answer, whether Antigravity.app runs (`pgrep`).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use quota_station_core::alerts::{self, Effect, Facts, Kind, Record, Switches};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::desktop::Desktop;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::switch::{self as agy};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::{index, watch};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::app_dir::AppDir;
#[cfg(feature = "author-tools")]
use quota_station_core::firstmate::{self, PoolLine};
use quota_station_core::history::{self, Entry};
use quota_station_core::rotation::{self, FileState};
#[cfg(feature = "author-tools")]
use quota_station_core::runner;
use quota_station_core::settings::PoolAlert;
use quota_station_core::snapshot::SectionStatus;
use quota_station_core::{local_time, switcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(feature = "author-tools")]
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[cfg(feature = "antigravity-accounts")]
use crate::agy_switch;
use crate::notify::{self, Permission};
use crate::state::{AppState, Inner, Model, Tab};
use crate::{reader, windows};

/// The bell's panel (Thông báo) and its count.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertsView {
    /// Newest first.
    pub recent: Vec<Record>,
    pub switches: Switches,
    /// Alert 1's thresholds.
    pub pool_alert: PoolAlert,
    /// Cards with buttons nobody pressed yet.
    pub unanswered: usize,
    /// Cards that came since the panel was last opened: the bell's count.
    pub unread: usize,
    pub permission: Permission,
}

/// The Firstmate tab (F10).
#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstmateView {
    /// The state file, `~/…`; `None` when not set.
    pub file: Option<String>,
    /// `notSet`, `ok`, `missing`, `notPlain`, `notPoolFile` or `unreadable`.
    pub status: &'static str,
    pub line: Option<PoolLine>,
    /// When Firstmate last wrote it, RFC 3339.
    pub written_at: Option<String>,
    /// Written recently enough to act on.
    pub fresh: bool,
    /// The same dry rule, counted by the app from its own numbers.
    pub own_dry: Option<bool>,
}

#[cfg(feature = "author-tools")]
fn rfc(at: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(at).ok().and_then(|t| t.format(&Rfc3339).ok())
}

pub fn alerts_view(inner: &Inner) -> AlertsView {
    AlertsView {
        recent: inner.alerts.recent.clone(),
        switches: inner.settings.alerts,
        pool_alert: inner.settings.pool_alert,
        unanswered: inner.alerts.unanswered(),
        unread: inner.alerts.unread(),
        permission: inner.center.as_ref().map_or(Permission::Unbundled, |c| c.permission()),
    }
}

#[cfg(feature = "author-tools")]
pub fn firstmate_view(inner: &Inner) -> FirstmateView {
    let now = OffsetDateTime::now_utc();
    let file = inner.settings.firstmate_file.as_ref();
    let (status, reading) = match (file, inner.firstmate.as_ref()) {
        (None, _) => ("notSet", None),
        (Some(_), None) => ("unreadable", None),
        (Some(_), Some(Ok(r))) => ("ok", Some(r)),
        (Some(_), Some(Err(e))) => (e.kind(), None),
    };
    FirstmateView {
        file: file.map(|p| runner::display_path(p, &inner.home)),
        status,
        line: reading.map(|r| r.line.clone()),
        written_at: reading.and_then(|r| rfc(r.modified_at)),
        fresh: reading.is_some_and(|r| r.fresh(now.unix_timestamp())),
        own_dry: inner.own_dry,
    }
}

/// After a read: the history, then the alerts.
pub fn after_read(state: &AppState) {
    track_history(state);
    evaluate(state);
}

/// Keeps in the history what the newest readings show changed.
pub fn track_history(state: &AppState) {
    let _one = state.history.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, log, mut seen, claude, agy) = {
        let inner = state.lock();
        // A switch keeps its own line when it ends, then looks again; a
        // reading taken meanwhile is not compared.
        if inner.switching.is_some() || inner.agy_switching() {
            return;
        }
        let snapshot = inner.snapshot();
        let claude = (snapshot.claude.status == SectionStatus::Ok)
            .then(|| snapshot.claude.active().map(|a| a.number))
            .flatten()
            .filter(|n| *n > 0);
        let dir = inner.app_dir();
        // Only the accounts the app keeps have an id to follow.
        #[cfg(feature = "antigravity-accounts")]
        let agy = inner
            .agy
            .as_ref()
            .and_then(|r| index::load(&dir).ok().and_then(|idx| agy::shown_active_id(r, &idx)));
        #[cfg(not(feature = "antigravity-accounts"))]
        let agy: Option<String> = None;
        (dir, switcher::log_file(&inner.home), inner.seen.clone(), claude, agy)
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    history::track(&dir, &log, &mut seen, claude, agy.as_deref(), now);
    state.lock().seen = seen;
}

/// The rules of the five alerts on the newest readings; new alerts go out.
fn evaluate(state: &AppState) {
    let now = OffsetDateTime::now_utc();
    let (dir, home, claude, pool, advanced) = {
        let inner = state.lock();
        (
            inner.app_dir(),
            inner.home.clone(),
            inner.claude.clone(),
            inner.settings.pool_alert,
            inner.settings.advanced_shown(),
        )
    };
    #[cfg(feature = "author-tools")]
    let file = state.lock().settings.firstmate_file.clone();
    #[cfg(feature = "author-tools")]
    let reading = file.as_deref().map(firstmate::read);
    let has_rotation = claude
        .as_ref()
        .and_then(|r| r.primary("claude"))
        .and_then(|r| r.claude_accounts.as_ref())
        .is_some_and(|a| a.rotation.is_some());
    let rules = match has_rotation {
        true => None,
        false => match rotation::load(
            &quota_station_core::claude::Paths::new(&home, dir.path()).rotation_file(),
        ) {
            FileState::Ok { parsed, .. } => Some(parsed.rules),
            _ => None,
        },
    };
    #[cfg(feature = "antigravity-accounts")]
    let pending = watch::load(&dir).and_then(|w| w.alert()).is_some();
    #[cfg(feature = "antigravity-accounts")]
    let running = if pending { Desktop::system().running().ok() } else { None };
    let facts = Facts {
        now,
        offset: local_time::offset_at(now),
        claude: claude.as_ref(),
        rules: rules.as_ref(),
        #[cfg(feature = "antigravity-accounts")]
        agy: alerts::agy_logins(&dir),
        #[cfg(feature = "antigravity-accounts")]
        overwrite: alerts::overwrite(&dir, running),
        #[cfg(feature = "author-tools")]
        firstmate: reading.as_ref().and_then(|r| r.as_ref().ok()),
        pool,
        advanced,
    };
    let mut inner = state.lock();
    let before = inner.alerts.clone();
    let switches = inner.settings.alert_switches();
    let ids = alerts::evaluate(&mut inner.alerts, &facts, &switches);
    #[cfg(feature = "author-tools")]
    {
        inner.own_dry = claude
            .as_ref()
            .and_then(|r| alerts::pool_check(r, rules.as_ref(), pool, now, facts.offset))
            .map(|c| c.dry);
    }
    if let Some(center) = inner.center.clone() {
        alerts::deliver(&mut inner.alerts, &ids, &*center);
    }
    if inner.alerts != before {
        let _ = alerts::save(&dir, &inner.alerts);
    }
    #[cfg(feature = "author-tools")]
    {
        inner.firstmate = reading;
    }
}

/// What the window that pressed a button does next.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pressed {
    /// Opens its switch sheet for this Claude account.
    pub open_switch: Option<u32>,
}

/// The bell's panel opened, in the main window: every card in it is read.
#[tauri::command]
pub fn alerts_read(app: AppHandle, state: State<'_, AppState>) {
    let shared = state.inner().clone();
    {
        let mut inner = shared.lock();
        if !inner.alerts.mark_read() {
            return;
        }
        let _ = alerts::save(&inner.app_dir(), &inner.alerts);
    }
    reader::publish(&app, &shared);
}

/// A button on a card in the bell's panel.
#[tauri::command]
pub async fn alert_action(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> Result<Pressed, String> {
    press(&app, state.inner().clone(), &id, &action, false).await
}

/// A press on a macOS card: a button, or the card itself (`None`), which
/// opens the main window with the bell's panel open (`Tab::Alerts` asks for
/// the panel, not a page). Comes on the notification center's thread.
pub fn on_card(app: &AppHandle, id: String, action: Option<String>) {
    let Some(action) = action else {
        windows::open_main(app, Tab::Alerts);
        return;
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let shared = app.state::<AppState>().inner().clone();
        let _ = press(&app, shared, &id, &action, true).await;
    });
}

#[cfg(feature = "antigravity-accounts")]
fn refusal(code: &str) -> &'static str {
    match code {
        "busy" => "đang có một lần chuyển khác",
        "loggingOut" => "đang đăng xuất một tài khoản",
        "loggingIn" => "đang đăng nhập một tài khoản",
        "unknownAccount" => "tài khoản không còn trong app",
        _ => "không đọc được danh sách tài khoản",
    }
}

#[cfg(feature = "antigravity-accounts")]
fn account_id(dir: &AppDir, number: u32) -> Option<String> {
    let i = (number as usize).checked_sub(1)?;
    index::load(dir).ok()?.accounts.get(i).map(|a| a.id.clone())
}

/// Notes the answer, then does what the button says. A switch back runs
/// here and its result goes under the card; "Sang …" and "Chuyển ngay" only
/// open the switch sheet, where the owner confirms.
pub async fn press(
    app: &AppHandle,
    shared: AppState,
    id: &str,
    action: &str,
    on_macos: bool,
) -> Result<Pressed, String> {
    let now = OffsetDateTime::now_utc();
    let (at, offset) = (now.unix_timestamp(), local_time::offset_at(now));
    let (effect, dir) = {
        let mut inner = shared.lock();
        let center = inner.center.clone().filter(|_| !on_macos);
        let effect = match center {
            Some(c) => alerts::answer_in_app(&mut inner.alerts, id, action, at, offset, &*c),
            None => alerts::answer(&mut inner.alerts, id, action, at, offset),
        }
        .map_err(|e| e.code().to_string())?;
        let dir = inner.app_dir();
        let _ = alerts::save(&dir, &inner.alerts);
        (effect, dir)
    };
    reader::publish(app, &shared);
    match effect {
        Effect::None => {}
        Effect::OpenClaudeSwitch { slot } if on_macos => {
            windows::show_popover(app);
            let _ = app.emit_to(windows::POPOVER, "openSwitch", slot);
        }
        Effect::OpenClaudeSwitch { slot } => return Ok(Pressed { open_switch: Some(slot) }),
        #[cfg(feature = "antigravity-accounts")]
        Effect::DismissOverwrite => {
            watch::dismiss(&dir).map_err(|e| e.to_string())?;
            reader::publish(app, &shared);
        }
        Effect::OpenAgyTab => windows::open_main(app, Tab::Agy),
        Effect::OpenSkills => windows::open_main(app, Tab::Skills),
        Effect::OpenSettings => windows::open_settings(app, "services"),
        Effect::IgnoreEnv { name } => crate::env::ignore_name(app, &shared, &name)?,
        #[cfg(feature = "antigravity-accounts")]
        Effect::OpenVerification { number } => {
            let id = account_id(&dir, number).ok_or("notFound")?;
            crate::antigravity::open_verification(&dir, &id)?;
        }
        // Only the Antigravity cards offer these, and a build without the
        // Antigravity accounts sends none.
        #[cfg(not(feature = "antigravity-accounts"))]
        Effect::DismissOverwrite | Effect::OpenVerification { .. } => {
            let _ = &dir;
        }
        #[cfg(feature = "antigravity-accounts")]
        Effect::SwitchAgy { number, how } => {
            let roles = shared.lock().settings.advanced_shown();
            let said = match account_id(&dir, number) {
                None => alerts::agy_refused(number, refusal("unknownAccount"), roles),
                Some(account) => {
                    match agy_switch::switch(app, shared.clone(), account, how).await {
                        Ok(outcome) => alerts::agy_result(&outcome, roles),
                        Err(e) => alerts::agy_refused(number, refusal(&e), roles),
                    }
                }
            };
            {
                let mut inner = shared.lock();
                alerts::settle(&mut inner.alerts, id, said);
                let _ = alerts::save(&dir, &inner.alerts);
            }
            reader::publish(app, &shared);
        }
    }
    Ok(Pressed { open_switch: None })
}

/// "Gửi thử": a sample card of `kind` with its real buttons, which change
/// nothing. None for a kind this build does not have ([`Kind::built`]).
#[tauri::command]
pub fn test_alert(app: AppHandle, state: State<'_, AppState>, kind: Kind) -> Model {
    let now = OffsetDateTime::now_utc();
    if kind.built() {
        let mut inner = state.lock();
        let roles = inner.settings.advanced_shown();
        let alert = alerts::sample(kind, now.unix_timestamp(), local_time::offset_at(now), roles);
        let id = alerts::keep_test(&mut inner.alerts, alert);
        if let Some(center) = inner.center.clone() {
            alerts::deliver(&mut inner.alerts, &[id], &*center);
        }
        let _ = alerts::save(&inner.app_dir(), &inner.alerts);
    }
    reader::publish(&app, &state);
    state.lock().model()
}

/// Turns one kind of alert on or off; refuses (`notBuilt`) a kind this
/// build does not have ([`Kind::built`]).
#[tauri::command]
pub fn set_alert_switch(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: Kind,
    on: bool,
) -> Result<Model, String> {
    if !kind.built() {
        return Err("notBuilt".into());
    }
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.alerts.set(kind, on);
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// Sets alert 1's two thresholds, in percent left: refused (`range`,
/// `order`) unless each is 1 to 99 and the room asked of another account is
/// more than what counts as dry.
#[tauri::command]
pub fn set_pool_alert(
    app: AppHandle,
    state: State<'_, AppState>,
    dry_at_most: u8,
    room_at_least: u8,
) -> Result<Model, String> {
    let pool = PoolAlert { dry_at_most, room_at_least };
    pool.check().map_err(str::to_string)?;
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.pool_alert = pool;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// Sets Firstmate's `quota-pool.last`; empty forgets it. Refuses a file not
/// called `quota-pool.last` (`notPoolFile`) and a path with no file
/// (`notFound`), and keeps the one before.
#[cfg(feature = "author-tools")]
#[tauri::command]
pub fn set_firstmate_file(
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
            if !firstmate::is_pool_file(&p) {
                return Err("notPoolFile".into());
            }
            if !p.is_file() {
                return Err("notFound".into());
            }
            Some(p)
        };
        let mut settings = inner.settings.clone();
        settings.firstmate_file = chosen;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.firstmate = settings.firstmate_file.as_deref().map(firstmate::read);
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// The History tab's lines, newest first.
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<Entry> {
    let (dir, home) = {
        let inner = state.lock();
        (inner.app_dir(), inner.home.clone())
    };
    history::collect(&dir, &switcher::log_file(&home), history::SHOWN)
}

/// Asks macOS again whether cards may show, then sends the answer to the
/// windows.
#[tauri::command]
pub fn notify_refresh(app: AppHandle, state: State<'_, AppState>) {
    let Some(center) = state.lock().center.clone() else { return };
    center.refresh();
    let shared: Arc<_> = state.inner().clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        reader::publish(&app, &shared);
    });
}

/// System Settings › Notifications, where the owner turns the cards on.
#[tauri::command]
pub fn open_notification_settings() -> Result<(), String> {
    std::process::Command::new(Path::new("/usr/bin/open"))
        .arg("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Starts the macOS notification center, with presses coming to [`on_card`].
pub fn start_center(app: &AppHandle) -> notify::Center {
    let handle = app.clone();
    let refused = app.clone();
    notify::Center::start(
        Box::new(move |id, action| on_card(&handle, id, action)),
        Arc::new(move |id| on_refused(&refused, &id)),
    )
}

/// macOS did not post a card it was given: the card stays in the app only.
fn on_refused(app: &AppHandle, id: &str) {
    let shared = app.state::<AppState>().inner().clone();
    {
        let mut inner = shared.lock();
        let Some(record) = inner.alerts.recent.iter_mut().find(|r| r.id == id) else { return };
        record.notified = false;
        let dir = inner.app_dir();
        let _ = alerts::save(&dir, &inner.alerts);
    }
    reader::publish(app, &shared);
}
