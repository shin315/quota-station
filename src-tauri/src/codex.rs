// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Switching Codex from the app (M8): a thread that looks at Codex's files,
//! the process table and whether the custom provider answers, the model the
//! windows draw from it, and the commands the Codex row, sheet and tab use.
//!
//! The switch writes `config.toml` only when the owner confirms, and only
//! the keys `quota_station_core::codex::config` names. The windows never get
//! a key: only what kind of key the provider has. A key the owner pastes
//! ("Dán khoá") goes one way, from the window to the core, and comes back
//! only as its hint. The accounts' own commands are in
//! [`crate::codex_accounts`].

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::accounts::{self, Account, Store};
use quota_station_core::codex::config::{self, Current, Custom, Mcp, Refusal, Target};
use quota_station_core::codex::files::{self, Record};
use quota_station_core::codex::login::Login;
use quota_station_core::codex::platform::System;
use quota_station_core::codex::probe::{self, Answer};
use quota_station_core::codex::procs::{self, Procs, Restart};
use quota_station_core::codex::{
    self, log, resume, FileState, KeyOutcome, KeyPreview, Outcome, Reading,
};
use quota_station_core::icon::CodexUse;
use quota_station_core::runner;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::codex_accounts::{self, AccountView};
use crate::reader;
use crate::state::{AppState, Inner, Model};

/// How often the thread looks while a window is open, and while none is.
const VISIBLE_EVERY: Duration = Duration::from_secs(60);
const HIDDEN_EVERY: Duration = Duration::from_secs(5 * 60);
/// The Codex tab shows the last switches.
const RECENT: usize = 5;

/// What the app holds about Codex between looks.
#[derive(Debug, Default)]
pub struct Live {
    pub reading: Option<Reading>,
    pub procs: Option<Procs>,
    /// `codex --version`, once per path found.
    pub version: Option<(PathBuf, Option<String>)>,
    pub reach: Reach,
    /// Resumable sessions by provider, counted when the Codex tab opens.
    pub resume: Option<BTreeMap<String, u32>>,
    /// `switching`, `restarting`, `importing` or `waking` while one runs.
    pub busy: Option<&'static str>,
    /// The provider id the last look saw, to notice a change made outside.
    pub seen: Option<String>,
    /// A look is asked for now; with `resume`, the sessions are counted too.
    pub now: bool,
    pub count_resume: bool,
    /// Bumped when a switch writes the file, so a look that read it before
    /// cannot put the old reading back or count the switch as made outside.
    pub generation: u64,
    /// The ChatGPT accounts the app keeps, as last listed.
    pub accounts: Vec<Account>,
    /// The owner's nicknames of the custom providers, by table id.
    pub nicknames: BTreeMap<String, String>,
    /// Whether each custom provider answers, by table id.
    pub reaches: BTreeMap<String, Reach>,
    /// A ChatGPT sign-in waiting for the browser.
    pub signing: Option<Arc<Signing>>,
    /// Sign-ins read from files or pasted, waiting for "Nhập".
    pub import: Option<crate::codex_import::Held>,
    /// "Tự đánh thức": its settings and runs, and the account it wakes now.
    pub wake: quota_station_core::codex::wake::Book,
    pub waking: Option<String>,
}

/// A ChatGPT sign-in under way (`codex login` in a folder of its own).
#[derive(Debug, Default)]
pub struct Signing {
    /// Set to stop it.
    pub cancel: AtomicBool,
    /// The page `codex login` printed, for "Mở lại trang đăng nhập".
    pub link: Mutex<Option<String>>,
    /// With a device code: the code to type on that page.
    pub code: Mutex<Option<String>>,
}

/// Whether the custom provider answers.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reach {
    /// `unknown` (not asked yet, or no provider), `up` or `down`.
    pub state: &'static str,
    pub ms: Option<u64>,
    /// When it stopped answering, RFC 3339.
    pub since: Option<String>,
    pub checked_at: Option<String>,
    /// The base URL it was asked at: a change asks again at once.
    #[serde(skip)]
    pub url: Option<String>,
}

pub(crate) fn rfc(at: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(at).ok()?.format(&Rfc3339).ok()
}

fn now_rfc() -> String {
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default()
}

impl Inner {
    /// The Codex folder the app reads and edits.
    pub fn codex_home(&self) -> PathBuf {
        codex::home(&self.home, self.settings.codex_home.as_deref())
    }

