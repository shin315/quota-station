// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Tự đánh thức tài khoản ChatGPT" (M8), off until the owner turns it on:
//! a short message sent with Codex itself, `codex exec`, with each chosen
//! account's own folder as `CODEX_HOME` (the Codex folder for the account
//! Codex uses), so its five-hour window starts early, and so a sign-in that
//! ran out while it waited is renewed by Codex. The app holds no token for
//! it and imitates nothing: the request is Codex's own.
//!
//! When an account is due is decided in [`quota_station_core::codex::wake`];
//! here a thread asks every 30 seconds, runs the accounts due one after
//! another, never while Codex switches or signs in (the same `busy`), and
//! records each run without the model's answer. Codex's leftovers in an
//! account's folder (its databases and logs) go once the run ends.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use quota_station_core::codex::accounts::{Place, Store};
use quota_station_core::codex::platform::System;
use quota_station_core::codex::wake::{self, Book, Login, Mode, Outcome, Plan, Run, Seen, Why};
use quota_station_core::codex::{self, procs};
use quota_station_core::local_time;
use quota_station_core::os;
use quota_station_core::quota;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::codex::rfc;
use crate::codex_accounts::{chatgpt_number, login_of};
use crate::reader;
use crate::state::{AppState, Inner};

/// How often the thread asks which account is due.
const TICK: Duration = Duration::from_secs(30);
/// How long one wake-up may take before it is stopped.
const RUN_WAIT: Duration = Duration::from_secs(180);
/// What is sent.
const PROMPT: &str = "hi";
/// What an account's folder keeps after a run: its sign-in and its config.
const KEEP: [&str; 2] = [codex::AUTH, codex::CONFIG];

/// The settings the window sends.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerIn {
    pub on: bool,
    pub mode: Mode,
    pub at: String,
    pub keep_alive: bool,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerView {
    pub on: bool,
    pub mode: Mode,
    pub at: String,
    pub keep_alive: bool,
    pub ids: Vec<String>,
    /// When the next daily wake-up is due; `None` in the reset mode or off.
    pub next: Option<String>,
    /// The account being woken now.
    pub running: Option<String>,
    /// Newest first.
    pub runs: Vec<RunView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub at: String,
    pub id: String,
    pub why: Why,
    pub result: Outcome,
    /// `started`: when the window it opened resets, once a quota read shows it.
    pub resets_at: Option<String>,
    pub detail: Option<String>,
}

/// The quota row of `chatgpt:<n>`.
fn row_of(inner: &Inner, number: u32) -> Option<&quota::Provider> {
    let key = format!("{}{number}", quota::codex::KEPT_KEY);
    inner
        .codex_quota
        .as_ref()?
        .providers
        .iter()
        .find(|p| p.provider == "codex" && p.account_keys.contains(&key))
}

fn window<'a>(row: Option<&'a quota::Provider>, id: &str) -> Option<&'a quota::Window> {
    row?.windows.iter().find(|w| w.id == id)
}

fn parse(at: Option<&String>) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(at?, &Rfc3339).ok()
}

/// What the app knows of each ChatGPT account it keeps now.
fn seen(inner: &Inner) -> Vec<Seen> {
    inner
        .codex
        .accounts
        .iter()
        .map(|a| {
            let n = a.entry.number;
            let row = row_of(inner, n);
            let five = window(row, "five_hour");
            Seen {
                id: format!("{}{n}", codex::accounts::CHATGPT_ID),
                login: match login_of(inner, n, a.place) {
                    "relogin" => Login::Relogin,
                    "idle" => Login::Idle,
                    _ => Login::Ok,
                },
                waiting: a.place == Place::Folder,
                five_used: five.and_then(|w| w.percent_used),
                five_resets: parse(five.and_then(|w| w.resets_at.as_ref())),
                week_left: window(row, "weekly").and_then(|w| w.percent_remaining),
            }
        })
        .collect()
}

pub fn view(inner: &Inner) -> TriggerView {
    let book = &inner.codex.wake;
    let s = &book.settings;
    let now = OffsetDateTime::now_utc();
    let runs = book
        .runs
        .iter()
        .take(20)
        .filter_map(|r| {
            // The window a run opened, once a read shows one reset after it.
            let resets_at = (r.result == Outcome::Started)
                .then(|| {
                    let five = window(row_of(inner, chatgpt_number(&r.id)?), "five_hour")?;
                    let at = parse(five.resets_at.as_ref())?.unix_timestamp();
                    (r.at < at && at <= r.at + 5 * 3600 + 60).then(|| rfc(at)).flatten()
                })
                .flatten();
            Some(RunView {
                at: rfc(r.at)?,
                id: r.id.clone(),
                why: r.why,
                result: r.result,
                resets_at,
                detail: r.detail.clone(),
            })
        })
        .collect();
    TriggerView {
        on: s.on,
        mode: s.mode,
        at: s.at.clone(),
        keep_alive: s.keep_alive,
        ids: s.ids.clone(),
        next: wake::next(s, &book.runs, now, local_time::offset_at(now))
            .and_then(|t| rfc(t.unix_timestamp())),
        running: inner.codex.waking.clone(),
        runs,
    }
}

