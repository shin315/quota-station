// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the core asks of the operating system, behind four seams, beside
//! [`crate::login_item::LoginItems`] and [`crate::alerts::Notifier`]:
//!
//! - [`SecretStore`]: generic passwords by service and account (macOS:
//!   the login Keychain through `/usr/bin/security`,
//!   [`crate::claude::keychain::Security`]);
//! - [`Scheduler`]: the rotation run every few minutes while the app is
//!   closed, and the older switchers that must not run beside it (macOS:
//!   launchd through `/bin/launchctl`, [`crate::claude::agent::Launchd`]);
//! - [`ProcessList`]: which processes run, their command lines and the
//!   ports they listen on (macOS: `kill(pid, 0)`, `ps` and `lsof`, [`Ps`]);
//! - [`Opener`]: a URL or a file handed to the program the owner chose for
//!   it (macOS: `/usr/bin/open`, [`SystemOpener`]).
//!
//! The app gets each from its `system()` constructor; the tests give their
//! own, kept in memory, so none of them runs a system tool.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::claude::agent::AgentError;
use crate::claude::keychain::KeychainError;
use crate::runner;

/// Generic passwords: a value per service and account.
pub trait SecretStore: Send + Sync + std::fmt::Debug {
    /// The item's value, or `None` when there is no such item, waiting up
    /// to `wait` for the system (macOS may ask the owner first).
    fn get_within(
        &self,
        service: &str,
        account: &str,
        wait: Duration,
    ) -> Result<Option<String>, KeychainError>;
    /// The item's value within the store's own time limit.
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError>;
    /// Whether the item is there, without reading its value.
    fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError>;
    /// Adds or replaces the item. On a failure the item that was there
    /// stays as it was.
    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError>;
    /// Removes the item; one that is not there counts as removed.
    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError>;
}

/// The job that runs `<exe> claude rotate` every few minutes, and the
/// older switchers that must never run beside it.
pub trait Scheduler: Send + Sync + std::fmt::Debug {
    /// Whether the job is installed and loaded; `None` when that cannot be
    /// told.
    fn loaded(&self) -> Option<bool>;
    /// The older switcher still running, if any, or of which the system
    /// cannot tell: nothing switches or refreshes beside one.
    fn legacy_loaded(&self) -> Option<&'static str>;
    /// Installs and starts the job for `exe`. Refused while an older
    /// switcher runs.
    fn install(&self, home: &Path, exe: &Path) -> Result<(), AgentError>;
    /// Stops the job and removes it.
    fn remove(&self, home: &Path) -> Result<(), AgentError>;
}

/// One running process, as [`ProcessList`] lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    /// Its terminal; `None` for one started without (an app, a daemon).
    pub tty: Option<String>,
    /// The command line, its words joined by spaces.
    pub command: String,
}

/// The processes that run.
pub trait ProcessList: Send + Sync + std::fmt::Debug {
    /// Whether `pid` runs, even when it belongs to another user.
    fn alive(&self, pid: i32) -> bool;
    /// Every process, with its terminal; `None` when they cannot be listed.
    fn all(&self) -> Option<Vec<Process>>;
    /// This user's processes, within `wait`; `None` when they cannot be
    /// listed.
    fn own(&self, wait: Duration) -> Option<Vec<Process>>;
    /// The TCP ports `pid` listens on, within `wait`; `None` when that
    /// cannot be told.
    fn listening(&self, pid: u32, wait: Duration) -> Option<Vec<u16>>;
}

/// Hands a URL (or a file) to the program the owner chose for it.
pub trait Opener {
    fn open(&self, target: &str) -> Result<(), String>;
}

/// This system's secret store.
pub fn secret_store() -> crate::claude::keychain::Security {
    crate::claude::keychain::Security::system()
}

/// This system's scheduler.
pub fn scheduler() -> crate::claude::agent::Launchd {
    crate::claude::agent::Launchd::system()
}

/// This system's process list.
pub fn processes() -> Ps {
    Ps::system()
}

/// This system's opener.
pub fn opener() -> SystemOpener {
    SystemOpener
}

/// macOS's `ps`, `lsof` and `kill(pid, 0)`.
#[derive(Debug, Clone)]
pub struct Ps {
    pub ps: PathBuf,
    pub lsof: PathBuf,
}

impl Ps {
    pub fn system() -> Ps {
        Ps { ps: PathBuf::from("/bin/ps"), lsof: PathBuf::from("/usr/sbin/lsof") }
    }
}

