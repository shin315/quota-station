// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2025 Jason Young (CC Switch)
// SPDX-License-Identifier: MIT

//! Codex on this Mac as the process table shows it: `codex` sessions in a
//! terminal, Codex App, the shared background process (`codex app-server
//! --managed-daemon`), and whether CC Switch runs. Only `ps` is read (on
//! Windows `sysinfo`'s table, [`crate::win::Tasks`]). The one thing run is
//! Codex's own `codex app-server daemon restart`, after the owner
//! confirmed it.
//!
//! On Windows a command line names `codex.exe` with `\`, Codex App is the
//! ChatGPT app (`ChatGPT.exe`, or `Codex.exe`, in its Store package), and a
//! process has no terminal to tell: every `codex` that is not one of
//! [`NOT_SESSIONS`] counts as a session.
//!
//! Telling the daemon from the other app-servers by its command line is
//! ported from CC Switch (MIT, © 2025 Jason Young), commit 79e754ab,
//! `src-tauri/src/services/provider/codex_client_catalog.rs`
//! (`app_server_role`, `after_codex_executable`, `run_restart`). See `NOTICE`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::os::{Process, ProcessList};
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

/// Reads the process table; `None` when it cannot be read.
pub fn read() -> Option<Procs> {
    read_from(&crate::os::processes())
}

/// [`read`] from `list`.
pub fn read_from(list: &dyn ProcessList) -> Option<Procs> {
    Some(classify_all(&list.all()?))
}

/// The part of a command line after the executable `codex` (`codex` or
/// `…/codex`; on Windows `codex.exe` or `…\codex.exe`); `ps` joins the
/// words with spaces, so the name is looked for.
fn after_codex_executable(command: &str) -> Option<&str> {
    const NAME: &str = "codex";
    let mut from = 0;
    while let Some(found) = command[from..].find(NAME) {
        let start = from + found;
        let end = start + NAME.len();
        let rest = &command[end..];
        let rest = match rest.get(..4) {
            Some(ext) if ext.eq_ignore_ascii_case(".exe") => &rest[4..],
            _ => rest,
        };
        if (start == 0 || command[..start].ends_with('/') || command[..start].ends_with('\\'))
            && (rest.is_empty() || rest.starts_with(' '))
        {
            return Some(rest);
        }
        from = end;
    }
    None
}

/// Whether the command line starts with Node, which starts the native
/// `codex` that is counted itself.
fn is_node(command: &str) -> bool {
    let first = command.split(' ').next().unwrap_or("");
    first == "node"
        || first.ends_with("/node")
        || command
            .to_ascii_lowercase()
            .split(".exe")
            .next()
            .is_some_and(|program| program == "node" || program.ends_with("\\node"))
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

/// [`classify_all`] of a `ps -ww -Ao tty=,command=` table, whose lines
/// show their terminal.
pub fn classify(table: &str) -> Procs {
    classify_in(&crate::os::tty_table(table), true)
}

pub fn classify_all(processes: &[Process]) -> Procs {
    classify_in(processes, cfg!(unix))
}

/// [`classify_all`]; a session needs a terminal when `needs_tty`, which
/// a Windows process never shows.
pub fn classify_in(processes: &[Process], needs_tty: bool) -> Procs {
    let mut p = Procs::default();
    for process in processes {
        let command = process.command.as_str();
        if command.contains("/CC Switch.app/Contents/MacOS/")
            || command.split(' ').next().is_some_and(|w| w.ends_with("/cc-switch"))
            || command.contains("\\CC Switch.exe")
            || command.to_ascii_lowercase().contains("\\cc-switch.exe")
        {
            p.cc_switch = true;
            continue;
        }
        if [
            "/Codex.app/Contents/MacOS/",
            "/ChatGPT.app/Contents/MacOS/",
            "\\ChatGPT.exe",
            "\\Codex.exe",
        ]
        .iter()
        .any(|m| command.contains(m))
        {
            p.app = true;
            continue;
        }
        // `node …/bin/codex` starts the native `codex`, which is counted.
        if is_node(command) {
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
        if (process.tty.is_some() || !needs_tty) && !sub.is_some_and(|s| NOT_SESSIONS.contains(&s))
        {
            p.sessions += 1;
        }
    }
    p
}

/// Where `codex` is usually installed, in the order the app tries them.
/// On Windows `codex.exe` and npm's `codex.cmd` in the same folders, then
/// on the `PATH`, as `where codex` finds them.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![home.join(".local/bin")];
    dirs.extend(runner::bin_dirs(home).into_iter().filter(|d| *d != home.join(".local/bin")));
    if cfg!(windows) {
        dirs.extend(std::env::var_os("PATH").iter().flat_map(std::env::split_paths));
    }
    let mut out: Vec<PathBuf> =
        dirs.iter().flat_map(|d| runner::program_files(d, "codex")).collect();
    if cfg!(not(windows)) {
        out.push(PathBuf::from("/Applications/Codex.app/Contents/Resources/codex"));
    }
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
