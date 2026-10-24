// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// The order of a switch is ported from Cockpit Tools
// `src-tauri/src/commands/account_desktop_switch.rs` (the single-switch gate,
// the 60-second preparation, quitting before and opening after, writing the
// login before the account pointers, a reopen failure reported apart) and
// `src-tauri/src/modules/account.rs` (`prepare_account_for_injection`: a
// fresh token, then the project and GCP ToS probe)
// (1.3.70, commit 244b957, CC BY-NC-SA 4.0); changes: the probe is the
// quota read's `loadCodeAssist` (`cloud_code`), asked by its rule (no GCP
// ToS mark or no project) and kept as it keeps it; macOS and
// Antigravity.app 2.0 or later only (the SQLite login of older versions and
// the instance management are not ported); the app is quit with
// `osascript` and opened with `open -b` (`desktop`), never killed; the
// result is checked with quota-axi five seconds later; a log line and the
// six-hour watch (`watch`) are the app's own; no WebSocket broadcast.

//! One manual switch of the Antigravity account, start to finish, after the
//! owner confirmed it on the sheet:
//!
//! 1. one switch at a time (a second one is refused as busy); the account
//!    must be logged in here and not need a new login;
//! 2. Antigravity.app must be 2.0 or later, or not installed (then only
//!    agy's login changes); one whose version cannot be read stops here;
//! 3. the account's token is made fresh and, while it has no GCP ToS mark
//!    or no project, `loadCodeAssist` is asked for them, all within 60
//!    seconds;
//! 4. with "Thoát app rồi chuyển", Antigravity.app is asked to quit and
//!    given 20 seconds; if it does not quit, the shared login and the
//!    pointers are not touched. "Chuyển" was offered for an app that was not
//!    running, so one that runs again by then stops the switch;
//! 5. the account must still be logged in here, and the shared login is
//!    written (`credential`); a failure leaves the old one as it was;
//! 6. only then the index's current account and the binding quota-axi reads
//!    are written, both or neither, the six-hour watch starts, and the app
//!    is opened again if it was quit (a failure to open is reported apart:
//!    the switch stands);
//! 7. five seconds later quota-axi reads Antigravity again, skipping its
//!    cache, to say which account agy is really on.
//!
//! Each switch adds one line to `antigravity/switch-log.jsonl`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::cloud_code::{self, CloudCode, Context, Hosts};
use super::credential::{CredentialError, SharedLogin};
use super::desktop::{Desktop, Installed, QuitError};
use super::index::{self, AccountIndex, Standing};
use super::login::{self, UseError};
use super::oauth::Endpoints;
use super::store::Keychain;
use super::token::TokenData;
use super::version::Identity;
use super::watch;
use crate::app_dir::AppDir;
use crate::axi::Response;
use crate::runner::{self, ProviderId};

/// quota-axi reads the binding from this file, as it does in Cockpit's folder.
pub const BINDING_FILE: &str = "antigravity_legacy_instances.json";
pub const LOG_FILE: &str = "switch-log.jsonl";
/// The log keeps this many switches (Cockpit keeps 200 too).
const LOG_KEEP: usize = 200;

/// The waits of one switch.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// The token refresh and the project lookup together.
    pub prepare: Duration,
    /// For Antigravity.app to quit.
    pub quit: Duration,
    /// Before the check with quota-axi.
    pub settle: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            prepare: Duration::from_secs(60),
            quit: super::desktop::QUIT_WAIT,
            settle: Duration::from_secs(5),
        }
    }
}

/// The button the owner pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum How {
    /// "Chuyển": Antigravity.app was not running.
    Plain,
    /// "Thoát app rồi chuyển".
    QuitFirst,
    /// "Chuyển, để app chạy": the running app may write its own login back.
    KeepRunning,
}

/// What the project lookup found; `None` where it found nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMeta {
    pub project_id: Option<String>,
    pub is_gcp_tos: Option<bool>,
}

