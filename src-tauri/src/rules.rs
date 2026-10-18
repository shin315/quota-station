//! The rotation editor's commands (F06) and the thread that keeps the other
//! Macs' copies in step. A save writes this Mac's rotation file, has
//! quota-axi check it (the file before goes back when it refuses), then
//! copies it to every other Mac in settings over SSH. A Mac that could not
//! take it is tried again every 5 minutes; one holding a newer copy waits
//! for the owner to choose.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use quota_station_core::peers::{self, PeerRecord, PeerState, RemoteError, RETRY_MINUTES};
use quota_station_core::rotation::{self, ApplyError, FileState, Rules, Stamp};
use quota_station_core::settings::{self, Peer};
use quota_station_core::switcher;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, DryRunView, Model};
use crate::windows;

/// How often the sync thread looks for a Mac whose wait is over.
const LOOK_EVERY: Duration = Duration::from_secs(60);

pub fn spawn(app: AppHandle, state: AppState) {
    thread::Builder::new()
        .name("rotation-sync".into())
        .spawn(move || loop {
            {
                let mut inner = state.lock();
                if !inner.sync_now {
                    inner = state
                        .sync_wake
                        .wait_timeout(inner, LOOK_EVERY)
                        .unwrap_or_else(|e| e.into_inner())
                        .0;
                }
                inner.sync_now = false;
            }
            push_peers(&app, &state, false);
        })
        .expect("sync thread starts");
}

/// Copies this Mac's rotation file to the other Macs that need it: never
/// tried, behind this Mac's file, or failed and past their 5 minutes. With
/// `now_all`, a failed Mac is tried without waiting. Does nothing before the
/// app has saved the file once (no `updated` stamp: nothing of the app's to
/// copy), or while the file is one quota-axi would refuse.
pub fn push_peers(app: &AppHandle, state: &AppState, now_all: bool) {
    let _one_at_a_time = state.push_lock.lock().unwrap_or_else(|e| e.into_inner());
    let (file, peers) = {
        let inner = state.lock();
        (rotation::path(&inner.home), inner.settings.peers.clone())
    };
    let FileState::Ok { text, parsed } = rotation::load(&file) else { return };
    let Some(stamp) = parsed.stamp else { return };
    let now = OffsetDateTime::now_utc();
    // A wait counts as over when every Mac is tried now.
    let check_at = if now_all { now + time::Duration::minutes(RETRY_MINUTES) } else { now };
    let due: Vec<Peer> = {
        let mut inner = state.lock();
        let due: Vec<Peer> = peers
            .into_iter()
            .filter(|p| peers::due(inner.sync.peers.get(&p.alias), &stamp, check_at))
            .collect();
        inner.pushing.extend(due.iter().map(|p| p.alias.clone()));
        due
    };
    if due.is_empty() {
        return;
    }
    reader::publish(app, state);
    let ssh = peers::find_ssh(std::env::var_os("PATH").as_deref());
    let jobs: Vec<_> = due
        .into_iter()
        .map(|p| {
            let (ssh, text, stamp) = (ssh.clone(), text.clone(), stamp.clone());
            thread::spawn(move || {
                let record = match ssh {
                    Some(ssh) => peers::push(&ssh, &p.alias, &text, &stamp, now),
                    None => PeerRecord {
                        state: PeerState::Failed {
                            error: "không thấy ssh trên máy này".into(),
                            retry_at: retry_at(now),
                        },
                        against: stamp.at.clone(),
                        their_text: None,
                    },
                };
                (p.alias, record)
            })
        })
        .collect();
    let done: Vec<(String, PeerRecord)> = jobs.into_iter().filter_map(|j| j.join().ok()).collect();
    {
        let mut inner = state.lock();
        for (alias, record) in done {
            // A Mac removed from settings meanwhile is not kept.
            if inner.settings.peers.iter().any(|p| p.alias == alias) {
                inner.sync.peers.insert(alias, record);
            }
        }
        inner.pushing.clear();
        let _ = inner.sync.save(&inner.sync_file);
    }
    reader::publish(app, state);
}

fn retry_at(now: OffsetDateTime) -> String {
    use time::format_description::well_known::Rfc3339;
    (now + time::Duration::minutes(RETRY_MINUTES)).format(&Rfc3339).unwrap_or_default()
}

/// Why a rotation command did nothing, or put the file back.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesError {
    /// `busy`, `changed`, `invalid`, `write`, `refused`, `unchecked`,
    /// `unreachable`, `noFile` or `notNewer`.
    pub kind: &'static str,
    /// quota-axi's reason (`rotation_bad_value:personal_hours`) or the error.
    pub detail: Option<String>,
    /// The file before is back on disk.
    pub restored: bool,
}

