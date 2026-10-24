// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Chép sang Mac khác tab (M13, D), and `quota-station agents <word>`:
//! what another Mac's Quota Station runs here over SSH.
//!
//! The tab compares this Mac with one of Cài đặt › Các Mac khác, then
//! copies the items ticked one way or the other
//! (`quota_station_core::agents::copy`). A copy from this Mac is written by
//! the other Mac's app, which keeps it as its last restore: the undo runs
//! there too. A copy to this Mac goes through the same guard as a restore,
//! so one write runs at a time and never during a Codex switch.
//!
//! The commands run here for another Mac, all without a window:
//!
//! - `read`: this Mac's inventory, no key;
//! - `export`: the items asked on stdin, their keys only when asked;
//! - `start`: a copy (or its undo) given on stdin, run in its own process
//!   so that SSH need not wait for it; the keys stay in memory;
//! - `status`: how that run stands.
//!
//! A refusal prints `{"error": "<why>"}` and exits 7.

use std::io::{Read, Write as _};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use quota_station_core::agents::copy::{
    self, CopyError, Diff, ExportAsk, Here, Inventory, Job, JobInput, Remote, Selection, UndoAsk,
};
use quota_station_core::agents::restore;
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::{self, instructions, kho, pack, skills, Agent};
use quota_station_core::peers;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::state::{AppState, Inner, Shared};

/// How often the app asks the other Mac how its write stands.
const POLL: Duration = Duration::from_secs(1);
/// The most read on stdin: a payload with skills' files.
const MAX_INPUT: u64 = 256 * 1024 * 1024;

/// The last comparison, kept for the copy that follows it: what each Mac's
/// files were when read.
#[derive(Debug, Default)]
pub struct Live {
    pub alias: Option<String>,
    pub mine: Option<Inventory>,
    pub theirs: Option<Inventory>,
    pub busy: bool,
}

/// This Mac now, as a pack sees it, with what a copy needs besides.
pub fn here(state: &AppState) -> Here {
    let (places, dir, mac, version, home, codex_home, skill_places) = {
        let inner = state.lock();
        (
            inner.mcp_places(),
            inner.app_dir(),
            inner.machine(),
            inner.app_version.clone(),
            inner.home.clone(),
            inner.codex_home(),
            inner.skill_places(),
        )
    };
    let readings = agents::read_all(&places);
    let store = kho::load(&dir);
    let mut p = pack::content(&home, &readings, &store, &mac, &version, OffsetDateTime::now_utc());
    let extras = crate::skills::pack_extras(state);
    p.extend(Some(extras.skills), Some(extras.plugins), Some(extras.instructions));
    p.folders = extras.folders;
    let choices = instructions::load(&dir);
    let read =
        instructions::read_all(&home, &codex_home, &choices, &pack::secret_values(&readings));
    let files = copy::file_infos(&read, &choices);
    let versions = copy::versions(&skills::read(&skill_places));
    let agents = Agent::ALL.into_iter().filter(|a| places.present(*a)).collect();
    Here { pack: p, readings, files, versions, agents }
}

// ===== `quota-station agents <word>` =====

fn print<T: Serialize>(v: &T) -> i32 {
    match serde_json::to_string(v) {
        Ok(text) => {
            println!("{text}");
            0
        }
        Err(_) => refuse("json"),
    }
}

fn refuse(why: &str) -> i32 {
    println!("{}", serde_json::json!({ "error": why }));
    copy::REFUSED
}

