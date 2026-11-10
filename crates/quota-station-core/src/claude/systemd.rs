// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation on Linux: a `systemd --user` timer that runs
//! `<exe> claude rotate` every [`agent::EVERY_SECONDS`], once soon after it
//! starts, whether or not anyone is logged in when lingering is on, as
//! macOS's LaunchAgent does ([`super::agent`]). Its two units are files in
//! `~/.config/systemd/user`; `systemctl --user` loads, starts, stops and
//! asks about them.
//!
//! Where no user manager answers (a container, or WSL without
//! `systemd=true` in `/etc/wsl.conf`), turning it on is refused with the
//! line that runs the rotation from `cron` instead.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::agent::{self, AgentError};
use crate::os::Scheduler;
use crate::runner;

/// The units' name, as launchd's label on macOS.
pub const UNIT: &str = agent::LABEL;
const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(20);

/// The folder of the user's own units.
pub fn unit_dir(home: &Path) -> PathBuf {
    home.join(".config/systemd/user")
}

pub fn service_path(home: &Path) -> PathBuf {
    unit_dir(home).join(format!("{UNIT}.service"))
}

pub fn timer_path(home: &Path) -> PathBuf {
    unit_dir(home).join(format!("{UNIT}.timer"))
}

/// `text` as one word of an `ExecStart=` line: quoted, with systemd's
/// specifiers (`%`) and variables (`$`) doubled.
fn exec_word(text: &str) -> String {
    let escaped =
        text.replace('\\', "\\\\").replace('"', "\\\"").replace('%', "%%").replace('$', "$$");
    format!("\"{escaped}\"")
}

/// The service: one run of the rotation, its output dropped as the
/// LaunchAgent drops it (the rotation writes its own log).
pub fn service(exe: &Path) -> String {
    format!(
        "[Unit]
Description=Quota Station: one run of the Claude accounts' rotation

[Service]
Type=oneshot
ExecStart={} claude rotate
StandardOutput=null
StandardError=null
",
        exec_word(&exe.display().to_string())
    )
}

/// The timer: the service every [`agent::EVERY_SECONDS`], the first time
/// soon after the timer starts (the LaunchAgent's `RunAtLoad`).
pub fn timer() -> String {
    format!(
        "[Unit]
Description=Quota Station: the Claude accounts' rotation every {} seconds

[Timer]
OnActiveSec=5
OnUnitActiveSec={}
AccuracySec=10

[Install]
WantedBy=timers.target
",
        agent::EVERY_SECONDS,
        agent::EVERY_SECONDS
    )
}

/// What `install` says where no user manager answers.
pub fn no_systemd(exe: &Path) -> String {
    format!(
        "systemd --user does not answer here, so the rotation cannot be turned on; \
         run it every three minutes from cron instead (crontab -e): \
         */3 * * * * '{}' claude rotate >/dev/null 2>&1",
        exe.display()
    )
}

/// `systemctl --user`: the app's [`Scheduler`] on Linux.
#[derive(Debug, Clone)]
pub struct Systemd {
    pub systemctl: PathBuf,
}

impl Systemd {
    pub fn system() -> Systemd {
        Systemd { systemctl: PathBuf::from("/usr/bin/systemctl") }
    }

    /// `systemctl --user args…`, its exit code and its words. A shell
    /// without `XDG_RUNTIME_DIR` (a `cron` job, `sudo -u`, some terminals of
    /// WSL) finds the user manager in `/run/user/<uid>` all the same.
    fn run(&self, args: &[&str]) -> Result<runner::Output, AgentError> {
        let mut words = vec!["--user".to_string()];
        words.extend(args.iter().map(|a| a.to_string()));
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| OsString::from(format!("/run/user/{}", uid())));
        runner::run_with(
            &self.systemctl,
            &words,
            &[("XDG_RUNTIME_DIR", &runtime), ("LC_ALL", std::ffi::OsStr::new("C"))],
            None,
            SYSTEMCTL_TIMEOUT,
        )
        .map_err(|e| AgentError::Io(format!("systemctl: {e}")))
    }

    /// Whether a user manager answers.
    fn answers(&self) -> bool {
        self.run(&["is-system-running"]).is_ok_and(|out| {
            let state = String::from_utf8_lossy(&out.stdout);
            // `degraded` (a unit failed) and `starting` still run timers.
            matches!(state.trim(), "running" | "degraded" | "starting" | "initializing")
        })
    }

    fn checked(&self, args: &[&str]) -> Result<(), AgentError> {
        let out = self.run(args)?;
        match out.code {
            Some(0) => Ok(()),
            _ => Err(AgentError::Io(format!("systemctl: {}", runner::first_line(&out.stderr)))),
        }
    }
}