/// Looks up an account's Cloud Code project and GCP ToS mark before a
/// switch, as Cockpit does. `email` only names the account in Google's
/// error answers.
pub trait Probe: Send + Sync {
    fn probe(&self, token: &TokenData, email: &str) -> ProjectMeta;
}

/// The quota read's lookup: Cloud Code's `loadCodeAssist`.
pub struct CloudCodeProbe {
    pub hosts: Hosts,
    pub identity: Identity,
}

impl Probe for CloudCodeProbe {
    fn probe(&self, token: &TokenData, email: &str) -> ProjectMeta {
        let cloud = CloudCode { hosts: &self.hosts, identity: &self.identity };
        // A failure does not stop the switch, as in Cockpit.
        match cloud.load_code_assist(&token.access_token, email, &Context::from_token(token)) {
            // Asked and answered, kept as the quota read keeps it: a token
            // without the mark would be asked again on every switch.
            Ok(info) => ProjectMeta {
                project_id: info.project_id,
                is_gcp_tos: Some(info.is_gcp_tos.unwrap_or(false)),
            },
            Err(_) => ProjectMeta::default(),
        }
    }
}

/// No lookup: the token goes on as it is.
pub struct NoProbe;

impl Probe for NoProbe {
    fn probe(&self, _: &TokenData, _: &str) -> ProjectMeta {
        ProjectMeta::default()
    }
}

/// Everything one switch uses.
pub struct Deps<'a> {
    pub app: &'a AppDir,
    pub endpoints: &'a Endpoints,
    pub keychain: &'a Keychain,
    pub probe: Arc<dyn Probe>,
    pub shared: &'a SharedLogin,
    pub desktop: &'a Desktop,
    pub quota_axi: Option<&'a Path>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The app's id of the account to switch to.
    pub id: String,
    pub how: How,
    /// The account in use now, as the last reading showed it; the index's
    /// current account when there is none.
    pub from: Option<String>,
}

/// What a switch did. `reopened` is `None` when Antigravity.app was not quit
/// for the switch, else whether it opened again. `saved` is false when the
/// index and the binding could not both be written, so both still name the
/// account from before: the switch itself stands, but quota-axi may name
/// the account in use wrongly until the next switch.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Outcome {
    /// The check shows `number` in use. `basis` is quota-axi's: `reading`
    /// when agy's own numbers matched, `cockpit` when only the binding the
    /// switch wrote answered.
    Switched { number: usize, how: How, basis: Option<String>, reopened: Option<bool>, saved: bool },
    /// The login was written, and the check did not show `number` in use:
    /// it shows `active`, or could not tell (`None`).
    Unconfirmed {
        number: usize,
        how: How,
        active: Option<u32>,
        reopened: Option<bool>,
        saved: bool,
    },
    /// Nothing was switched.
    Failed { number: usize, error: &'static str, detail: String, reopened: Option<bool> },
}

#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    /// The Antigravity report read for the check, to show at once.
    pub report: Option<Response>,
}

/// Why a switch stopped before writing anything, or at the write.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stop {
    Busy,
    UnknownAccount,
    NeedsLogin,
    Index(String),
    OldApp(String),
    /// Antigravity.app is there, but its version could not be read.
    AppVersion(String),
    PrepareTimeout,
    /// The preparation thread ended without an answer.
    PrepareLost,
    Prepare(UseError),
    AppState(String),
    AppRunning,
    NotQuit,
    Write(CredentialError),
}

