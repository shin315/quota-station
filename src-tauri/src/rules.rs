// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation editor's commands (F06) and the thread that keeps the other
//! Macs' copies in step. A save checks the rules in the rotation file's format,
//! writes this Mac's rotation file, then copies it to every other Mac in
//! settings over SSH. A Mac that could not
//! take it is tried again every 5 minutes; one holding a newer copy waits
//! for the owner to choose.
//!
//! On Windows only the editor is built: the save, the preview, and no copy
//! to another Mac (the other Macs, SSH and their conflicts are a Mac's).

use std::sync::Arc;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use quota_station_core::advanced::settings::{self, Own, Peer};
use quota_station_core::claude::rotate;
#[cfg(unix)]
use quota_station_core::peers::{self, PeerRecord, PeerState, RemoteError, RETRY_MINUTES};
use quota_station_core::quota;
use quota_station_core::rotation::{self, ApplyError, FileState, Rules, Stamp};
use quota_station_core::switcher;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::advanced::model::DryRunView;
use crate::advanced::state::AdvancedState;
use crate::ext::{Host, Model};

/// How often the sync thread looks for a Mac whose wait is over.
#[cfg(unix)]
const LOOK_EVERY: Duration = Duration::from_secs(60);

/// Starts the sync thread, from the advanced part's `start_threads`.
#[cfg(unix)]
pub fn spawn(host: Host, own: Arc<AdvancedState>) {
    thread::Builder::new()
        .name("rotation-sync".into())
        .spawn(move || loop {
            {
                let mut fields = own.lock();
                if !fields.sync_now {
                    fields = own
                        .sync_wake
                        .wait_timeout(fields, LOOK_EVERY)
                        .unwrap_or_else(|e| e.into_inner())
                        .0;
                }
                fields.sync_now = false;
            }
            push_peers(&host, &own, false, false);
        })
        .expect("sync thread starts");
}

/// Copies this Mac's rotation file to the other Macs that need it: never
/// tried, behind this Mac's file, or failed and past their 5 minutes. With
/// `now_all`, a failed Mac is tried without waiting. With `force_conflicts`
/// (the owner kept this Mac's copy), a Mac holding a newer or hand-edited
/// copy is written too. Does nothing before the app has saved the file once
/// (no `updated` stamp: nothing of the app's to copy), or while the file is
/// one the rules refuse.
#[cfg(unix)]
pub fn push_peers(host: &Host, own: &AdvancedState, now_all: bool, force_conflicts: bool) {
    let _one_at_a_time = own.push_lock.lock().unwrap_or_else(|e| e.into_inner());
    let (file, peers) = (host.claude_paths().rotation_file(), Own::of(&host.settings()).peers);
    let FileState::Ok { text, parsed } = rotation::load(&file) else { return };
    let Some(stamp) = parsed.stamp else { return };
    let version = rotation::version(&text);
    let now = OffsetDateTime::now_utc();
    // A wait counts as over when every Mac is tried now.
    let check_at = if now_all { now + time::Duration::minutes(RETRY_MINUTES) } else { now };
    // Under the app's lock too, so a model drawn meanwhile sees the Macs
    // being written all at once.
    let due: Vec<(Peer, bool)> = host.locked(|_| {
        let mut fields = own.lock();
        let due: Vec<(Peer, bool)> = peers
            .into_iter()
            .filter_map(|p| {
                let record = fields.sync.peers.get(&p.alias);
                let conflict = record.is_some_and(|r| {
                    matches!(r.state, PeerState::Newer { .. } | PeerState::Edited { .. })
                });
                peers::due(record, &version, check_at).then_some((p, force_conflicts && conflict))
            })
            .collect();
        fields.pushing.extend(due.iter().map(|(p, _)| p.alias.clone()));
        due
    });
    if due.is_empty() {
        return;
    }
    host.publish();
    let ssh = peers::find_ssh(std::env::var_os("PATH").as_deref());
    let jobs: Vec<_> = due
        .into_iter()
        .map(|(p, force)| {
            let (ssh, text, stamp, version) =
                (ssh.clone(), text.clone(), stamp.clone(), version.clone());
            thread::spawn(move || {
                let record = match ssh {
                    Some(ssh) => peers::push(&ssh, &p.alias, &text, &stamp, now, force),
                    None => PeerRecord {
                        state: PeerState::Failed {
                            error: "không thấy ssh trên máy này".into(),
                            retry_at: retry_at(now),
                        },
                        against: version,
                        their_text: None,
                    },
                };
                (p.alias, record)
            })
        })
        .collect();
    let done: Vec<(String, PeerRecord)> = jobs.into_iter().filter_map(|j| j.join().ok()).collect();
    host.locked(|locked| {
        let peers = Own::of(locked.settings()).peers;
        let mut fields = own.lock();
        for (alias, record) in done {
            // A Mac removed from settings meanwhile is not kept.
            if peers.iter().any(|p| p.alias == alias) {
                fields.sync.peers.insert(alias, record);
            }
        }
        fields.pushing.clear();
        let _ = fields.sync.save(&fields.sync_file);
    });
    host.publish();
}

