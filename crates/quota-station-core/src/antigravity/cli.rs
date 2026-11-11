// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! `quota-station antigravity …`: the Antigravity accounts the app logs in,
//! from a terminal, whether or not the app is open. `status` lists them with
//! their readings, `switch` runs the app's own switch ([`super::switch`]),
//! `rotate` is one run of the rotation ([`super::rotate`]) and `rotation`
//! turns its LaunchAgent on or off. No command prints a token; only
//! `status --emails` prints emails.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use time::OffsetDateTime;

use super::cloud_code::Hosts;
use super::credential::SharedLogin;
use super::desktop::Desktop;
use super::index::{self, AccountIndex, Standing};
use super::oauth::Endpoints;
use super::rotate::{self, Fresh, Schedule, Settings, World};
use super::store::Keychain;
use super::switch::{self, CloudCodeProbe, Deps, How, Limits, Outcome, Request};
use super::version::AntigravityApp;
use super::{export, listing, login, pace, quota};
use crate::app_dir::AppDir;
use crate::quota::semantics::AGY_SCOPES;
use crate::quota::{agy, iso, Response, Scope, Window};

/// The two scopes of an account's reading.
const GEMINI: &str = AGY_SCOPES[0];
const CLAUDE_GPT: &str = AGY_SCOPES[1];