impl Stop {
    fn kind(&self) -> &'static str {
        match self {
            Stop::Busy => "busy",
            Stop::UnknownAccount => "unknownAccount",
            Stop::NeedsLogin | Stop::Prepare(UseError::NeedsLogin) => "needsLogin",
            Stop::Index(_) => "index",
            Stop::OldApp(_) => "oldApp",
            Stop::AppVersion(_) => "appVersion",
            Stop::PrepareTimeout => "prepareTimeout",
            Stop::PrepareLost => "prepare",
            Stop::Prepare(UseError::Client(_)) => "client",
            Stop::Prepare(UseError::Keychain(_)) => "keychain",
            Stop::Prepare(UseError::Refresh(_)) => "refresh",
            Stop::AppState(_) => "appState",
            Stop::AppRunning => "appRunning",
            Stop::NotQuit => "notQuit",
            Stop::Write(CredentialError::Timeout) => "writeTimeout",
            Stop::Write(_) => "write",
        }
    }

    /// Words for the log and the UI's detail line; never a token or an email.
    fn detail(&self) -> String {
        match self {
            Stop::Index(e) | Stop::AppState(e) => e.clone(),
            Stop::OldApp(version) | Stop::AppVersion(version) => version.clone(),
            Stop::Prepare(UseError::Client(s)) => format!("{s:?}"),
            Stop::Prepare(e) => e.to_string(),
            Stop::Write(e) => e.to_string(),
            _ => String::new(),
        }
    }
}

static BUSY: AtomicBool = AtomicBool::new(false);

/// Held for the whole switch; a second switch meanwhile is refused.
struct Permit;

impl Permit {
    fn take() -> Option<Permit> {
        BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).ok().map(|_| Permit)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::SeqCst);
    }
}

/// Switches the Antigravity account. Blocks for the preparation, the quit
/// and `limits.settle` before the check. `now` is the clock in Unix seconds.
pub fn switch_antigravity(
    deps: &Deps,
    request: &Request,
    limits: Limits,
    now: &dyn Fn() -> i64,
) -> Done {
    let Some(_permit) = Permit::take() else {
        return failed(0, &Stop::Busy, None);
    };
    let index = match index::load(deps.app) {
        Ok(index) => index,
        Err(e) => return failed(0, &Stop::Index(e.to_string()), None),
    };
    let Some(number) = number_of(&index, &request.id) else {
        return failed(0, &Stop::UnknownAccount, None);
    };
    let email = index.accounts[number - 1].email.clone();
    let started = now();
    let from = request
        .from
        .clone()
        .filter(|f| number_of(&index, f).is_some())
        .or_else(|| index.current_account_id.clone())
        .filter(|f| *f != request.id);
    let log = |result: &str, error: Option<&str>, reopened: Option<bool>| {
        let _ = append_log(
            deps.app,
            &LogLine {
                at: started,
                from: from.clone(),
                to: request.id.clone(),
                how: request.how,
                result: result.to_string(),
                error: error.map(String::from),
                reopened,
            },
        );
    };
    let stop = |stop: Stop, reopened: Option<bool>| {
        log("failed", Some(stop.kind()), reopened);
        failed(number, &stop, reopened)
    };

    let statuses = index::load_statuses(deps.app);
    if matches!(statuses.accounts.get(&request.id), Some(Standing::NeedsLogin { .. })) {
        return stop(Stop::NeedsLogin, None);
    }
    let installed = deps.desktop.installed();
    if let Installed::TooOld { version } = &installed {
        return stop(Stop::OldApp(version.clone()), None);
    }
    if let Installed::Unreadable { detail } = &installed {
        return stop(Stop::AppVersion(detail.clone()), None);
    }
    let token = match prepare(deps, &request.id, &email, limits.prepare, started) {
        Ok(token) => token,
        Err(e) => return stop(e, None),
    };

    // Quit first when asked and the app runs; one that does not quit stops
    // the switch before anything is written. The process list decides, not
    // `installed`: an app whose plist could not be read may still run.
    let mut quit = false;
    if request.how != How::KeepRunning {
        match deps.desktop.running() {
            Ok(false) => {}
            Ok(true) if request.how == How::Plain => return stop(Stop::AppRunning, None),
            Ok(true) => match deps.desktop.quit(limits.quit) {
                Ok(()) => quit = true,
                Err(QuitError::StillRunning) => return stop(Stop::NotQuit, None),
                Err(QuitError::Unknown(e)) => return stop(Stop::AppState(e), None),
            },
            Err(e) => return stop(Stop::AppState(e), None),
        }
    }
    let reopen = || quit.then(|| deps.desktop.open().is_ok());

    // A logout while the token was made fresh leaves nothing to switch to.
    if !index::load(deps.app).is_ok_and(|i| number_of(&i, &request.id).is_some()) {
        let reopened = reopen();
        return stop(Stop::UnknownAccount, reopened);
    }
    if let Err(e) = deps.shared.write(&token) {
        // The old login is intact, so the app comes back on it.
        let reopened = reopen();
        return stop(Stop::Write(e), reopened);
    }

    // The login is switched. Now the pointers, then the watch, which an
    // earlier switch's watch never outlives.
    let saved = commit(deps.app, &request.id, now()).is_ok();
    let _ = match &from {
        Some(from) => watch::begin(deps.app, from, &request.id, now()),
        None => watch::clear(deps.app),
    };
    let reopened = reopen();

    thread::sleep(limits.settle);
    let report = deps.quota_axi.and_then(|p| runner::read_fresh(p, ProviderId::Agy).ok());
    let shown = report.as_ref().and_then(|r| shown_active(r, &index));
    if let Some((active, _)) = &shown {
        let id = index.accounts.get(*active as usize - 1).map(|a| a.id.as_str());
        watch::observe(deps.app, id, now());
    }
    let outcome = match shown {
        Some((active, basis)) if active as usize == number => {
            Outcome::Switched { number, how: request.how, basis, reopened, saved }
        }
        other => Outcome::Unconfirmed {
            number,
            how: request.how,
            active: other.map(|(a, _)| a),
            reopened,
            saved,
        },
    };
    log(
        match outcome {
            Outcome::Switched { .. } => "switched",
            _ => "unconfirmed",
        },
        None,
        reopened,
    );
    Done { outcome, report }
}