#[cfg(unix)]
fn retry_at(now: OffsetDateTime) -> String {
    use time::format_description::well_known::Rfc3339;
    (now + time::Duration::minutes(RETRY_MINUTES)).format(&Rfc3339).unwrap_or_default()
}

/// Why a rotation command did nothing, or put the file back.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesError {
    /// `busy`, `changed`, `invalid`, `write`, `unreachable`, `noFile` or
    /// `notNewer`.
    pub kind: &'static str,
    /// The rule's reason (`rotation_bad_value:personal_hours`) or the error.
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
        let detail = match e {
            ApplyError::Changed => None,
            ApplyError::Invalid(r) | ApplyError::Write(r) => Some(r),
        };
        RulesError { kind, detail, restored: false }
    }
}

/// Writes `text` here when the file is still at `base` and its rules hold,
/// then copies it to the other Macs (on a Mac). `text` gets the file on
/// disk first.
async fn write_here(
    host: &Host,
    own: &Arc<AdvancedState>,
    base: Option<String>,
    force_conflicts: bool,
    text: impl FnOnce(&FileState, &str) -> Result<String, RulesError> + Send + 'static,
) -> Result<Model, RulesError> {
    let (file, store, machine, hooks) =
        (host.claude_paths().rotation_file(), host.claude_store(), host.machine(), host.hooks());
    {
        let mut fields = own.lock();
        if fields.rotation_saving {
            return Err(RulesError::new("busy"));
        }
        fields.rotation_saving = true;
    }
    host.publish();
    let done = tauri::async_runtime::spawn_blocking(move || {
        let now = OffsetDateTime::now_utc();
        let text = text(&rotation::load(&file), &machine)?;
        rotation::apply(&file, &text, base.as_deref(), now).map_err(RulesError::from)?;
        // The Claude row shows the new rules at once.
        Ok(quota::claude::own_now(&store, hooks, now))
    })
    .await
    .unwrap_or_else(|e| {
        Err(RulesError { kind: "write", detail: Some(e.to_string()), restored: false })
    });
    // The end of the save and the new reading, whole for a model drawn
    // meanwhile.
    let ok = host.locked(|locked| {
        own.lock().rotation_saving = false;
        if let Some(report) = done? {
            locked.store_claude_report(report);
        }
        Ok::<(), RulesError>(())
    });
    host.publish();
    ok?;
    #[cfg(unix)]
    {
        let (host2, own2) = (host.clone(), own.clone());
        let _ = tauri::async_runtime::spawn_blocking(move || {
            push_peers(&host2, &own2, true, force_conflicts)
        })
        .await;
    }
    #[cfg(not(unix))]
    let _ = force_conflicts;
    Ok(host.model())
}

/// Saves the editor's rules on this Mac, stamped with this Mac's name and
/// the time, and copies them to the other Macs. `base` is the file version
/// the editor started from.
#[tauri::command]
pub async fn save_rotation(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    rules: Rules,
    base: Option<String>,
) -> Result<Model, RulesError> {
    write_here(&Host::of(&app), &own, base, false, move |current, machine| {
        rules.check().map_err(|r| RulesError {
            kind: "invalid",
            detail: Some(r),
            restored: false,
        })?;
        let stamp = Stamp::next(OffsetDateTime::now_utc(), machine, current.stamp());
        Ok(rotation::render(&rules, &stamp, current.text()))
    })
    .await
}