// ===== The thread =====

pub fn spawn(app: AppHandle, state: AppState) {
    {
        let mut inner = state.lock();
        let book = Book::load(&inner.app_dir());
        inner.codex.wake = book;
    }
    thread::Builder::new()
        .name("codex-wake".into())
        .spawn(move || loop {
            thread::sleep(TICK);
            let plans = {
                let inner = state.lock();
                let book = &inner.codex.wake;
                let now = OffsetDateTime::now_utc();
                wake::due(
                    &book.settings,
                    &book.runs,
                    &seen(&inner),
                    now,
                    local_time::offset_at(now),
                )
            };
            if !plans.is_empty() {
                run(&app, &state, plans);
            }
        })
        .expect("Codex wake-up thread starts");
}

/// Where `id` runs: the Codex folder for the account Codex uses (left as
/// it is afterwards), else its own folder (tidied afterwards).
fn target(inner: &Inner, id: &str) -> Option<(PathBuf, bool)> {
    let n = chatgpt_number(id)?;
    let a = inner.codex.accounts.iter().find(|a| a.entry.number == n)?;
    match a.place {
        Place::Home => Some((inner.codex_home(), false)),
        Place::Folder => {
            let (home, dir) = (inner.codex_home(), inner.app_dir());
            let platform = System::default();
            Some((Store::new(&dir, &home, &platform).folder(n), true))
        }
        Place::Missing => None,
    }
}

/// Runs `plans` one after another and records each. A wake-up that finds
/// Codex busy is left for the next tick, unrecorded.
fn run(app: &AppHandle, state: &AppState, plans: Vec<Plan>) {
    let user_home = state.lock().home.clone();
    let cli = procs::discover(&user_home);
    for p in plans {
        let at = OffsetDateTime::now_utc().unix_timestamp();
        let (result, detail) = match p.skip {
            Some(skip) => (skip, None),
            None => {
                let place = {
                    let mut inner = state.lock();
                    if inner.codex.busy.is_some() || inner.codex.signing.is_some() {
                        continue;
                    }
                    let place = target(&inner, &p.id);
                    if place.is_some() && cli.is_some() {
                        inner.codex.busy = Some("waking");
                        inner.codex.waking = Some(p.id.clone());
                    }
                    place
                };
                match (place, &cli) {
                    (None, _) => (Outcome::Relogin, None),
                    (Some(_), None) => (Outcome::Failed, Some("codex not found".into())),
                    (Some((home, tidy)), Some(cli)) => {
                        reader::publish(app, state);
                        let out = wake_one(cli, &home, tidy);
                        let mut inner = state.lock();
                        inner.codex.busy = None;
                        inner.codex.waking = None;
                        out
                    }
                }
            }
        };
        let saved = {
            let mut inner = state.lock();
            inner.codex.wake.record(Run { at, id: p.id.clone(), why: p.why, result, detail });
            let dir = inner.app_dir();
            inner.codex.wake.save(&dir)
        };
        if let Err(e) = saved {
            eprintln!("quota-station: the wake-up's record was not saved: {e}");
        }
        reader::publish(app, state);
    }
    state.request_read();
}

/// One wake-up: `codex exec` with `home` as `CODEX_HOME`, the Mac's own
/// config and rules left out (so no MCP server, hook or custom provider
/// runs), read-only, nothing kept as a session. With `tidy`, what Codex
/// left in the folder besides the sign-in goes afterwards.
fn wake_one(cli: &Path, home: &Path, tidy: bool) -> (Outcome, Option<String>) {
    let before = entries(home);
    let out = exec(cli, home);
    if tidy {
        for name in entries(home).difference(&before) {
            if KEEP.contains(&name.as_str()) {
                continue;
            }
            // A database Codex still holds for a moment is tried again
            // on Windows; what stays, stays.
            let _ = os::remove_retrying(&home.join(name));
        }
    }
    out
}

