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
use std::sync::Arc;
use std::time::Duration;

use quota_station_core::advanced::settings::Own;
use quota_station_core::agents::copy::{
    self, CopyError, Diff, ExportAsk, Here, Inventory, Job, JobInput, Remote, Selection, UndoAsk,
};
use quota_station_core::agents::restore;
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::{self, instructions, kho, pack, skills, Agent};
use quota_station_core::ext::Hooks;
use quota_station_core::peers;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tauri::{AppHandle, State};
use time::OffsetDateTime;

use crate::advanced::state::AdvancedState;
use crate::ext::{Host, Local};

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
pub fn here(local: &Local) -> Here {
    let places = local.mcp_places();
    let dir = local.app_dir();
    let mac = local.machine();
    let version = local.app_version();
    let home = local.home();
    let codex_home = local.codex_home();
    let skill_places = local.skill_places();
    let readings = agents::read_all(&places);
    let store = kho::load(&dir);
    let mut p = pack::content(&home, &readings, &store, &mac, &version, OffsetDateTime::now_utc());
    let extras = local.pack_extras();
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
    println!("{}", refusal(why));
    copy::REFUSED
}

/// What a refusal prints: `{"error": "<why>"}`.
fn refusal(why: &str) -> String {
    serde_json::json!({ "error": why }).to_string()
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
pub fn cli(args: &[String], hooks: &'static dyn Hooks) -> i32 {
    answer(std::env::var_os("HOME").map(PathBuf::from), args, hooks)
}

/// [`cli`] in `home`, the home folder of the user it runs as.
fn answer(home: Option<PathBuf>, args: &[String], hooks: &'static dyn Hooks) -> i32 {
    let Some(home) = home else { return refuse("noHome") };
    let local = Local::open(home, hooks);
    let dir = local.app_dir();
    match args.first().map(String::as_str) {
        Some("read") => print(&here(&local).inventory()),
        Some("export") => {
            let Some(ask) = stdin_json::<ExportAsk>() else { return refuse("input") };
            if ask.protocol != copy::PROTOCOL {
                return refuse("protocol");
            }
            let secrets = Secrets::system();
            match copy::payload(&here(&local), &ask.selection, ask.keys.then_some(&secrets)) {
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
            let (places, tools) = (local.mcp_places(), local.restore_tools());
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

fn remote(host: &Host, alias: &str) -> Result<Remote, String> {
    let alias = alias.trim();
    let known = Own::of(&host.settings()).peers.iter().any(|p| p.alias == alias);
    if !peers::alias_ok(alias) || !known {
        return Err("unknownMac".into());
    }
    let ssh = peers::find_ssh(std::env::var_os("PATH").as_deref()).ok_or("noSsh")?;
    Ok(Remote::new(ssh, alias))
}

fn begin(own: &AdvancedState) -> Result<(), String> {
    let mut fields = own.lock();
    if fields.copy.busy {
        return Err("busy".into());
    }
    fields.copy.busy = true;
    Ok(())
}

/// "So sánh": reads the other Mac and this one. Writes nothing.
#[tauri::command]
pub async fn copy_compare(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    alias: String,
) -> Result<Comparison, String> {
    let host = Host::of(&app);
    let remote = remote(&host, &alias)?;
    begin(&own)?;
    let (local, o) = (host.local(), own.inner().clone());
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<Comparison, String> {
        let theirs = remote.read().map_err(|e| e.code())?;
        let mine = here(&local).inventory();
        let diff = copy::diff(&mine, &theirs);
        let out = Comparison {
            alias: remote.alias.clone(),
            mac: theirs.mac.clone(),
            at: OffsetDateTime::now_utc().unix_timestamp(),
            diff,
        };
        let mut fields = o.lock();
        fields.copy.alias = Some(remote.alias.clone());
        fields.copy.mine = Some(mine);
        fields.copy.theirs = Some(theirs);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string());
    own.lock().copy.busy = false;
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
    own: State<'_, Arc<AdvancedState>>,
    alias: String,
    push: bool,
    selection: Selection,
    keys: bool,
) -> Result<Json, String> {
    let host = Host::of(&app);
    let remote = remote(&host, &alias)?;
    if selection.is_empty() {
        return Err("nothing".into());
    }
    let (mine, theirs) = {
        let fields = own.lock();
        if fields.copy.alias.as_deref() != Some(remote.alias.as_str()) {
            return Err("stale".into());
        }
        (fields.copy.mine.clone(), fields.copy.theirs.clone())
    };
    let (Some(mine), Some(theirs)) = (mine, theirs) else { return Err("stale".into()) };
    begin(&own)?;
    let done = if push {
        let local = host.local();
        tauri::async_runtime::spawn_blocking(move || -> Result<Json, String> {
            let secrets = Secrets::system();
            let mut p = copy::payload(&here(&local), &selection, keys.then_some(&secrets))
                .map_err(|_| "skill".to_string())?;
            p.bases = theirs.bases.clone();
            p.file_bases = theirs.file_bases();
            on_their_mac(&remote, &JobInput::Apply { payload: Box::new(p) })
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r)
    } else {
        pull(&host, remote, selection, keys, mine).await
    };
    own.lock().copy.busy = false;
    done
}

/// A copy to this Mac: the other Mac sends the items, this Mac's app writes
/// them as a restore.
async fn pull(
    host: &Host,
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
    let done = host
        .write_agents(move |places, dir, tools| {
            copy::apply(places, dir, &Secrets::system(), tools, &payload, OffsetDateTime::now_utc())
        })
        .await;
    let done = done?.map_err(|e| job_error(&serde_json::to_value(e).unwrap_or(Json::Null)))?;
    if !done.written.is_empty() || done.can_undo {
        host.agents_changed();
    }
    serde_json::to_value(done).map_err(|e| e.to_string())
}

/// "Hoàn tác": the copy that ran at `at`, on the Mac it was written to;
/// `force` once the owner said yes to what changed since.
#[tauri::command]
pub async fn copy_undo(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
    alias: String,
    push: bool,
    at: i64,
    force: bool,
) -> Result<Json, String> {
    let host = Host::of(&app);
    let remote = remote(&host, &alias)?;
    begin(&own)?;
    let ask = UndoAsk { at, force };
    let done = if push {
        tauri::async_runtime::spawn_blocking(move || on_their_mac(&remote, &JobInput::Undo { ask }))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r)
    } else {
        undo_here(&host, ask).await
    };
    own.lock().copy.busy = false;
    done
}

async fn undo_here(host: &Host, ask: UndoAsk) -> Result<Json, String> {
    let done = host
        .write_agents(move |places, dir, tools| {
            copy::undo(places, dir, &Secrets::system(), tools, &ask, OffsetDateTime::now_utc())
        })
        .await;
    let done: Result<restore::Undone, Json> = done?;
    let undone = done.map_err(|e| job_error(&e))?;
    serde_json::to_value(undone).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use quota_station_core::agents::copy;

    use super::{answer, refusal};
    use crate::advanced::hooks::AdvancedHooks;

    fn words(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    /// A home folder of its own, never the owner's.
    fn home(tag: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("qs-agents-cli-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        home
    }

    #[test]
    fn a_refusal_prints_its_reason_as_json_and_exits_7() {
        assert_eq!(refusal("protocol"), r#"{"error":"protocol"}"#);
        assert_eq!(refusal("noHome"), r#"{"error":"noHome"}"#);
        assert_eq!(copy::REFUSED, 7);
    }

    #[test]
    fn without_a_home_every_word_is_refused() {
        for word in ["read", "export", "start", "status", "nonsense"] {
            assert_eq!(answer(None, &words(&[word]), &AdvancedHooks), copy::REFUSED, "{word}");
        }
    }

    #[test]
    fn a_word_it_does_not_know_exits_2() {
        let home = home("unknown");
        assert_eq!(answer(Some(home.clone()), &words(&["nonsense"]), &AdvancedHooks), 2);
        assert_eq!(answer(Some(home.clone()), &[], &AdvancedHooks), 2);
        let _ = std::fs::remove_dir_all(home);
    }

    #[test]
    fn status_answers_with_no_run_yet() {
        let home = home("status");
        assert_eq!(answer(Some(home.clone()), &words(&["status"]), &AdvancedHooks), 0);
        let _ = std::fs::remove_dir_all(home);
    }
}