    /// How Codex reaches its models, for the menu bar: before the first look
    /// at `config.toml`, a Codex folder counts as the ChatGPT login.
    pub fn codex_use(&self) -> CodexUse {
        let Some(reading) = &self.codex.reading else {
            return if self.codex_home().is_dir() { CodexUse::ChatGpt } else { CodexUse::Missing };
        };
        if !reading.present {
            return CodexUse::Missing;
        }
        match reading.parsed().map(|p| (p.current(), p.custom.as_ref())) {
            Some((Current::Custom, Some(custom))) => CodexUse::Provider(custom.name.clone()),
            Some((Current::Other(id), _)) => CodexUse::Provider(id),
            _ => CodexUse::ChatGpt,
        }
    }

    fn codex_backups(&self) -> PathBuf {
        self.app_dir().path().join(files::BACKUP_DIR)
    }
}

// ===== The model =====

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexView {
    /// `~/.codex`
    pub home: String,
    /// Set in settings rather than Codex's own.
    pub custom_home: bool,
    /// The Codex folder is there.
    pub present: bool,
    /// `ok`, `missing`, `invalid` or `unreadable`.
    pub config: &'static str,
    pub reason: Option<String>,
    /// Sent back with a switch, so a file changed since is not overwritten.
    pub hash: Option<String>,
    /// `chatgpt`, `custom` or `other`.
    pub current: &'static str,
    /// The id in use when it is `other`.
    pub other: Option<String>,
    pub custom: Option<Custom>,
    /// What a switch to the custom provider would write.
    pub wants_requires: Option<bool>,
    /// "Dùng plugin khi chạy Custom API (giữ đăng nhập ChatGPT)".
    pub plugins: bool,
    /// The custom provider's table takes a pasted key ("Dán khoá").
    pub takes_key: bool,
    /// "Hoàn tác" after "Dán khoá", while the file is the one it left.
    pub key_undo: Option<KeyUndoView>,
    /// Why a switch to the custom provider, or back, cannot run.
    pub refusal: Option<Refusal>,
    pub back_refusal: Option<Refusal>,
    pub login: Login,
    pub reach: Reach,
    pub cc_switch: CcSwitchView,
    pub external: Option<ExternalView>,
    pub procs: Option<Procs>,
    pub version: Option<String>,
    pub mcp: Vec<Mcp>,
    pub resume: Option<BTreeMap<String, u32>>,
    pub last_write: Option<LastWrite>,
    /// When each backup was taken, newest first, RFC 3339.
    pub backups: Vec<String>,
    pub backup_dir: String,
    /// The last switches and changes, newest first.
    pub recent: Vec<RecentView>,
    /// The app's first switch on this Mac.
    pub first_switch_at: Option<String>,
    pub busy: Option<&'static str>,
    /// A top-level `openai_base_url` sends the ChatGPT login elsewhere.
    pub openai_base_url: bool,
    /// Every Codex account: the ChatGPT sign-ins the app keeps, then the
    /// provider tables of `config.toml`.
    pub accounts: Vec<AccountView>,
    /// Where the ChatGPT sign-ins not in the Codex folder wait.
    pub accounts_dir: String,
    /// A sign-in with a device code: the page and the code to type there.
    pub login_code: Option<LoginCode>,
    /// "Tự đánh thức tài khoản ChatGPT".
    pub trigger: crate::codex_wake::TriggerView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginCode {
    pub url: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyUndoView {
    pub at: Option<String>,
    /// What it puts back: `placeholder`, `empty` or `none`.
    pub token: &'static str,
    /// It puts the old `base_url` back too.
    pub base_url: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchView {
    /// `CC Switch.app` is in /Applications.
    pub installed: bool,
    pub running: bool,
    /// Its folder `~/.cc-switch` is there (only its presence is looked at).
    pub data: bool,
    /// What it left in `config.toml`.
    pub traces: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalView {
    /// When the file changed.
    pub at: Option<String>,
    pub last_at: Option<String>,
    /// The provider id the app wrote, and the one in use now.
    pub last_provider: String,
    pub now: String,
    pub mcp_changed: bool,
    pub mcp_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastWrite {
    pub at: Option<String>,
    pub provider: String,
    /// The file is still the one the app wrote.
    pub matches: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentView {
    pub at: String,
    pub from: String,
    pub to: String,
    /// `switched` or `observed`.
    pub result: String,
    pub restarted: Option<bool>,
}

fn current_name(c: &Current) -> (&'static str, Option<String>) {
    match c {
        Current::ChatGpt => ("chatgpt", None),
        Current::Custom => ("custom", None),
        Current::Other(id) => ("other", Some(id.clone())),
    }
}

pub fn view(inner: &Inner) -> CodexView {
    let live = &inner.codex;
    let app = inner.app_dir();
    let record = files::load(&app);
    let home = inner.codex_home();
    let empty = Reading {
        present: false,
        file: FileState::Missing,
        hash: None,
        modified_at: None,
        login: Login::Missing,
    };
    let reading = live.reading.as_ref().unwrap_or(&empty);
    let parsed = reading.parsed();
    let (config, reason) = match &reading.file {
        FileState::Ok(_) => ("ok", None),
        FileState::Missing => ("missing", None),
        FileState::Invalid(r) => ("invalid", Some(r.clone())),
        FileState::Unreadable(r) => ("unreadable", Some(r.clone())),
    };
    let (current, other) = parsed.map(|p| current_name(&p.current())).unwrap_or(("chatgpt", None));
    let login = reading.login;
    let plugins = inner.settings.codex.plugins;
    let plan = |target| parsed.map(|p| config::plan(p, target, login.on_disk(), plugins));
    let to_custom = plan(Target::Custom);
    let refusal = match &to_custom {
        Some(Err(r)) => Some(r.clone()),
        None => Some(Refusal::NoCustom),
        Some(Ok(_)) => None,
    };
    let back_refusal = plan(Target::ChatGpt).and_then(|r| r.err());
    let wants_requires = to_custom.and_then(|r| r.ok()).and_then(|p| p.requires).map(|(_, w)| w);

    let external = match (parsed, reading.hash.as_deref()) {
        (Some(p), Some(hash)) if record.changed_outside(hash) => Some(ExternalView {
            at: reading.modified_at.and_then(rfc),
            last_at: record.written_at.and_then(rfc),
            last_provider: codex::provider_id(record.written_provider.as_deref()),
            now: codex::provider_id(p.selected.as_deref()),
            mcp_changed: record.written_mcp.as_deref()
                != Some(config::mcp_fingerprint(&p.mcp).as_str()),
            mcp_count: p.mcp.len(),
        }),
        _ => None,
    };
    let key_undo =
        record.key_undo.as_ref().filter(|u| reading.hash.as_deref() == Some(u.hash.as_str())).map(
            |u| KeyUndoView {
                at: rfc(u.at),
                token: match u.token.as_deref() {
                    None => "none",
                    Some(config::PROXY_TOKEN_PLACEHOLDER) => "placeholder",
                    Some(_) => "empty",
                },
                base_url: u.base_url.is_some(),
            },
        );
    let last_write = record.written_hash.as_ref().map(|w| LastWrite {
        at: record.written_at.and_then(rfc),
        provider: codex::provider_id(record.written_provider.as_deref()),
        matches: reading.hash.as_deref() == Some(w.as_str()),
    });

    let backups = files::backups(&inner.codex_backups())
        .iter()
        .filter_map(|p| {
            let t = std::fs::metadata(p).and_then(|m| m.modified()).ok()?;
            OffsetDateTime::from(t).format(&Rfc3339).ok()
        })
        .collect();
    let lines = log::read(&app);
    let first_switch_at = lines.iter().find(|l| l.result == "switched").and_then(|l| rfc(l.at));
    let recent = lines
        .iter()
        .rev()
        .filter(|l| l.result != "failed")
        .take(RECENT)
        .filter_map(|l| {
            Some(RecentView {
                at: rfc(l.at)?,
                from: l.from.clone(),
                to: l.to.clone(),
                result: l.result.clone(),
                restarted: l.restarted,
            })
        })
        .collect();

    CodexView {
        home: runner::display_path(&home, &inner.home),
        custom_home: inner.settings.codex_home.is_some(),
        present: reading.present,
        config,
        reason,
        hash: reading.hash.clone(),
        current,
        other,
        custom: parsed.and_then(|p| p.custom.clone()),
        wants_requires,
        plugins,
        takes_key: parsed.is_some_and(codex::takes_key),
        key_undo,
        refusal,
        back_refusal,
        login,
        reach: live.reach.clone(),
        cc_switch: CcSwitchView {
            installed: std::path::Path::new("/Applications/CC Switch.app").exists(),
            running: live.procs.as_ref().is_some_and(|p| p.cc_switch),
            data: inner.home.join(".cc-switch").is_dir(),
            traces: parsed.map(|p| p.traces.clone()).unwrap_or_default(),
        },
        external,
        procs: live.procs.clone(),
        version: live.version.as_ref().and_then(|(_, v)| v.clone()),
        mcp: parsed.map(|p| p.mcp.clone()).unwrap_or_default(),
        resume: live.resume.clone(),
        last_write,
        backups,
        backup_dir: runner::display_path(&inner.codex_backups(), &inner.home),
        recent,
        first_switch_at,
        busy: live.busy,
        openai_base_url: parsed.is_some_and(|p| p.openai_base_url),
        accounts: codex_accounts::view(inner, parsed, login, current),
        accounts_dir: runner::display_path(&app.path().join(accounts::DIR), &inner.home),
        login_code: live.signing.as_ref().and_then(|s| {
            let lock = |m: &Mutex<Option<String>>| m.lock().ok().and_then(|v| v.clone());
            Some(LoginCode { url: lock(&s.link)?, code: lock(&s.code)? })
        }),
        trigger: crate::codex_wake::view(inner),
    }
}

// ===== The thread =====

pub fn spawn(app: AppHandle, state: AppState) {
    thread::Builder::new()
        .name("codex".into())
        .spawn(move || run(&app, &state))
        .expect("Codex thread starts");
}

fn run(app: &AppHandle, state: &AppState) {
    // Before M9 the switch kept `config.toml` whole in its backups.
    let backups = state.lock().codex_backups();
    files::scrub_backups(&backups);
    loop {
        look(app, state);
        let inner = state.lock();
        if inner.codex.now {
            continue;
        }
        let every = if inner.schedule.visible() { VISIBLE_EVERY } else { HIDDEN_EVERY };
        let _ = state.codex_wake.wait_timeout(inner, every);
    }
}

/// Asks the thread to look now; with `resume`, it counts the sessions too.
pub fn request(state: &AppState, resume: bool) {
    {
        let mut inner = state.lock();
        inner.codex.now = true;
        inner.codex.count_resume |= resume;
    }
    state.codex_wake.notify_all();
}

/// Reads the files, the accounts and the process table, notes a change of
/// provider made outside the app, then asks every custom provider.
fn look(app: &AppHandle, state: &AppState) {
    let (home, dir, count, user_home, generation) = {
        let mut inner = state.lock();
        inner.codex.now = false;
        let count = std::mem::take(&mut inner.codex.count_resume);
        (inner.codex_home(), inner.app_dir(), count, inner.home.clone(), inner.codex.generation)
    };
    let record = files::load(&dir);
    let reading = codex::read(&home, &record);
    let listed = list(&home, &dir);
    let procs = procs::read();
    let resume = count.then(|| resume::count(&home));
    let found = procs::discover(&user_home);
    let targets: Vec<(String, String)> = reading
        .parsed()
        .map(|p| {
            p.providers.iter().filter_map(|c| Some((c.id.clone(), c.base_url.clone()?))).collect()
        })
        .unwrap_or_default();
    let chosen = reading.parsed().and_then(|p| p.custom.as_ref()).map(|c| c.id.clone());
    {
        let mut inner = state.lock();
        if inner.codex.generation != generation || inner.codex.busy == Some("switching") {
            // A switch wrote meanwhile: its own reading stands; look again.
            inner.codex.now = true;
            return;
        }
        observe(&dir, &mut inner.codex, &record, &reading);
        if inner.codex.version.as_ref().map(|(p, _)| Some(p)) != Some(found.as_ref()) {
            inner.codex.version = found.map(|p| {
                let v = procs::version(&p);
                (p, v)
            });
        }
        inner.codex.reading = Some(reading);
        if let Some((accounts, nicknames)) = listed {
            inner.codex.accounts = accounts;
            inner.codex.nicknames = nicknames;
        }
        inner.codex.procs = procs.or(inner.codex.procs.take());
        if resume.is_some() {
            inner.codex.resume = resume;
        }
        inner.codex.reaches.retain(|id, _| targets.iter().any(|(t, _)| t == id));
        if chosen.as_ref().is_none_or(|id| !inner.codex.reaches.contains_key(id)) {
            inner.codex.reach = Reach { state: "unknown", ..Reach::default() };
        }
    }
    reader::publish(app, state);
    if targets.is_empty() {
        return;
    }
    let answers: Vec<(String, String, Answer)> = thread::scope(|scope| {
        let asks: Vec<_> = targets
            .iter()
            .map(|(id, url)| scope.spawn(move || (id.clone(), url.clone(), probe::check(url))))
            .collect();
        asks.into_iter().filter_map(|a| a.join().ok()).collect()
    });
    let mut inner = state.lock();
    let live = &mut inner.codex;
    for (id, url, answer) in answers {
        let before = live.reaches.get(&id).cloned().unwrap_or_default();
        live.reaches.insert(id, reach_of(&before, url, answer));
    }
    if let Some(r) = chosen.and_then(|id| live.reaches.get(&id).cloned()) {
        live.reach = r;
    }
    drop(inner);
    reader::publish(app, state);
}

/// The ChatGPT accounts the app keeps and the providers' nicknames; a
/// switch cut short is put back on the way.
pub fn list(
    home: &std::path::Path,
    dir: &AppDir,
) -> Option<(Vec<Account>, BTreeMap<String, String>)> {
    let platform = System::system();
    let store = Store::new(dir, home, &platform);
    let accounts = store.list(OffsetDateTime::now_utc()).ok()?;
    Some((accounts, store.registry().nicknames))
}

/// A provider's answer, keeping since when it has been down at `url`.
fn reach_of(before: &Reach, url: String, answer: Answer) -> Reach {
    let at = now_rfc();
    match answer {
        Answer::Up { ms } => {
            Reach { state: "up", ms: Some(ms), since: None, checked_at: Some(at), url: Some(url) }
        }
        Answer::Down { .. } | Answer::BadUrl => {
            let since = before
                .since
                .clone()
                .filter(|_| before.state == "down" && before.url.as_deref() == Some(url.as_str()));
            Reach {
                state: "down",
                ms: None,
                since: since.or(Some(at.clone())),
                checked_at: Some(at),
                url: Some(url),
            }
        }
    }
}

/// A provider in use that the app did not write: kept in the log once.
fn observe(dir: &AppDir, live: &mut Live, record: &Record, reading: &Reading) {
    let Some(parsed) = reading.parsed() else { return };
    let now = codex::provider_id(parsed.selected.as_deref());
    let before = live.seen.replace(now.clone());
    let Some(before) = before else { return };
    let outside = reading
        .hash
        .as_deref()
        .is_some_and(|h| record.changed_outside(h) || record.written_hash.is_none());
    if before != now && outside {
        let at = OffsetDateTime::now_utc().unix_timestamp();
        let _ = log::append(
            dir,
            &log::Line {
                at,
                from: before,
                to: now,
                result: "observed".into(),
                restarted: None,
                requires: None,
            },
        );
    }
}

// ===== Commands =====

/// Switches Codex to `target` after the owner confirmed it on the sheet.
/// `base` is the file's hash the sheet showed: a file changed since is not
/// overwritten. Rejects with `busy` while a switch or a restart runs.
#[tauri::command]
pub async fn codex_switch(
    app: AppHandle,
    state: State<'_, AppState>,
    target: Target,
    base: Option<String>,
) -> Result<Outcome, String> {
    let shared = state.inner().clone();
    let plugins = shared.lock().settings.codex.plugins;
    write(&app, &shared, move |home, dir| {
        codex::switch(home, dir, target, base.as_deref(), plugins, OffsetDateTime::now_utc())
    })
    .await
}

/// Runs one write of `config.toml` (a switch, a key, its undo) with the
/// Codex thread held off, then reads the file again. Rejects with `busy`
/// while another switch, restart or MCP write runs.
pub(crate) async fn write<T: Send + 'static>(
    app: &AppHandle,
    shared: &AppState,
    run: impl FnOnce(&std::path::Path, &AppDir) -> T + Send + 'static,
) -> Result<T, String> {
    let (home, dir) = {
        let mut inner = shared.lock();
        if inner.codex.busy.is_some() || inner.mcp.busy == Some("writing") {
            return Err("busy".into());
        }
        inner.codex.busy = Some("switching");
        inner.codex.generation += 1;
        (inner.codex_home(), inner.app_dir())
    };
    reader::publish(app, shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        let out = run(&home, &dir);
        let record = files::load(&dir);
        let reading = codex::read(&home, &record);
        (out, reading, list(&home, &dir))
    })
    .await;
    {
        let mut inner = shared.lock();
        inner.codex.busy = None;
        inner.codex.generation += 1;
        if let Ok((_, reading, listed)) = &done {
            if let Some(p) = reading.parsed() {
                inner.codex.seen = Some(codex::provider_id(p.selected.as_deref()));
            }
            inner.codex.reading = Some(reading.clone());
            if let Some((accounts, nicknames)) = listed {
                inner.codex.accounts = accounts.clone();
                inner.codex.nicknames = nicknames.clone();
            }
        }
    }
    request(shared, false);
    reader::publish(app, shared);
    done.map(|(out, _, _)| out).map_err(|e| e.to_string())
}

/// The key sheet's preview: the lines "Dán khoá" would change, the key
/// shown only as its hint. Writes nothing.
#[tauri::command]
pub async fn codex_key_preview(
    state: State<'_, AppState>,
    key: String,
    base_url: Option<String>,
) -> Result<KeyPreview, String> {
    let (home, dir) = {
        let inner = state.lock();
        (inner.codex_home(), inner.app_dir())
    };
    tauri::async_runtime::spawn_blocking(move || {
        codex::key_preview(&home, &dir, &key, base_url.as_deref())
    })
    .await
    .map_err(|_| "error".to_string())
}

/// "Dán khoá", after the owner confirmed its preview. `base` is the file's
/// hash the preview showed.
#[tauri::command]
pub async fn codex_set_key(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    base_url: Option<String>,
    base: String,
) -> Result<KeyOutcome, String> {
    let shared = state.inner().clone();
    write(&app, &shared, move |home, dir| {
        codex::set_key(home, dir, &key, base_url.as_deref(), &base, OffsetDateTime::now_utc())
    })
    .await
    // A join error's text could name the closure's values: say no more.
    .map_err(|e| if e == "busy" { e } else { "error".into() })
}

/// "Hoàn tác" after "Dán khoá".
#[tauri::command]
pub async fn codex_undo_key(
    app: AppHandle,
    state: State<'_, AppState>,
    base: String,
) -> Result<KeyOutcome, String> {
    let shared = state.inner().clone();
    write(&app, &shared, move |home, dir| {
        codex::undo_key(home, dir, &base, OffsetDateTime::now_utc())
    })
    .await
}

/// "Dùng plugin khi chạy Custom API": kept in the settings for the next
/// switch. Writes nothing to Codex: applying it to the provider in use is
/// a switch the owner confirms.
#[tauri::command]
pub fn set_codex_plugins(
    app: AppHandle,
    state: State<'_, AppState>,
    on: bool,
) -> Result<Model, String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.codex.plugins = on;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// Restarts Codex's background process, after the owner confirmed it on its
/// own sheet. `after_switch` marks the last switch in the log as followed
/// by it.
#[tauri::command]
pub async fn codex_restart(
    app: AppHandle,
    state: State<'_, AppState>,
    after_switch: bool,
) -> Result<Restart, String> {
    let shared = state.inner().clone();
    let (home, user_home, dir) = {
        let mut inner = shared.lock();
        if inner.codex.busy.is_some() {
            return Err("busy".into());
        }
        inner.codex.busy = Some("restarting");
        (inner.codex_home(), inner.home.clone(), inner.app_dir())
    };
    reader::publish(&app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        // The process table now, not the last look's.
        let running = procs::read().map(|p| p.daemon).unwrap_or(true);
        let out = procs::restart_daemon(procs::discover(&user_home).as_deref(), &home, running);
        if after_switch {
            let _ = log::mark_restarted(&dir, out == Restart::Restarted);
        }
        out
    })
    .await;
    shared.lock().codex.busy = None;
    request(&shared, false);
    reader::publish(&app, &shared);
    done.map_err(|e| e.to_string())
}

/// "Giữ như bây giờ" and "Đã hiểu": the file changed outside the app is
/// kept as it is, and not reported again.
#[tauri::command]
pub fn codex_keep(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    {
        let inner = state.lock();
        let reading = inner.codex.reading.as_ref().ok_or("unread")?;
        let hash = reading.hash.as_deref().ok_or("unread")?;
        codex::keep(&inner.app_dir(), hash, reading.parsed()).map_err(|e| e.to_string())?;
    }
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// The Codex tab opened, or "Kiểm lại": look now and count the sessions.
#[tauri::command]
pub fn codex_check(state: State<'_, AppState>) {
    request(state.inner(), true);
}

/// Sets Codex's folder; empty goes back to `~/.codex`. Rejects a path that
/// is not a folder.
#[tauri::command]
pub fn set_codex_home(
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
            if !p.is_dir() {
                return Err("notFound".into());
            }
            Some(p)
        };
        let mut settings = inner.settings.clone();
        settings.codex_home = chosen;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
        inner.codex.reading = None;
        inner.codex.seen = None;
        inner.codex.resume = None;
    }
    request(state.inner(), true);
    reader::publish(&app, &state);
    Ok(state.lock().model())
}