fn uid() -> u32 {
    // SAFETY: `getuid` only reads the process's user id.
    unsafe { libc::getuid() }
}

impl Scheduler for Systemd {
    /// `systemctl --user is-active` of the timer: `active` is loaded, any
    /// other answer is not; no answer at all (no user manager) cannot be
    /// told.
    fn loaded(&self) -> Option<bool> {
        let out = self.run(&["is-active", &format!("{UNIT}.timer")]).ok()?;
        let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
        match state.as_str() {
            "active" => Some(true),
            "inactive" | "failed" | "activating" | "deactivating" => Some(false),
            _ => None,
        }
    }

    /// The older switchers are macOS's LaunchAgents: none runs here.
    fn legacy_loaded(&self) -> Option<&'static str> {
        None
    }

    /// Writes the two units, has the user manager read them and starts the
    /// timer, which starts again with the user manager.
    fn install(&self, home: &Path, exe: &Path) -> Result<(), AgentError> {
        if !self.answers() {
            return Err(AgentError::Io(no_systemd(exe)));
        }
        let io = |e: std::io::Error| AgentError::Io(e.to_string());
        std::fs::create_dir_all(unit_dir(home)).map_err(io)?;
        std::fs::write(service_path(home), service(exe)).map_err(io)?;
        std::fs::write(timer_path(home), timer()).map_err(io)?;
        self.checked(&["daemon-reload"])?;
        // A timer already running (another copy's) starts again on the new
        // units.
        self.checked(&["enable", &format!("{UNIT}.timer")])?;
        self.checked(&["restart", &format!("{UNIT}.timer")])
    }

    /// Stops the timer and removes the units.
    fn remove(&self, home: &Path) -> Result<(), AgentError> {
        let timer = format!("{UNIT}.timer");
        if self.answers() {
            if self.loaded() != Some(false) {
                self.checked(&["stop", &timer])?;
            }
            // Not enabled is fine: only the link is wanted gone.
            let _ = self.run(&["disable", &timer]);
        }
        for file in [timer_path(home), service_path(home)] {
            match std::fs::remove_file(file) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(AgentError::Io(e.to_string()))
                }
                _ => {}
            }
        }
        if self.answers() {
            let _ = self.run(&["daemon-reload"]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_service_runs_the_rotation_of_this_program() {
        let text = service(Path::new("/home/lan/.local/bin/quota-station"));
        assert!(
            text.contains("\nExecStart=\"/home/lan/.local/bin/quota-station\" claude rotate\n"),
            "{text}"
        );
        assert!(text.contains("\nType=oneshot\n"));
        let odd = service(Path::new("/home/l an/100%/$x/\"q\"/qs"));
        assert!(
            odd.contains("ExecStart=\"/home/l an/100%%/$$x/\\\"q\\\"/qs\" claude rotate"),
            "{odd}"
        );
    }

    #[test]
    fn the_timer_runs_it_every_three_minutes_and_soon_after_it_starts() {
        let text = timer();
        assert!(text.contains("\nOnUnitActiveSec=180\n"), "{text}");
        assert!(text.contains("\nOnActiveSec=5\n"), "{text}");
        assert!(text.contains("\nWantedBy=timers.target\n"), "{text}");
    }

    #[test]
    fn the_units_are_the_users_own() {
        let home = Path::new("/home/lan");
        assert_eq!(
            timer_path(home),
            home.join(".config/systemd/user/com.shin315.quota-station.rotation.timer")
        );
        assert_eq!(
            service_path(home),
            home.join(".config/systemd/user/com.shin315.quota-station.rotation.service")
        );
    }

    #[test]
    fn without_systemd_the_refusal_gives_the_cron_line() {
        let text = no_systemd(Path::new("/home/lan/.local/bin/quota-station"));
        assert!(
            text.ends_with(
                "*/3 * * * * '/home/lan/.local/bin/quota-station' claude rotate >/dev/null 2>&1"
            ),
            "{text}"
        );
    }
}