fn entries(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

fn exec(cli: &Path, home: &Path) -> (Outcome, Option<String>) {
    let mut child = match os::no_window(Command::new(cli).args([
        "exec",
        "--skip-git-repo-check",
        "--ephemeral",
        "--ignore-user-config",
        "--ignore-rules",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "--json",
        PROMPT,
    ]))
    .env("CODEX_HOME", home)
    .current_dir(std::env::temp_dir())
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    {
        Ok(c) => c,
        Err(e) => return (Outcome::Failed, Some(e.to_string())),
    };
    // Read as it comes, so a full pipe never holds Codex up.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut text);
            }
            text
        })
    };
    let events = drain(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let errors = drain(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < RUN_WAIT => thread::sleep(Duration::from_millis(250)),
            _ => {
                let _ = os::kill_tree(&mut child);
                let _ = child.wait();
                break None;
            }
        }
    };
    let events = events.join().unwrap_or_default();
    let errors = errors.join().unwrap_or_default();
    match status {
        None => (Outcome::Failed, Some("timeout".into())),
        Some(s) => wake::outcome(s.success(), &events, &errors),
    }
}

/// "Tự đánh thức": its settings, kept by the app. Refused with `at` or `ids`.
#[tauri::command]
pub fn codex_trigger_set(
    app: AppHandle,
    state: State<'_, AppState>,
    trigger: TriggerIn,
) -> Result<(), String> {
    let settings = wake::Settings {
        on: trigger.on,
        mode: trigger.mode,
        at: trigger.at,
        keep_alive: trigger.keep_alive,
        ids: trigger.ids,
    };
    wake::check(&settings).map_err(str::to_string)?;
    {
        let mut inner = state.lock();
        let mut book = inner.codex.wake.clone();
        book.settings = settings;
        book.save(&inner.app_dir()).map_err(|e| e.to_string())?;
        inner.codex.wake = book;
    }
    reader::publish(&app, &state);
    Ok(())
}

/// "Chạy ngay": wakes the chosen accounts now, one after another.
#[tauri::command]
pub fn codex_trigger_now(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    let plans = {
        let inner = shared.lock();
        if inner.codex.busy.is_some() || inner.codex.signing.is_some() {
            return Err("busy".into());
        }
        let book = &inner.codex.wake;
        wake::now(&book.settings, &book.runs, &seen(&inner), OffsetDateTime::now_utc())
    };
    thread::Builder::new()
        .name("codex-wake-now".into())
        .spawn(move || run(&app, &shared, plans))
        .map_err(|e| e.to_string())?;
    Ok(())
}

// The fake `codex` is a shell script.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A temporary folder with a fake `codex` running `body`.
    fn fake(name: &str, body: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("qs-codex-wake-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("account");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join(codex::AUTH), "{}").unwrap();
        let script = root.join("codex");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        (script, home)
    }

    #[test]
    fn a_wake_up_runs_codex_exec_in_the_accounts_folder_and_leaves_only_its_sign_in() {
        let (cli, home) = fake(
            "ok",
            "echo \"$@\" > \"$CODEX_HOME/../args\"; echo \"$CODEX_HOME\" > \"$CODEX_HOME/../home\"; \
             touch \"$CODEX_HOME/state_5.sqlite\"; mkdir -p \"$CODEX_HOME/shell_snapshots\"; \
             echo '{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"Hello there\"}}'; \
             echo '{\"type\":\"turn.completed\"}'",
        );
        let (result, detail) = wake_one(&cli, &home, true);
        assert_eq!((result, detail), (Outcome::Started, None));
        let root = home.parent().unwrap();
        let args = std::fs::read_to_string(root.join("args")).unwrap();
        assert!(
            args.starts_with("exec --skip-git-repo-check --ephemeral --ignore-user-config"),
            "{args}"
        );
        assert!(args.trim_end().ends_with("--json hi"), "{args}");
        assert_eq!(
            std::fs::read_to_string(root.join("home")).unwrap().trim(),
            home.to_str().unwrap()
        );
        assert_eq!(entries(&home), BTreeSet::from([codex::AUTH.to_string()]));
    }

    #[test]
    fn a_refused_sign_in_is_told_from_a_wake_up_and_the_codex_folder_in_use_is_left_as_is() {
        let (cli, home) = fake(
            "refused",
            "echo '{\"type\":\"turn.failed\",\"error\":{\"message\":\"unexpected status 401 Unauthorized, url: https://chatgpt.com/x\"}}'; exit 1",
        );
        assert_eq!(wake_one(&cli, &home, true).0, Outcome::Relogin);
        // The Codex folder of the account in use is left as Codex leaves it.
        let (cli, home) = fake(
            "home",
            "touch \"$CODEX_HOME/history.jsonl\"; echo '{\"type\":\"turn.completed\"}'",
        );
        assert_eq!(wake_one(&cli, &home, false).0, Outcome::Started);
        assert!(home.join("history.jsonl").exists());
    }
}