fn failed(number: usize, stop: &Stop, reopened: Option<bool>) -> Done {
    Done {
        outcome: Outcome::Failed { number, error: stop.kind(), detail: stop.detail(), reopened },
        report: None,
    }
}

/// The account's place in the index, from 1.
fn number_of(index: &AccountIndex, id: &str) -> Option<usize> {
    index.accounts.iter().position(|a| a.id == id).map(|i| i + 1)
}

/// Cockpit's preparation, in a thread so it can be given up after `limit`:
/// the token refreshed when it is about to expire, and the project and GCP
/// ToS mark looked up while the token lacks either, the quota read's rule
/// (`quota::read_account`). A preparation given up may still finish later;
/// it then only updates the app's own Keychain item.
fn prepare(
    deps: &Deps,
    id: &str,
    email: &str,
    limit: Duration,
    now: i64,
) -> Result<TokenData, Stop> {
    let (app, endpoints, keychain) =
        (deps.app.clone(), deps.endpoints.clone(), deps.keychain.clone());
    let (probe, id, email) = (deps.probe.clone(), id.to_string(), email.to_string());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let login = login::Deps { app: &app, endpoints: &endpoints, keychain: &keychain };
        let result = login::fresh_token(&login, &id, now).map(|mut token| {
            let has_project = cloud_code::valid_project_id(token.project_id.as_deref()).is_some();
            if token.is_gcp_tos.is_none() || !has_project {
                let meta = probe.probe(&token, &email);
                let mut changed = false;
                if let Some(gcp_tos) = meta.is_gcp_tos.filter(|g| token.is_gcp_tos != Some(*g)) {
                    token.is_gcp_tos = Some(gcp_tos);
                    changed = true;
                }
                if let Some(project) =
                    meta.project_id.filter(|p| token.project_id.as_ref() != Some(p))
                {
                    token.project_id = Some(project);
                    changed = true;
                }
                if changed {
                    // Like a refreshed token: used now even if keeping it failed.
                    let _ = keychain.save(&id, &token);
                }
            }
            token
        });
        let _ = tx.send(result);
    });
    match rx.recv_timeout(limit) {
        Ok(Ok(token)) => Ok(token),
        Ok(Err(e)) => Err(Stop::Prepare(e)),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(Stop::PrepareTimeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(Stop::PrepareLost),
    }
}