fn stdin_json<T: for<'de> Deserialize<'de>>() -> Option<T> {
    let mut text = String::new();
    std::io::stdin().take(MAX_INPUT).read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

/// What a job's process gets on stdin: when it began, and what to do.
#[derive(Serialize, Deserialize)]
struct JobStdin {
    started: i64,
    input: JobInput,
}

/// Answers another Mac: `args` are the words after `agents`.
pub fn cli(args: &[String]) -> i32 {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else { return refuse("noHome") };
    let settings_file =
        home.join("Library/Application Support/com.shin315.quota-station/settings.json");
    let mut inner = Inner::new(home, settings_file, crate::machine_name());
    inner.app_version = env!("CARGO_PKG_VERSION").to_string();
    let state = Shared::new(inner);
    let dir = state.lock().app_dir();
    match args.first().map(String::as_str) {
        Some("read") => print(&here(&state).inventory()),
        Some("export") => {
            let Some(ask) = stdin_json::<ExportAsk>() else { return refuse("input") };
            if ask.protocol != copy::PROTOCOL {
                return refuse("protocol");
            }
            let secrets = Secrets::system();
            match copy::payload(&here(&state), &ask.selection, ask.keys.then_some(&secrets)) {
                Ok(p) => print(&p),
                Err(_) => refuse("skill"),
            }
        }
        Some("start") => {
            if copy::busy(&dir) {
                return refuse("busy");
            }
            let Some(input) = stdin_json::<JobInput>() else { return refuse("input") };
            let started = OffsetDateTime::now_utc().unix_timestamp();
            let Ok(exe) = std::env::current_exe() else { return refuse("exe") };
            // Its own process group: SSH closing does not stop it.
            let child = Command::new(exe)
                .args(["agents", "job"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn();
            let Ok(mut child) = child else { return refuse("spawn") };
            let kind = input.kind().to_string();
            let text = serde_json::to_vec(&JobStdin { started, input }).unwrap_or_default();
            let sent = child.stdin.take().map(|mut s| s.write_all(&text).is_ok()).unwrap_or(false);
            if !sent {
                return refuse("spawn");
            }
            print(&Job {
                kind,
                state: "running".into(),
                started,
                pid: child.id(),
                result: None,
                error: None,
            })
        }
        Some("job") => {
            let Some(job) = stdin_json::<JobStdin>() else { return 2 };
            let running = Job {
                kind: job.input.kind().into(),
                state: "running".into(),
                started: job.started,
                pid: std::process::id(),
                result: None,
                error: None,
            };
            let _ = copy::save_job(&dir, &running);
            let (places, tools) = {
                let inner = state.lock();
                (inner.mcp_places(), inner.restore_tools())
            };
            let now = OffsetDateTime::now_utc();
            copy::run_job(&places, &dir, &Secrets::system(), &tools, &job.input, job.started, now);
            0
        }
        Some("status") => print(&copy::job(&dir)),
        _ => {
            eprintln!("quota-station agents read|export|start|status");
            2
        }
    }
}

// ===== The tab =====

/// What "So sánh" found.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub alias: String,
    /// The other Mac's name in its app.
    pub mac: String,
    /// When it was read, Unix seconds.
    pub at: i64,
    pub diff: Diff,
}

fn remote(state: &AppState, alias: &str) -> Result<Remote, String> {
    let alias = alias.trim();
    if !peers::alias_ok(alias) || !state.lock().settings.peers.iter().any(|p| p.alias == alias) {
        return Err("unknownMac".into());
    }
    let ssh = peers::find_ssh(std::env::var_os("PATH").as_deref()).ok_or("noSsh")?;
    Ok(Remote::new(ssh, alias))
}

fn begin(state: &AppState) -> Result<(), String> {
    let mut inner = state.lock();
    if inner.copy.busy {
        return Err("busy".into());
    }
    inner.copy.busy = true;
    Ok(())
}

/// "So sánh": reads the other Mac and this one. Writes nothing.
#[tauri::command]
pub async fn copy_compare(state: State<'_, AppState>, alias: String) -> Result<Comparison, String> {
    let shared = state.inner().clone();
    let remote = remote(&shared, &alias)?;
    begin(&shared)?;
    let s = shared.clone();
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<Comparison, String> {
        let theirs = remote.read().map_err(|e| e.code())?;
        let mine = here(&s).inventory();
        let diff = copy::diff(&mine, &theirs);
        let out = Comparison {
            alias: remote.alias.clone(),
            mac: theirs.mac.clone(),
            at: OffsetDateTime::now_utc().unix_timestamp(),
            diff,
        };
        let mut inner = s.lock();
        inner.copy.alias = Some(remote.alias.clone());
        inner.copy.mine = Some(mine);
        inner.copy.theirs = Some(theirs);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string());
    shared.lock().copy.busy = false;
    done?
}

/// The word the window gets for a write that failed there or here.
fn job_error(e: &Json) -> String {
    let kind = e.get("kind").and_then(Json::as_str).unwrap_or_default();
    let detail = e.get("detail");
    match (kind, detail) {
        ("run", Some(d)) => job_error(d),
        ("changed", Some(Json::Array(agents))) => format!(
            "changed:{}",
            agents.iter().filter_map(Json::as_str).collect::<Vec<_>>().join(",")
        ),
        ("changed", Some(d)) => {
            let list = |k: &str| -> Vec<String> {
                d.get(k)
                    .and_then(Json::as_array)
                    .map(|v| v.iter().filter_map(Json::as_str).map(str::to_string).collect())
                    .unwrap_or_default()
            };
            let mut out = vec![list("agents").join(",")];
            out.extend(list("files").into_iter().map(|f| format!("file:{f}")));
            out.extend(list("skills").into_iter().map(|s| format!("skill:{s}")));
            format!("changed:{}", out.join("\n"))
        }
        ("", _) => e.as_str().unwrap_or("failed").to_string(),
        (k, _) => k.to_string(),
    }
}

fn ended(job: Job) -> Result<Json, String> {
    match (job.state.as_str(), job.result, job.error) {
        ("done", Some(r), _) => Ok(r),
        ("lost", _, _) => Err("lost".into()),
        (_, _, Some(e)) => Err(job_error(&e)),
        _ => Err("failed".into()),
    }
}

/// Starts `input` on the other Mac and waits for it.
fn on_their_mac(remote: &Remote, input: &JobInput) -> Result<Json, String> {
    let job = remote.start(input).map_err(|e| e.code())?;
    let job =
        remote.wait(&job, POLL, copy::WAIT_LIMIT, &std::thread::sleep).map_err(|e| e.code())?;
    ended(job)
}

/// "Chép": the items ticked, `push` from this Mac, else to it. The keys
/// go only with `keys`.
#[tauri::command]
pub async fn copy_run(
    app: AppHandle,
    state: State<'_, AppState>,
    alias: String,
    push: bool,
    selection: Selection,
    keys: bool,
) -> Result<Json, String> {
    let shared = state.inner().clone();
    let remote = remote(&shared, &alias)?;
    if selection.is_empty() {
        return Err("nothing".into());
    }
    let (mine, theirs) = {
        let inner = shared.lock();
        if inner.copy.alias.as_deref() != Some(remote.alias.as_str()) {
            return Err("stale".into());
        }
        (inner.copy.mine.clone(), inner.copy.theirs.clone())
    };
    let (Some(mine), Some(theirs)) = (mine, theirs) else { return Err("stale".into()) };
    begin(&shared)?;
    let done = if push {
        let s = shared.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<Json, String> {
            let secrets = Secrets::system();
            let mut p = copy::payload(&here(&s), &selection, keys.then_some(&secrets))
                .map_err(|_| "skill".to_string())?;
            p.bases = theirs.bases.clone();
            p.file_bases = theirs.file_bases();
            on_their_mac(&remote, &JobInput::Apply { payload: Box::new(p) })
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r)
    } else {
        pull(&app, &shared, remote, selection, keys, mine).await
    };
    shared.lock().copy.busy = false;
    done
}

/// A copy to this Mac: the other Mac sends the items, this Mac's app writes
/// them as a restore.
async fn pull(
    app: &AppHandle,
    shared: &AppState,
    remote: Remote,
    selection: Selection,
    keys: bool,
    mine: Inventory,
) -> Result<Json, String> {
    let payload = tauri::async_runtime::spawn_blocking(move || remote.export(&selection, keys))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e: CopyError| e.code())?;
    let mut payload = payload;
    payload.bases = mine.bases.clone();
    payload.file_bases = mine.file_bases();
    crate::skills::begin_restore(shared)?;
    let (places, dir) = crate::agents::begin_write(app, shared).inspect_err(|_| {
        shared.lock().skills.busy = None;
    })?;
    let tools = shared.lock().restore_tools();
    let done = tauri::async_runtime::spawn_blocking(move || {
        copy::apply(&places, &dir, &Secrets::system(), &tools, &payload, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    crate::agents::end_write(app, shared).await;
    crate::skills::end_restore(app, shared).await;
    let done = done?.map_err(|e| job_error(&serde_json::to_value(e).unwrap_or(Json::Null)))?;
    if !done.written.is_empty() || done.can_undo {
        crate::backup::after_change(app, shared);
    }
    serde_json::to_value(done).map_err(|e| e.to_string())
}

/// "Hoàn tác": the copy that ran at `at`, on the Mac it was written to;
/// `force` once the owner said yes to what changed since.
#[tauri::command]
pub async fn copy_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    alias: String,
    push: bool,
    at: i64,
    force: bool,
) -> Result<Json, String> {
    let shared = state.inner().clone();
    let remote = remote(&shared, &alias)?;
    begin(&shared)?;
    let ask = UndoAsk { at, force };
    let done = if push {
        tauri::async_runtime::spawn_blocking(move || on_their_mac(&remote, &JobInput::Undo { ask }))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r)
    } else {
        undo_here(&app, &shared, ask).await
    };
    shared.lock().copy.busy = false;
    done
}

async fn undo_here(app: &AppHandle, shared: &AppState, ask: UndoAsk) -> Result<Json, String> {
    crate::skills::begin_restore(shared)?;
    let (places, dir) = crate::agents::begin_write(app, shared).inspect_err(|_| {
        shared.lock().skills.busy = None;
    })?;
    let tools = shared.lock().restore_tools();
    let done = tauri::async_runtime::spawn_blocking(move || {
        copy::undo(&places, &dir, &Secrets::system(), &tools, &ask, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    crate::agents::end_write(app, shared).await;
    crate::skills::end_restore(app, shared).await;
    let done: Result<restore::Undone, Json> = done?;
    let undone = done.map_err(|e| job_error(&e))?;
    serde_json::to_value(undone).map_err(|e| e.to_string())
}