impl ProcessList for Ps {
    fn alive(&self, pid: i32) -> bool {
        if pid <= 1 {
            return false;
        }
        // SAFETY: signal 0 sends nothing; it only checks that the pid exists.
        let r = unsafe { libc::kill(pid, 0) };
        r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    /// `ps -ww -Ao tty=,command=`, which shows `??` for no terminal.
    fn all(&self) -> Option<Vec<Process>> {
        let out = std::process::Command::new(&self.ps)
            .args(["-ww", "-Ao", "tty=,command="])
            .env("LC_ALL", "C")
            .stdin(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        Some(tty_table(&String::from_utf8_lossy(&out.stdout)))
    }

    /// `ps -x -u <euid> -o pid=,command=`.
    fn own(&self, wait: Duration) -> Option<Vec<Process>> {
        // SAFETY: `geteuid` only reads the process's user id.
        let uid = unsafe { libc::geteuid() }.to_string();
        let args: Vec<String> =
            ["-x", "-u", &uid, "-o", "pid=,command="].iter().map(|s| s.to_string()).collect();
        let out = runner::run(&self.ps, &args, wait).ok().filter(|o| o.code == Some(0))?;
        Some(pid_table(&String::from_utf8_lossy(&out.stdout)))
    }

    /// `lsof -nP -a -p PID -iTCP -sTCP:LISTEN`.
    fn listening(&self, pid: u32, wait: Duration) -> Option<Vec<u16>> {
        let args: Vec<String> = ["-nP", "-a", "-p", &pid.to_string(), "-iTCP", "-sTCP:LISTEN"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = runner::run(&self.lsof, &args, wait).ok().filter(|o| o.code == Some(0))?;
        Some(listening_ports(&String::from_utf8_lossy(&out.stdout)))
    }
}

/// `ps`'s `tty=,command=` lines; `??` and `-` are no terminal.
pub fn tty_table(table: &str) -> Vec<Process> {
    table
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (tty, command) = line.split_once(char::is_whitespace)?;
            let tty = (tty != "??" && tty != "-").then(|| tty.to_string());
            Some(Process { pid: 0, tty, command: command.trim_start().to_string() })
        })
        .collect()
}

/// `ps`'s `pid=,command=` lines; a line that does not start with a pid
/// (a header) or has no command is left out.
pub fn pid_table(table: &str) -> Vec<Process> {
    table
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid: u32 = pid.parse().ok()?;
            let command = command.trim();
            (!command.is_empty()).then(|| Process { pid, tty: None, command: command.into() })
        })
        .collect()
}

/// The ports `lsof` shows listening.
pub fn listening_ports(lsof: &str) -> Vec<u16> {
    let mut out = Vec::new();
    for line in lsof.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = words.iter().position(|w| *w == "TCP") else { continue };
        let (Some(addr), Some(state)) = (words.get(at + 1), words.get(at + 2)) else { continue };
        if *state != "(LISTEN)" {
            continue;
        }
        let port = addr.rsplit(':').next().and_then(|p| p.parse::<u16>().ok());
        if let Some(p) = port.filter(|p| *p > 0) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// The default browser (or the app for a file), through `open`.
pub struct SystemOpener;

impl Opener for SystemOpener {
    fn open(&self, target: &str) -> Result<(), String> {
        let out =
            runner::run(Path::new("/usr/bin/open"), &[target.to_string()], Duration::from_secs(15))
                .map_err(|e| e.to_string())?;
        if out.code == Some(0) {
            Ok(())
        } else {
            Err(runner::first_line(&out.stderr))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_tables_split_into_processes() {
        let tty = tty_table("ttys001 codex --resume\n??       /Applications/Codex.app/x\n");
        assert_eq!(tty[0].tty.as_deref(), Some("ttys001"));
        assert_eq!(tty[0].command, "codex --resume");
        assert_eq!(tty[1].tty, None);
        let pid = pid_table("  PID COMMAND\n 4242 /usr/local/bin/agy --resume\n 7 \n");
        assert_eq!(
            pid,
            [Process { pid: 4242, tty: None, command: "/usr/local/bin/agy --resume".into() }]
        );
    }

    #[test]
    fn lsof_lines_give_the_listening_ports() {
        let out = "COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME\n\
                   language 202 me 10u IPv4 0x1 0t0 TCP 127.0.0.1:53210 (LISTEN)\n\
                   language 202 me 11u IPv6 0x2 0t0 TCP [::1]:53211 (LISTEN)\n\
                   language 202 me 12u IPv4 0x3 0t0 TCP 127.0.0.1:53212->127.0.0.1:1 (ESTABLISHED)\n";
        assert_eq!(listening_ports(out), [53210, 53211]);
    }
}
