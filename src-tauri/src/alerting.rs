// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The history and the alerts (M6), after every read: the history keeps a
//! change of the account in use that nothing explains, and the alert rules
//! run on the newest readings and each new alert goes out as a macOS card
//! with buttons. The extension's own alerts run with the app's, from what it
//! reads for them first ([`crate::ext::Extension::alert_read`]), and alert
//! 1's check counts the Claude pool by the extension's policy, read for
//! the pass ([`Hooks::claude_pool`]).
//! The buttons, on the macOS card or the same card in the bell's panel
//! (Thông báo), come back through [`press`].
//!
//! Reads only: the app's own files and, while an overwrite waits for an
//! answer, whether Antigravity.app runs (`pgrep`).

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
use quota_station_core::claude::Paths;
use quota_station_core::history::{self, Entry};
use quota_station_core::local_time;
use quota_station_core::settings::PoolAlert;
use quota_station_core::snapshot::SectionStatus;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use time::OffsetDateTime;

#[cfg(feature = "antigravity-accounts")]
use crate::agy_switch;
use crate::ext::{AlertInput, Hooks};
use crate::notify::{self, Permission};
use crate::state::{AppState, Inner, Model, Tab};
use crate::{open, reader, windows};

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

pub fn alerts_view(inner: &Inner) -> AlertsView {
    AlertsView {
        recent: inner.alerts.recent.clone(),
        switches: inner.settings.alerts.shown(inner.hooks),
        pool_alert: inner.settings.pool_alert,
        unanswered: inner.alerts.unanswered(),
        unread: inner.alerts.unread(),
        permission: inner.center.as_ref().map_or(Permission::Unbundled, |c| c.permission()),
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
    let (dir, mut seen, claude, agy) = {
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
        (dir, inner.seen.clone(), claude, agy)
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    history::track(&dir, &mut seen, claude, agy.as_deref(), now);
    state.lock().seen = seen;
}

/// The alert rules on the newest readings; new alerts go out on the
/// notification center.
fn evaluate(state: &AppState) {
    run_rules(state, |inner, ids| {
        if let Some(center) = inner.center.clone() {
            alerts::deliver(&mut inner.alerts, ids, &*center, inner.hooks);
        }
    });
}

/// The alert rules on the newest readings, the new cards handed to
/// `deliver` under the app's lock. The extension reads what its own alerts
/// need and its policy of the Claude pool first, without the app's lock.
/// Apart from [`evaluate`] so a test
/// runs the rules without the notification center, which holds the window
/// runtime a Windows test program cannot load.
fn run_rules(state: &AppState, deliver: impl FnOnce(&mut Inner, &[String])) {
    let now = OffsetDateTime::now_utc();
    let (dir, claude, settings, home, extension, hooks) = {
        let inner = state.lock();
        (
            inner.app_dir(),
            inner.claude.clone(),
            inner.settings.clone(),
            inner.home.clone(),
            inner.extension,
            inner.hooks,
        )
    };
    let pool = settings.pool_alert;
    let read =
        extension.alert_read_part(&AlertInput { settings: &settings, home: &home, app_dir: &dir });
    let pool_policy = hooks.claude_pool(&Paths::new(&home, dir.path()));
    #[cfg(feature = "antigravity-accounts")]
    let pending = watch::load(&dir).and_then(|w| w.alert()).is_some();
    #[cfg(feature = "antigravity-accounts")]
    let running = if pending { Desktop::system().running().ok() } else { None };
    let facts = Facts {
        now,
        offset: local_time::offset_at(now),
        claude: claude.as_ref(),
        pool_policy: pool_policy.as_deref(),
        #[cfg(feature = "antigravity-accounts")]
        agy: alerts::agy_logins(&dir),
        #[cfg(feature = "antigravity-accounts")]
        overwrite: alerts::overwrite(&dir, running),
        ext: read.as_deref().map(|r| r.source()),
        pool,
    };
    let mut inner = state.lock();
    let before = inner.alerts.clone();
    let switches = inner.settings.alert_switches(hooks);
    let ids = alerts::evaluate(&mut inner.alerts, &facts, &switches, hooks);
    deliver(&mut inner, &ids);
    if inner.alerts != before {
        let _ = alerts::save(&dir, &inner.alerts);
    }
    if let Some(read) = read {
        read.done(&inner.kept);
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
    crate::session::note("toast-press", serde_json::json!({ "action": action }));
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
        let hooks: &dyn Hooks = inner.hooks;
        let effect = match center {
            Some(c) => alerts::answer_in_app(&mut inner.alerts, id, action, at, offset, &*c, hooks),
            None => alerts::answer(&mut inner.alerts, id, action, at, offset, hooks),
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
        Effect::OpenCliTools => windows::open_main(app, Tab::CliTools),
        Effect::IgnoreEnv { name } => crate::env::ignore_name(app, &shared, &name)?,
        #[cfg(feature = "antigravity-accounts")]
        Effect::OpenVerification { number } => {
            let id = account_id(&dir, number).ok_or("notFound")?;
            crate::antigravity::open_verification(app, &dir, &id)?;
        }
        // Only the Antigravity cards offer these, and a build without the
        // Antigravity accounts sends none.
        #[cfg(not(feature = "antigravity-accounts"))]
        Effect::DismissOverwrite | Effect::OpenVerification { .. } => {
            let _ = &dir;
        }
        #[cfg(feature = "antigravity-accounts")]
        Effect::SwitchAgy { number, how } => {
            let refused = |why: &str| alerts::agy_refused(number, why);
            let said = match account_id(&dir, number) {
                None => refused(refusal("unknownAccount")),
                Some(account) => {
                    match agy_switch::switch(app, shared.clone(), account, how).await {
                        Ok(outcome) => alerts::agy_result(&outcome),
                        Err(e) => refused(refusal(&e)),
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

/// A kind of alert this build does not have, refused.
fn unknown_kind(kind: &Kind) -> String {
    format!("unknown alert kind `{}`", kind.name())
}

/// "Gửi thử": a sample card of `kind` with its real buttons, which change
/// nothing. A kind this build does not have sends nothing and is refused.
#[tauri::command]
pub fn test_alert(app: AppHandle, state: State<'_, AppState>, kind: Kind) -> Result<Model, String> {
    let now = OffsetDateTime::now_utc();
    {
        let mut inner = state.lock();
        let hooks: &dyn Hooks = inner.hooks;
        let (at, offset) = (now.unix_timestamp(), local_time::offset_at(now));
        let alert = alerts::sample(&kind, at, offset, hooks);
        let alert = alert.ok_or_else(|| unknown_kind(&kind))?;
        let id = alerts::keep_test(&mut inner.alerts, alert);
        if let Some(center) = inner.center.clone() {
            alerts::deliver(&mut inner.alerts, &[id], &*center, hooks);
        }
        let _ = alerts::save(&inner.app_dir(), &inner.alerts);
    }
    reader::publish(&app, &state);
    crate::session::note("toast-sent", serde_json::json!({ "kind": kind }));
    crate::session::shoot(&app, "toast", Duration::from_millis(1800));
    Ok(state.lock().model())
}

/// Turns one kind of alert on or off. A kind this build does not have is
/// refused.
#[tauri::command]
pub fn set_alert_switch(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: Kind,
    on: bool,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let hooks: &dyn Hooks = inner.hooks;
        if !kind.known(hooks) {
            return Err(unknown_kind(&kind));
        }
        let mut settings = inner.settings.clone();
        settings.alerts.set(&kind, on, hooks);
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

/// The History tab's lines, newest first.
#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<Entry> {
    let dir = state.lock().app_dir();
    history::collect(&dir, history::SHOWN)
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
        let permission = center.permission();
        crate::session::note("notifications", serde_json::json!({ "permission": permission }));
        reader::publish(&app, &shared);
    });
}

/// System Settings › Notifications, where the owner turns the cards on
/// (Windows: Settings › System › Notifications).
#[tauri::command]
pub fn open_notification_settings(app: AppHandle) -> Result<(), String> {
    #[cfg(not(windows))]
    let page = "x-apple.systempreferences:com.apple.Notifications-Settings.extension";
    #[cfg(windows)]
    let page = "ms-settings:notifications";
    open::url(&app, page)
}

/// Starts the macOS notification center (Windows: the toasts), with
/// presses coming to [`on_card`].
pub fn start_center(app: &AppHandle) -> notify::Center {
    let handle = app.clone();
    let refused = app.clone();
    notify::Center::start(
        app,
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Mutex;

    use quota_station_core::alerts::{act, Alert, ExtKind, ExtPass, ExtSource, Tone};
    use quota_station_core::quota::claude::PoolPolicy;
    use quota_station_core::quota::{ClaudeAccounts, Response, Slot};

    use super::*;
    use crate::ext::{AlertRead, Extension, Kept};
    use crate::state::Shared;

    /// One kind of alert, `testKind`, on by default.
    struct KindHooks;

    impl Hooks for KindHooks {
        fn alert_kinds(&self) -> &'static [ExtKind] {
            &[ExtKind { number: 2, name: "testKind", on: true, answers: &[] }]
        }
    }

    /// Reads the machine's name for its alerts: one `testKind` card naming
    /// it, and the name had back once the rules ran.
    struct Reads;

    impl Extension for Reads {
        fn alert_read(&self, input: &AlertInput<'_>) -> Option<Box<dyn AlertRead>> {
            Some(Box::new(Read(input.settings.machine_name.clone().unwrap_or_default())))
        }
    }

    struct Read(String);

    impl ExtSource for Read {
        fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
            vec![Alert {
                key: format!("test:{}", self.0),
                kind: Kind::Ext("testKind".into()),
                tone: Tone::Info,
                title: self.0.clone(),
                body: String::new(),
                actions: vec![act("dismiss", "Fine")],
                at: pass.facts.now.unix_timestamp(),
                test: false,
            }]
        }
    }

    #[derive(Default)]
    struct Had(Mutex<Vec<String>>);

    impl AlertRead for Read {
        fn source(&self) -> &dyn ExtSource {
            self
        }

        fn done(self: Box<Self>, kept: &Kept) {
            kept.get::<Had>().expect("kept").0.lock().unwrap().push(self.0);
        }
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qs-alerting-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn state(dir: &std::path::Path, hooks: &'static dyn Hooks) -> AppState {
        let mut inner = Inner::new(dir.into(), dir.join("settings.json"), "Mac".into(), hooks);
        inner.settings.machine_name = Some("Mac A".into());
        inner.kept.keep(Arc::new(Had::default()));
        Shared::new(inner)
    }

    /// After a read, the extension reads what its alerts need, its card
    /// goes out with the app's rules (once), is kept in the alerts' file,
    /// and what it read is its again.
    #[test]
    fn after_a_read_the_extensions_alerts_go_out_from_what_it_read() {
        let dir = temp("ext");
        let state = state(&dir, &KindHooks);
        state.lock().extension = &Reads;
        run_rules(&state, |_, _| {});
        run_rules(&state, |_, _| {});
        let inner = state.lock();
        assert_eq!(inner.alerts.recent.len(), 1, "once");
        let alert = &inner.alerts.recent[0].alert;
        assert_eq!(
            (alert.kind.clone(), alert.title.as_str()),
            (Kind::Ext("testKind".into()), "Mac A")
        );
        assert_eq!(alerts::load(&inner.app_dir(), &KindHooks), inner.alerts);
        let had = inner.kept.get::<Had>().unwrap();
        assert_eq!(*had.0.lock().unwrap(), ["Mac A", "Mac A"]);
        drop(inner);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Without an extension the rules run as the app's own: nothing read,
    /// no card of a kind it does not have.
    #[test]
    fn without_an_extension_the_rules_are_the_apps_own() {
        let dir = temp("none");
        let state = state(&dir, quota_station_core::ext::none());
        run_rules(&state, |_, _| {});
        let inner = state.lock();
        assert!(inner.alerts.recent.is_empty());
        assert!(inner.kept.get::<Had>().unwrap().0.lock().unwrap().is_empty());
        drop(inner);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A pool policy that counts the account in use only.
    struct InUseOnly;

    impl PoolPolicy for InUseOnly {
        fn counts(
            &self,
            accounts: &ClaudeAccounts,
            slot: &Slot,
            _now: OffsetDateTime,
            _offset: time::UtcOffset,
        ) -> bool {
            accounts.active_slot == Some(slot.slot)
        }
    }

    /// Hooks whose policy of the Claude pool is [`InUseOnly`], read where
    /// the app keeps the accounts.
    struct InUseHooks;

    impl Hooks for InUseHooks {
        fn claude_pool(&self, paths: &Paths) -> Option<Box<dyn PoolPolicy>> {
            assert!(paths.dir.ends_with("claude"), "{}", paths.dir.display());
            Some(Box::new(InUseOnly))
        }
    }

    /// The account in use, 2, nearly out; 3 with room.
    fn nearly_out() -> Response {
        serde_json::from_value(serde_json::json!({
            "generatedAt": "2026-01-16T07:20:00.000Z",
            "providers": [{
                "provider": "claude", "windows": [], "state": { "status": "fresh" },
                "claudeAccounts": { "activeSlot": 2, "slots": [
                    { "slot": 2, "active": true, "status": "fresh", "effectivePercentRemaining": 5.0 },
                    { "slot": 3, "active": false, "status": "fresh", "effectivePercentRemaining": 90.0 }
                ] }
            }]
        }))
        .unwrap()
    }

    /// Alert 1's check at each pass counts the Claude pool by the policy
    /// the extension's hooks read for that pass: counted alone, the account
    /// in use is dry; with every account counted, it is not.
    #[test]
    fn alert_one_counts_the_pool_by_the_extensions_policy() {
        let dir = temp("pool");
        let counted = state(&dir, &InUseHooks);
        counted.lock().claude = Some(nearly_out());
        run_rules(&counted, |_, _| {});
        let inner = counted.lock();
        let kinds: Vec<&Kind> = inner.alerts.recent.iter().map(|r| &r.alert.kind).collect();
        assert_eq!(kinds, [&Kind::Pool]);
        assert!(inner.alerts.recent[0].alert.key.starts_with("pool:dry:"));
        drop(inner);
        let _ = std::fs::remove_dir_all(&dir);

        let dir = temp("pool-none");
        let every = state(&dir, quota_station_core::ext::none());
        every.lock().claude = Some(nearly_out());
        run_rules(&every, |_, _| {});
        assert!(every.lock().alerts.recent.is_empty(), "3 has room");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