impl RulesError {
    fn new(kind: &'static str) -> RulesError {
        RulesError { kind, detail: None, restored: false }
    }
}

impl From<ApplyError> for RulesError {
    fn from(e: ApplyError) -> RulesError {
        let kind = e.kind();
        let (detail, restored) = match e {
            ApplyError::Changed => (None, false),
            ApplyError::Invalid(r) | ApplyError::Write(r) => (Some(r), false),
            ApplyError::Refused { reason, restored } => (Some(reason), restored),
            ApplyError::Unchecked { error, restored } => (Some(error), restored),
        };
        RulesError { kind, detail, restored }
    }
}

/// Writes `text` here when the file is still at `base`, has quota-axi check
/// it, then copies it to the other Macs. `text` gets the file on disk first.
async fn write_here(
    app: &AppHandle,
    state: &AppState,
    base: Option<String>,
    text: impl FnOnce(&FileState, &str) -> Result<String, RulesError> + Send + 'static,
) -> Result<Model, RulesError> {
    let (file, quota_axi, machine) = {
        let mut inner = state.lock();
        if inner.rotation_saving {
            return Err(RulesError::new("busy"));
        }
        inner.rotation_saving = true;
        (rotation::path(&inner.home), inner.quota_axi(), inner.machine())
    };
    reader::publish(app, state);
    let done = tauri::async_runtime::spawn_blocking(move || {
        let now = OffsetDateTime::now_utc();
        let text = text(&rotation::load(&file), &machine)?;
        rotation::apply(&file, quota_axi.as_deref(), &text, base.as_deref(), now)
            .map_err(RulesError::from)
    })
    .await
    .unwrap_or_else(|e| {
        Err(RulesError { kind: "write", detail: Some(e.to_string()), restored: false })
    });
    let ok = {
        let mut inner = state.lock();
        inner.rotation_saving = false;
        match done {
            Ok(applied) => {
                inner.store_claude(applied.report);
                Ok(())
            }
            Err(e) => Err(e),
        }
    };
    reader::publish(app, state);
    ok?;
    let (app2, state2) = (app.clone(), state.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || push_peers(&app2, &state2, true)).await;
    Ok(state.lock().model())
}

/// Saves the editor's rules on this Mac, stamped with this Mac's name and
/// the time, and copies them to the other Macs. `base` is the file version
/// the editor started from.
#[tauri::command]
pub async fn save_rotation(
    app: AppHandle,
    state: State<'_, AppState>,
    rules: Rules,
    base: Option<String>,
) -> Result<Model, RulesError> {
    let state = state.inner().clone();
    write_here(&app, &state, base, move |current, machine| {
        rules.check().map_err(|r| RulesError {
            kind: "invalid",
            detail: Some(r),
            restored: false,
        })?;
        let stamp = Stamp::new(OffsetDateTime::now_utc(), machine);
        Ok(rotation::render(&rules, &stamp, current.text()))
    })
    .await
}

/// Takes the copy on the Mac at `alias` as this Mac's file (it was newer, or
/// edited there), then copies it to the other Macs.
#[tauri::command]
pub async fn adopt_rotation(
    app: AppHandle,
    state: State<'_, AppState>,
    alias: String,
    base: Option<String>,
) -> Result<Model, RulesError> {
    let state = state.inner().clone();
    if !state.lock().settings.peers.iter().any(|p| p.alias == alias) {
        return Err(RulesError::new("unknownMac"));
    }
    let theirs = tauri::async_runtime::spawn_blocking(move || {
        let ssh = peers::find_ssh(std::env::var_os("PATH").as_deref())
            .ok_or_else(|| RulesError::new("unreachable"))?;
        peers::read(&ssh, &alias)
            .map_err(|e| RulesError {
                kind: "unreachable",
                detail: Some(e.to_string()),
                restored: false,
            })?
            .ok_or_else(|| RulesError::new("noFile"))
    })
    .await
    .unwrap_or_else(|e| {
        Err(RulesError { kind: "unreachable", detail: Some(e.to_string()), restored: false })
    })?;
    write_here(&app, &state, base, move |_, _| {
        rotation::parse(&theirs).map_err(|r| RulesError {
            kind: "invalid",
            detail: Some(r),
            restored: false,
        })?;
        Ok(theirs)
    })
    .await
}