/// Points the index and the binding at the new account, after the login is
/// written (Cockpit's order). Both or neither: each file is replaced whole
/// (`write_private`), the index first; if the binding then fails, the index
/// is put back as it was, so the app and quota-axi never name different
/// accounts, and `saved: false` says the pointers still name the old one.
/// An account logged out meanwhile gets no pointer.
fn commit(app: &AppDir, id: &str, now: i64) -> Result<(), String> {
    let index_path = index::index_path(app);
    let before = app.read_to_string(&index_path).map_err(|e| e.to_string())?;
    let mut index = index::load(app).map_err(|e| e.to_string())?;
    let summary = index.accounts.iter_mut().find(|a| a.id == id).ok_or("account logged out")?;
    summary.last_used = now;
    index.current_account_id = Some(id.to_string());
    index::save(app, &index).map_err(|e| e.to_string())?;
    if let Err(e) = write_binding(app, id) {
        return Err(match app.write_private(&index_path, &before) {
            Ok(()) => e,
            Err(back) => format!("{e}; the index could not be put back: {back}"),
        });
    }
    Ok(())
}

pub fn binding_path(app: &AppDir) -> PathBuf {
    index::dir(app).join(BINDING_FILE)
}

/// Sets `defaultSettings.bindAccountId`, keeping anything else the file
/// holds. A file that is not a JSON object is replaced.
fn write_binding(app: &AppDir, id: &str) -> Result<(), String> {
    let path = binding_path(app);
    let mut root = app
        .read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    let settings = &mut root["defaultSettings"];
    if !settings.is_object() {
        *settings = serde_json::json!({});
    }
    settings["bindAccountId"] = serde_json::Value::String(id.to_string());
    let text = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    app.write_private(&path, &text).map_err(|e| e.to_string())
}

/// The account quota-axi shows in use, by its number in this app's list,
/// and how it told. Only a report read from this app's folder numbers the
/// accounts as the app does.
pub fn shown_active(report: &Response, index: &AccountIndex) -> Option<(u32, Option<String>)> {
    let cockpit = report.primary("agy")?.cockpit.as_ref()?;
    if cockpit.status != "ok" || cockpit.source.as_deref() != Some("quota-station") {
        return None;
    }
    let active = cockpit.accounts.iter().find(|a| a.active)?.account;
    (active >= 1 && (active as usize) <= index.accounts.len())
        .then(|| (active, cockpit.active_basis.clone()))
}

/// The app's id of the account the report shows in use.
pub fn shown_active_id(report: &Response, index: &AccountIndex) -> Option<String> {
    let (n, _) = shown_active(report, index)?;
    index.accounts.get(n as usize - 1).map(|a| a.id.clone())
}

/// One line of `switch-log.jsonl`. Ids only, never an email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Unix seconds.
    pub at: i64,
    pub from: Option<String>,
    pub to: String,
    pub how: How,
    /// `switched`, `unconfirmed` or `failed`.
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reopened: Option<bool>,
}

pub fn log_path(app: &AppDir) -> PathBuf {
    index::dir(app).join(LOG_FILE)
}

/// The log's lines, oldest first; lines it cannot read are skipped.
pub fn read_log(app: &AppDir) -> Vec<LogLine> {
    let text = app.read_to_string(&log_path(app)).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

fn append_log(app: &AppDir, line: &LogLine) -> std::io::Result<()> {
    let text = app.read_to_string(&log_path(app)).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    lines.push(serde_json::to_string(line).map_err(std::io::Error::other)?);
    let skip = lines.len().saturating_sub(LOG_KEEP);
    let mut out = lines[skip..].join("\n");
    out.push('\n');
    app.write_private(&log_path(app), &out)
}
