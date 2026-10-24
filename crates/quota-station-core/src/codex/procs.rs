// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2025 Jason Young (CC Switch)
// SPDX-License-Identifier: MIT

//! Codex on this Mac as the process table shows it: `codex` sessions in a
//! terminal, Codex App, the shared background process (`codex app-server
//! --managed-daemon`), and whether CC Switch runs. Only `ps` is read. The
//! one thing run is Codex's own `codex app-server daemon restart`, after
//! the owner confirmed it.
//!
//! Telling the daemon from the other app-servers by its command line is
//! ported from CC Switch (MIT, © 2025 Jason Young), commit 79e754ab,
//! `src-tauri/src/services/provider/codex_client_catalog.rs`
//! (`app_server_role`, `after_codex_executable`, `run_restart`). See `NOTICE`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::Serialize;

use crate::runner;

/// The daemon finishes its running work first, up to its 60-second grace,
/// then starts again; CC Switch allows 75 seconds on top of the grace.
pub const RESTART_TIMEOUT: Duration = Duration::from_secs(60 + 75);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Procs {
    /// `codex` running in a terminal.
    pub sessions: u32,
    /// Codex App (the ChatGPT or Codex app's main process).
    pub app: bool,
    pub daemon: bool,
    pub cc_switch: bool,
}

/// Reads `ps -ww -Ao tty=,command=`; `None` when it cannot run.
pub fn read() -> Option<Procs> {
    let out = Command::new("/bin/ps")
        .args(["-ww", "-Ao", "tty=,command="])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(classify(&String::from_utf8_lossy(&out.stdout)))
}

/// The part of a command line after the executable `codex` (`codex` or
/// `…/codex`); `ps` joins the words with spaces, so the name is looked for.
fn after_codex_executable(command: &str) -> Option<&str> {
    const NAME: &str = "codex";
    let mut from = 0;
    while let Some(found) = command[from..].find(NAME) {
        let start = from + found;
        let end = start + NAME.len();
        let rest = &command[end..];
        if (start == 0 || command[..start].ends_with('/'))
            && (rest.is_empty() || rest.starts_with(' '))
        {
            return Some(rest);
        }
        from = end;
    }
    None
}

/// The first word after the executable that is not an option.
fn subcommand(rest: &str) -> (Option<&str>, Vec<&str>) {
    let mut words = rest.split_whitespace();
    loop {
        match words.next() {
            None => return (None, Vec::new()),
            Some("-c") | Some("--config") => {
                words.next();
            }
            Some(flag) if flag.starts_with('-') => {}
            Some(word) => return (Some(word), words.collect()),
        }
    }
}

/// Subcommands that are not a session someone works in.
const NOT_SESSIONS: &[&str] = &["app-server", "mcp-server", "login", "logout", "mcp", "completion"];

pub fn classify(table: &str) -> Procs {
    let mut p = Procs::default();
    for line in table.lines() {
        let line = line.trim_start();
        let Some((tty, command)) = line.split_once(char::is_whitespace) else { continue };
        let command = command.trim_start();
        if command.contains("/CC Switch.app/Contents/MacOS/")
            || command.split(' ').next().is_some_and(|w| w.ends_with("/cc-switch"))
        {
            p.cc_switch = true;
            continue;
        }
        if ["/Codex.app/Contents/MacOS/", "/ChatGPT.app/Contents/MacOS/"]
            .iter()
            .any(|m| command.contains(m))
        {
            p.app = true;
            continue;
        }
        // `node …/bin/codex` starts the native `codex`, which is counted.
        if command.split(' ').next().is_some_and(|w| w == "node" || w.ends_with("/node")) {
            continue;
        }
        let Some(rest) = after_codex_executable(command) else { continue };
        let (sub, args) = subcommand(rest);
        if sub == Some("app-server") {
            if args.contains(&"--managed-daemon")
                && !matches!(args.first(), Some(&"daemon") | Some(&"proxy"))
            {
                p.daemon = true;
            }
            continue;
        }
        let terminal = tty != "??" && tty != "-";
        if terminal && !sub.is_some_and(|s| NOT_SESSIONS.contains(&s)) {
            p.sessions += 1;
        }
    }
    p
}

/// Where `codex` is usually installed, in the order the app tries them.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![home.join(".local/bin/codex")];
    out.extend(
        runner::bin_dirs(home)
            .into_iter()
            .map(|d| d.join("codex"))
            .filter(|p| !p.starts_with(home.join(".local/bin"))),
    );
    out.push(PathBuf::from("/Applications/Codex.app/Contents/Resources/codex"));
    out.dedup();
    out
}

pub fn discover(home: &Path) -> Option<PathBuf> {
    candidates(home).into_iter().find(|p| p.is_file())
}

/// `codex --version`, as printed (`codex-cli 0.163.0`).
pub fn version(codex: &Path) -> Option<String> {
    let out = runner::run(codex, &["--version".into()], Duration::from_secs(10)).ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let text = text.as_str();
    (!text.is_empty()).then(|| text.rsplit(' ').next().unwrap_or(text).to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Restart {
    Restarted,
    /// No daemon ran: nothing to restart.
    NotRunning,
    NoCodex,
    Failed {
        detail: String,
    },
}

/// Runs `codex app-server daemon restart` for the Codex home the app edits.
pub fn restart_daemon(codex: Option<&Path>, codex_home: &Path, running: bool) -> Restart {
    if !running {
        return Restart::NotRunning;
    }
    let Some(codex) = codex else { return Restart::NoCodex };
    let args: Vec<String> =
        ["app-server", "daemon", "restart"].iter().map(|s| s.to_string()).collect();
    let env = [("CODEX_HOME", codex_home.as_os_str())];
    match runner::run_with(codex, &args, &env, None, RESTART_TIMEOUT) {
        Ok(out) if out.code == Some(0) => Restart::Restarted,
        Ok(out) => Restart::Failed {
            detail: format!("exit {}: {}", out.code.unwrap_or(-1), runner::first_line(&out.stderr)),
        },
        Err(e) => Restart::Failed { detail: e.to_string() },
    }
}