/// Takes the copy on the Mac at `alias` as this Mac's file (it was newer, or
/// edited there), stamped again so it is the newest copy, then copies it to
/// the other Macs.
#[cfg(unix)]
#[tauri::command]
pub async fn adopt_rotation(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    alias: String,
    base: Option<String>,
) -> Result<Model, RulesError> {
    let host = Host::of(&app);
    if !Own::of(&host.settings()).peers.iter().any(|p| p.alias == alias) {
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
    write_here(&host, &own, base, false, move |current, machine| {
        rotation::adopt(&theirs, current.stamp(), OffsetDateTime::now_utc(), machine)
            .map_err(|r| RulesError { kind: "invalid", detail: Some(r), restored: false })
    })
    .await
}

/// Keeps this Mac's file over another Mac's newer or edited copy: stamps it
/// again, so it is the newest, and copies it out. That Mac keeps a backup.
#[cfg(unix)]
#[tauri::command]
pub async fn keep_rotation(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    base: Option<String>,
) -> Result<Model, RulesError> {
    write_here(&Host::of(&app), &own, base, true, |current, machine| match current {
        FileState::Ok { text, parsed } => {
            let stamp = Stamp::next(OffsetDateTime::now_utc(), machine, parsed.stamp.as_ref());
            Ok(rotation::render(&parsed.rules, &stamp, Some(text)))
        }
        _ => Err(RulesError::new("noFile")),
    })
    .await
}

/// Tries every Mac that is behind now, without waiting for its 5 minutes.
#[cfg(unix)]
#[tauri::command]
pub async fn retry_rotation(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
) -> Result<Model, String> {
    let host = Host::of(&app);
    let (host2, own2) = (host.clone(), own.inner().clone());
    tauri::async_runtime::spawn_blocking(move || push_peers(&host2, &own2, true, false))
        .await
        .map_err(|e| e.to_string())?;
    Ok(host.model())
}

/// Sets the other Macs. Refuses an alias `ssh` would not take safely, or one
/// listed twice (`badAlias:<i>`, `sameAlias:<i>`, `tooMany`).
#[cfg(unix)]
#[tauri::command]
pub fn set_peers(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    peers: Vec<Peer>,
) -> Result<Model, String> {
    let peers = settings::peers(&peers).map_err(|e| e.code())?;
    let host = Host::of(&app);
    host.locked(|locked| {
        let mut settings = locked.settings().clone();
        let kept: Vec<String> = peers.iter().map(|p| p.alias.clone()).collect();
        Own::change(&mut settings, |own| own.peers = peers);
        locked.save_settings(settings)?;
        let mut fields = own.lock();
        fields.sync.peers.retain(|alias, _| kept.contains(alias));
        let _ = fields.sync.save(&fields.sync_file);
        Ok::<(), String>(())
    })?;
    own.request_sync();
    host.publish();
    Ok(host.model())
}

/// What "Thử SSH" found on another Mac. Reads its rotation file and
/// changes nothing.
#[cfg(unix)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerTest {
    pub reached: bool,
    pub error: Option<String>,
    /// It has a rotation file.
    pub has_file: bool,
    pub stamp: Option<Stamp>,
}

#[cfg(unix)]
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

/// What the rotation would decide now with the editor's rules, from the
/// readings the app has: switches nothing, reads nothing over the network
/// and leaves the rotation file alone.
#[tauri::command]
pub async fn preview_rotation(app: AppHandle, rules: Rules) -> Result<DryRunView, String> {
    rules.check()?;
    let host = Host::of(&app);
    let (store, tools) = (host.claude_store(), tools(&host));
    tauri::async_runtime::spawn_blocking(move || {
        let now = OffsetDateTime::now_utc();
        let decided = rotate::dry_run(&store, &tools, &rotate::Policy::Rules(rules), now);
        let run = switcher::decided(&decided.line);
        Ok(DryRunView { decision: run.decision, raw: run.raw, at: String::new() })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// What a run of the rotation uses: the `claude` program the app found, and
/// the system's own for the rest.
pub fn tools(host: &Host) -> rotate::Tools {
    rotate::Tools { claude: host.claude_program(), ..rotate::Tools::system(&host.home()) }
}