pub const USAGE: &str = "\
usage: quota-station antigravity <command>

  status [--json] [--emails] [--refresh]
                        every account, its Gemini and Claude/GPT windows,
                        and the account in use
  switch N|EMAIL [--keep-running] [--json]
                        make account N Antigravity's login; a running
                        Antigravity.app is quit and opened again, unless
                        --keep-running (it may then write its own login back)
  rotate [--dry-run]    one run of the rotation (the LaunchAgent's)
  rotation on [--threshold PCT] [--cooldown MIN]
                        every 3 minutes, switch when the account in use has
                        PCT% of Gemini left or less (default 5), at most once
                        every MIN minutes (default 30)
  rotation off          stop it
  rotation status [--json]
";

/// After a switch: what `agy` sessions that were already running do.
pub const AGY_NOTE: &str = "note: agy sessions started before the switch stay on the old \
account, and write its login back at their next hourly token refresh; restart them to move them \
over";

/// What the commands run with.
pub struct Cli {
    pub app: AppDir,
    pub home: PathBuf,
    /// This program, for the LaunchAgent.
    pub exe: PathBuf,
    pub world: Box<dyn World>,
    pub schedule: Schedule,
}

impl Cli {
    /// The app's folder under `home`, and this system's programs.
    pub fn system(home: &Path) -> Cli {
        Cli {
            app: AppDir::new(crate::claude::app_dir(home)),
            home: home.to_path_buf(),
            exe: std::env::current_exe().unwrap_or_else(|_| "quota-station".into()),
            world: Box::new(System),
            schedule: Schedule::system(),
        }
    }
}

/// The system: Google, the Keychain, Antigravity.app and agy.
pub struct System;

fn now_ms() -> i64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

impl World for System {
    fn read(&self, app: &AppDir, fresh: Fresh, now: OffsetDateTime) -> Response {
        let tools = agy::Tools::system();
        let max_age = match fresh {
            Fresh::Cached => 120,
            Fresh::Due => 60,
            Fresh::All => 0,
        };
        let mut report = listing::read(&tools, app, max_age, now);
        if fresh == Fresh::Cached {
            return report;
        }
        let Ok((index, candidates)) = quota::candidates(app) else { return report };
        let shown = report
            .primary("agy")
            .and_then(|r| r.agy_accounts.as_ref())
            .and_then(|l| l.accounts.iter().find(|a| a.active))
            .map(|a| a.account as usize);
        let active = quota::active_id(&index, shown);
        let at = now_ms();
        let due: Vec<String> = candidates
            .iter()
            .filter(|c| !c.skip)
            .filter(|c| {
                fresh == Fresh::All
                    || Some(&c.id) == active.as_ref()
                    || c.read_at.is_none_or(|r| at - r >= pace::OTHERS_EVERY_MS)
            })
            .map(|c| c.id.clone())
            .collect();
        if due.is_empty() {
            return report;
        }
        let identity = AntigravityApp::installed().identity();
        let (endpoints, hosts, keychain) =
            (Endpoints::google(), Hosts::google(), Keychain::system());
        let login = login::Deps { app, endpoints: &endpoints, keychain: &keychain };
        let deps = quota::Deps { login: &login, hosts: &hosts, identity: &identity };
        quota::read_in_turn(&deps, &due, &now_ms, &mut |_, _| {});
        if let Some(row) = report.primary_mut("agy") {
            listing::attach(row, app, OffsetDateTime::now_utc());
        }
        report
    }

    fn switch(&self, app: &AppDir, id: &str, from: Option<String>, how: Option<How>) -> Outcome {
        let desktop = Desktop::system();
        // Quit and opened again when it runs, so it starts on the new login.
        let how =
            how.unwrap_or(if desktop.running() == Ok(false) { How::Plain } else { How::QuitFirst });
        let (endpoints, keychain) = (Endpoints::google(), Keychain::system());
        let shared = SharedLogin::system();
        let tools = agy::Tools::system();
        let deps = Deps {
            app,
            endpoints: &endpoints,
            keychain: &keychain,
            probe: Arc::new(CloudCodeProbe {
                hosts: Hosts::google(),
                identity: AntigravityApp::installed().identity(),
            }),
            shared: &shared,
            desktop: &desktop,
            agy: &tools,
        };
        let request = Request { id: id.to_string(), how, from };
        let now = || OffsetDateTime::now_utc().unix_timestamp();
        switch::switch_antigravity(&deps, &request, Limits::default(), &now).outcome
    }
}

/// Runs one command; the exit code.
pub fn run(cli: &Cli, args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let word = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    let has = |flag: &str| rest.iter().any(|a| a == flag);
    let fail = |err: &mut dyn Write, e: &dyn std::fmt::Display| {
        let _ = writeln!(err, "quota-station antigravity {word}: {e}");
        1
    };
    match word {
        "status" => {
            let fresh = if has("--refresh") { Fresh::All } else { Fresh::Cached };
            let now = OffsetDateTime::now_utc();
            let report = cli.world.read(&cli.app, fresh, now);
            let view = status(cli, &report, has("--emails"), now);
            if has("--json") {
                print_json(out, &view);
            } else {
                print_status(&view, out);
            }
            if view.status == "ok" || view.reason.as_deref() == Some("no_accounts") {
                0
            } else {
                1
            }
        }
        "switch" => {
            let Some(target) = rest.first().filter(|a| !a.starts_with("--")) else {
                return usage_error(err);
            };
            let index = match index::load(&cli.app) {
                Ok(index) => index,
                Err(e) => return fail(err, &e),
            };
            let Some(number) = find(&index, target) else {
                return fail(err, &format!("no account {target}"));
            };
            let id = index.accounts[number - 1].id.clone();
            let report = cli.world.read(&cli.app, Fresh::Cached, OffsetDateTime::now_utc());
            let from = switch::shown_active_id(&report, &index);
            if from.as_deref() == Some(id.as_str()) {
                let _ = writeln!(out, "already on account {number}");
                return 0;
            }
            let how = has("--keep-running").then_some(How::KeepRunning);
            let outcome = cli.world.switch(&cli.app, &id, from, how);
            if has("--json") {
                print_json(out, &outcome);
            } else {
                let _ = writeln!(out, "{}", rotate::outcome_text(&outcome));
            }
            if !matches!(outcome, Outcome::Failed { .. }) {
                let _ = writeln!(err, "{AGY_NOTE}");
            }
            match outcome {
                Outcome::Switched { .. } => 0,
                Outcome::Unconfirmed { .. } => 3,
                Outcome::Failed { .. } => 1,
            }
        }
        "rotate" => {
            let run =
                rotate::run(&cli.app, &*cli.world, OffsetDateTime::now_utc(), has("--dry-run"));
            let _ = write!(out, "{}", run.print());
            match run.outcome {
                Some(Outcome::Failed { .. }) => 1,
                _ => 0,
            }
        }
        "rotation" => rotation(cli, rest, out, err),
        "help" | "--help" | "-h" => {
            let _ = write!(out, "{USAGE}");
            0
        }
        _ => usage_error(err),
    }
}

fn rotation(cli: &Cli, rest: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let fail = |err: &mut dyn Write, e: &dyn std::fmt::Display| {
        let _ = writeln!(err, "quota-station antigravity rotation: {e}");
        1
    };
    match rest.first().map(String::as_str) {
        Some("on") => {
            let mut settings = rotate::load_settings(&cli.app);
            let flag = |name: &str| rest.iter().position(|a| a == name).map(|i| rest.get(i + 1));
            match flag("--threshold") {
                None => {}
                Some(Some(v)) if v.parse::<f64>().is_ok_and(|t| (0.0..100.0).contains(&t)) => {
                    settings.threshold = v.parse().expect("checked");
                }
                Some(_) => return fail(err, &"--threshold takes a percent from 0 to 99"),
            }
            match flag("--cooldown") {
                None => {}
                Some(Some(v)) if v.parse::<u32>().is_ok_and(|m| m <= 24 * 60) => {
                    settings.cooldown_minutes = v.parse().expect("checked");
                }
                Some(_) => return fail(err, &"--cooldown takes minutes, from 0 to 1440"),
            }
            if let Err(e) = rotate::save_settings(&cli.app, &settings) {
                return fail(err, &e);
            }
            match cli.schedule.install(&cli.home, &cli.exe) {
                Ok(()) => {
                    let _ = writeln!(out, "{}", settings_text(&settings));
                    0
                }
                Err(e) => fail(err, &e),
            }
        }
        Some("off") => match cli.schedule.remove(&cli.home) {
            Ok(()) => 0,
            Err(e) => fail(err, &e),
        },
        Some("status") => {
            let view = rotation_view(cli);
            if rest.iter().any(|a| a == "--json") {
                print_json(out, &view);
            } else {
                let on = match view.on {
                    Some(true) => "on",
                    Some(false) => "off",
                    None => "unknown",
                };
                let _ = writeln!(out, "rotation {on}: {}", settings_text(&view.settings));
                if let Some(at) = view.last_switch_at.as_deref() {
                    let _ = writeln!(out, "last switch {at}");
                }
            }
            0
        }
        _ => usage_error(err),
    }
}

fn settings_text(s: &Settings) -> String {
    format!(
        "switch at {}% of Gemini left or less, to an account with {} points more, at most once \
         every {} minutes",
        s.threshold, s.min_lead, s.cooldown_minutes
    )
}

fn usage_error(err: &mut dyn Write) -> i32 {
    let _ = write!(err, "{USAGE}");
    2
}

/// The account a word names: its number, or its email in any case.
fn find(index: &AccountIndex, word: &str) -> Option<usize> {
    if let Ok(n) = word.parse::<usize>() {
        return (n >= 1 && n <= index.accounts.len()).then_some(n);
    }
    let id = index.find_email(word)?.id.clone();
    index.accounts.iter().position(|a| a.id == id).map(|i| i + 1)
}

/// What `status` prints.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusOut {
    pub generated_at: String,
    /// `ok`, or `unavailable` with `reason`.
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The account in use, by number, and how it was told: `identity`,
    /// `reading` or `binding`.
    pub active: Option<u32>,
    pub active_basis: Option<String>,
    /// The live read of the account agy is on.
    pub live: LiveOut,
    pub accounts: Vec<AccountOut>,
    pub rotation: RotationOut,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveOut {
    pub status: String,
    pub refreshed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountOut {
    pub account: u32,
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub active: bool,
    /// `fresh` (at most 15 minutes old), `stale` or `no_data`.
    pub status: String,
    /// `needsLogin`, `needsVerification` or `forbidden`.
    pub standing: Option<&'static str>,
    pub age_seconds: Option<f64>,
    pub fetched_at: Option<String>,
    /// `gemini` and `claude_gpt`: what is left, the windows holding it there
    /// and when the latest of them resets.
    pub scopes: Vec<Scope>,
    /// Each window as read: `gemini_5h`, `gemini_weekly`, `claude_gpt_5h`…
    pub windows: Vec<Window>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationOut {
    /// Whether its LaunchAgent is loaded; `None` when that cannot be told.
    pub on: Option<bool>,
    #[serde(flatten)]
    pub settings: Settings,
    pub last_switch_at: Option<String>,
    pub last_run: Option<serde_json::Value>,
}

pub fn rotation_view(cli: &Cli) -> RotationOut {
    let last = switch::read_log(&cli.app).last().map(|l| l.at);
    RotationOut {
        on: cli.schedule.loaded(),
        settings: rotate::load_settings(&cli.app),
        last_switch_at: last.and_then(|at| OffsetDateTime::from_unix_timestamp(at).ok()).map(iso),
        last_run: rotate::last_run(&cli.app),
    }
}

/// The status of every account from `report` and the app's files.
pub fn status(cli: &Cli, report: &Response, emails: bool, now: OffsetDateTime) -> StatusOut {
    let row = report.primary("agy");
    let live = LiveOut {
        status: row.map_or("unavailable".into(), |r| r.state.status.clone()),
        refreshed_at: row.and_then(|r| r.state.refreshed_at.clone()),
        error: row.and_then(|r| r.state.error.clone()),
    };
    let rotation = rotation_view(cli);
    let index = index::load(&cli.app);
    let list = row.and_then(|r| r.agy_accounts.as_ref());
    let (status, reason) = match (&index, list) {
        (Err(_), _) => ("unavailable", Some("index_unreadable")),
        (Ok(i), _) if i.accounts.is_empty() => ("unavailable", Some("no_accounts")),
        (_, Some(l)) if l.status == "ok" => ("ok", None),
        (_, Some(l)) => ("unavailable", l.reason.as_deref().or(Some("unlisted"))),
        (_, None) => ("unavailable", Some("unlisted")),
    };
    let index = index.unwrap_or_default();
    let statuses = index::load_statuses(&cli.app);
    let accounts: Vec<AccountOut> = list
        .filter(|l| l.status == "ok")
        .map(|l| l.accounts.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|a| {
            let summary = index.accounts.get((a.account as usize).checked_sub(1)?)?;
            let windows = export::read(&cli.app, &summary.email)
                .and_then(|r| r.payload.get("quota_summary").filter(|s| s.is_object()).cloned())
                .map(|s| agy::quota_summary(&s))
                .unwrap_or_default();
            Some(AccountOut {
                account: a.account,
                label: summary.name.clone().filter(|n| !n.trim().is_empty()),
                email: emails.then(|| summary.email.clone()),
                active: a.active,
                status: a.status.clone(),
                standing: statuses.accounts.get(&summary.id).map(|s| match s {
                    Standing::NeedsLogin { .. } => "needsLogin",
                    Standing::NeedsVerification { .. } => "needsVerification",
                    Standing::Forbidden { .. } => "forbidden",
                }),
                age_seconds: a.age_seconds,
                fetched_at: a.fetched_at.clone(),
                scopes: a.scopes.clone(),
                windows,
            })
        })
        .collect();
    let active = accounts.iter().find(|a| a.active).map(|a| a.account);
    StatusOut {
        generated_at: iso(now),
        status: status.into(),
        reason: reason.map(String::from),
        active,
        active_basis: list.and_then(|l| l.active_basis.clone()).filter(|_| active.is_some()),
        live,
        accounts,
        rotation,
    }
}

fn print_status(view: &StatusOut, out: &mut dyn Write) {
    if view.status != "ok" {
        let why = view.reason.as_deref().unwrap_or("unavailable");
        let _ = match why {
            "no_accounts" => writeln!(out, "no Antigravity accounts yet: log one in from the app"),
            _ => writeln!(out, "the accounts cannot be listed ({why})"),
        };
        return;
    }
    let left = |a: &AccountOut, scope: &str| {
        let s = a.scopes.iter().find(|s| s.scope == scope);
        match s.map(|s| (s.status.as_str(), s.effective_percent_remaining)) {
            Some(("known", Some(l))) => format!("{l:>3.0}%"),
            Some(("reset_pending", _)) => "rset".into(),
            _ => "   -".into(),
        }
    };
    for a in &view.accounts {
        let age = a.age_seconds.map(|s| format!(" {}m", (s / 60.0).round())).unwrap_or_default();
        let _ = writeln!(
            out,
            "{} {:>2}  Gemini {}  Claude/GPT {}  {:<8} {}{}{}",
            if a.active { '*' } else { ' ' },
            a.account,
            left(a, GEMINI),
            left(a, CLAUDE_GPT),
            format!("{}{age}", a.status),
            a.standing.map(|s| format!("[{s}] ")).unwrap_or_default(),
            a.label.as_deref().unwrap_or(""),
            a.email.as_deref().map(|e| format!("  {e}")).unwrap_or_default(),
        );
        for w in &a.windows {
            let left = w.percent_remaining.map_or("-".to_string(), |p| format!("{p:.0}%"));
            let _ = writeln!(
                out,
                "       {:<20} {left:>5} left  resets {}",
                w.id,
                w.resets_at.as_deref().unwrap_or("-")
            );
        }
    }
}

fn print_json(out: &mut dyn Write, value: &impl Serialize) {
    let _ = writeln!(out, "{}", serde_json::to_string_pretty(value).expect("serializes"));
}