/// Keeps this Mac's file over another Mac's newer or edited copy: stamps it
/// again, so it is the newest, and copies it out. That Mac keeps a backup.
#[tauri::command]
pub async fn keep_rotation(
    app: AppHandle,
    state: State<'_, AppState>,
    base: Option<String>,
) -> Result<Model, RulesError> {
    let state = state.inner().clone();
    write_here(&app, &state, base, |current, machine| match current {
        FileState::Ok { text, parsed } => {
            let stamp = Stamp::new(OffsetDateTime::now_utc(), machine);
            Ok(rotation::render(&parsed.rules, &stamp, Some(text)))
        }
        _ => Err(RulesError::new("noFile")),
    })
    .await
}

/// Tries every Mac that is behind now, without waiting for its 5 minutes.
#[tauri::command]
pub async fn retry_rotation(app: AppHandle, state: State<'_, AppState>) -> Result<Model, String> {
    let state = state.inner().clone();
    let (app2, state2) = (app.clone(), state.clone());
    tauri::async_runtime::spawn_blocking(move || push_peers(&app2, &state2, true))
        .await
        .map_err(|e| e.to_string())?;
    let model = state.lock().model();
    Ok(model)
}

/// Sets the other Macs. Refuses an alias `ssh` would not take safely, or one
/// listed twice (`badAlias:<i>`, `sameAlias:<i>`, `tooMany`).
#[tauri::command]
pub fn set_peers(
    app: AppHandle,
    state: State<'_, AppState>,
    peers: Vec<Peer>,
) -> Result<Model, String> {
    let peers = settings::peers(&peers).map_err(|e| e.code())?;
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.peers = peers;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
        let kept: Vec<String> = inner.settings.peers.iter().map(|p| p.alias.clone()).collect();
        inner.sync.peers.retain(|alias, _| kept.contains(alias));
        let _ = inner.sync.save(&inner.sync_file);
    }
    state.request_sync();
    reader::publish(&app, &state);
    Ok(state.lock().model())
}

/// What "Thử SSH" found on another Mac. Reads its rotation file and
/// changes nothing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerTest {
    pub reached: bool,
    pub error: Option<String>,
    /// It has a rotation file.
    pub has_file: bool,
    pub stamp: Option<Stamp>,
}

#[tauri::command]
pub async fn test_peer(alias: String) -> Result<PeerTest, String> {
    if !peers::alias_ok(alias.trim()) {
        return Err("badAlias".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Some(ssh) = peers::find_ssh(std::env::var_os("PATH").as_deref()) else {
            return PeerTest {
                reached: false,
                error: Some("không thấy ssh".into()),
                has_file: false,
                stamp: None,
            };
        };
        match peers::read(&ssh, alias.trim()) {
            Ok(text) => PeerTest {
                reached: true,
                error: None,
                has_file: text.is_some(),
                stamp: text.and_then(|t| rotation::parse(&t).ok()).and_then(|p| p.stamp),
            },
            Err(e) => PeerTest {
                reached: false,
                error: Some(match e {
                    RemoteError::Timeout => "timeout".into(),
                    other => other.to_string(),
                }),
                has_file: false,
                stamp: None,
            },
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Gives each preview its own draft file, so two previews at once never
/// remove each other's.
static PREVIEWS: AtomicU64 = AtomicU64::new(0);

/// What the switcher would decide now with the editor's rules: its
/// `--dry-run`, pointed at a draft file in the app's folder. Switches nothing
/// and leaves the rotation file alone.
#[tauri::command]
pub async fn preview_rotation(
    state: State<'_, AppState>,
    rules: Rules,
) -> Result<DryRunView, String> {
    rules.check()?;
    let (home, draft, machine) = {
        let inner = state.lock();
        let n = PREVIEWS.fetch_add(1, Ordering::Relaxed);
        let draft: PathBuf = inner.settings_file.with_file_name(format!("rotation-preview-{n}"));
        (inner.home.clone(), draft, inner.machine())
    };
    tauri::async_runtime::spawn_blocking(move || {
        let now = OffsetDateTime::now_utc();
        let current = rotation::load(&rotation::path(&home));
        let text = rotation::render(&rules, &Stamp::new(now, &machine), current.text());
        if let Some(dir) = draft.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&draft, text).map_err(|e| e.to_string())?;
        let run = switcher::dry_run_with(&switcher::script(&home), &draft);
        let _ = std::fs::remove_file(&draft);
        let run = run.map_err(|e| e.kind().to_string())?;
        Ok(DryRunView { decision: run.decision, raw: run.raw, at: String::new() })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The editor has changes not saved: closing the main window asks first.
#[tauri::command]
pub fn rules_dirty(state: State<'_, AppState>, dirty: bool) {
    state.lock().rules_dirty = dirty;
}

/// Closes the main window after the owner answered the unsaved-changes dialog.
#[tauri::command]
pub fn close_main(app: AppHandle, state: State<'_, AppState>) {
    state.lock().rules_dirty = false;
    windows::close_main(&app);
}
