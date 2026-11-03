// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Quota Station's own LaunchAgent, which runs the rotation every three
//! minutes whether or not the app is open, as `com.cswap.prefer-team` ran
//! the old switcher, and the check that the two older switchers are not
//! loaded beside it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::os::Scheduler;
use crate::runner;
use crate::switcher;

pub const LABEL: &str = "com.shin315.quota-station.rotation";
/// How often launchd runs the rotation, as the old switcher ran.
pub const EVERY_SECONDS: u32 = 180;
/// The old switcher, and claude-swap's own.
pub const LEGACY: [&str; 2] = [switcher::SERVICE, switcher::AUTO_SERVICE];
const LAUNCHCTL_TIMEOUT: Duration = Duration::from_secs(10);

pub fn plist_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The agent's plist: `<exe> claude rotate` every [`EVERY_SECONDS`].
pub fn plist(exe: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>claude</string>
    <string>rotate</string>
  </array>
  <key>StartInterval</key>
  <integer>{EVERY_SECONDS}</integer>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Background</string>
  <key>StandardOutPath</key>
  <string>/dev/null</string>
  <key>StandardErrorPath</key>
  <string>/dev/null</string>
</dict>
</plist>
"#,
        escape(&exe.display().to_string())
    )
}

#[cfg(unix)]
fn domain() -> String {
    // SAFETY: `getuid` only reads the process's user id.
    format!("gui/{}", unsafe { libc::getuid() })
}

/// No launchd: nothing reaches `launchctl` (see [`install`]).
#[cfg(not(unix))]
fn domain() -> String {
    String::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// One of the older switchers is loaded: two switchers would switch
    /// against each other.
    Legacy(String),
    Io(String),
    /// `launchctl` refused: its first line.
    Launchctl(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Legacy(label) => write!(f, "{label} is loaded"),
            AgentError::Io(e) => write!(f, "{e}"),
            AgentError::Launchctl(e) => write!(f, "launchctl: {e}"),
        }
    }
}

impl std::error::Error for AgentError {}

/// The older switcher still loaded, if any: the first of [`LEGACY`] that
/// launchd has loaded, or of which it cannot tell, so nothing switches or
/// refreshes beside a switcher that may run. The older switchers are
/// macOS's: elsewhere none runs, and no `launchctl` is asked.
pub fn legacy_loaded(launchctl: &Path) -> Option<&'static str> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    LEGACY.into_iter().find(|label| switcher::service_loaded(launchctl, label) != Some(false))
}

/// Whether launchd has the agent loaded; `None` when that cannot be told.
pub fn loaded(launchctl: &Path) -> Option<bool> {
    switcher::service_loaded(launchctl, LABEL)
}

/// Writes the plist and loads it. Refused while an older switcher is
/// loaded.
pub fn install(launchctl: &Path, home: &Path, exe: &Path) -> Result<(), AgentError> {
    if !cfg!(unix) {
        return Err(AgentError::Io(crate::os::UNSUPPORTED.into()));
    }
    if let Some(label) = legacy_loaded(launchctl) {
        return Err(AgentError::Legacy(label.to_string()));
    }
    let file = plist_path(home);
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| AgentError::Io(e.to_string()))?;
    }
    std::fs::write(&file, plist(exe)).map_err(|e| AgentError::Io(e.to_string()))?;
    if loaded(launchctl) == Some(true) {
        // A plist for another copy of the app: load this one instead.
        let _ = launchctl_run(launchctl, &["bootout", &format!("{}/{LABEL}", domain())]);
    }
    launchctl_run(launchctl, &["bootstrap", &domain(), &file.display().to_string()])
}

/// Unloads the agent and removes its plist.
pub fn remove(launchctl: &Path, home: &Path) -> Result<(), AgentError> {
    if loaded(launchctl) != Some(false) {
        launchctl_run(launchctl, &["bootout", &format!("{}/{LABEL}", domain())])?;
    }
    match std::fs::remove_file(plist_path(home)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(AgentError::Io(e.to_string())),
        _ => Ok(()),
    }
}

/// launchd, through `launchctl`: the app's [`Scheduler`] on macOS.
#[derive(Debug, Clone)]
pub struct Launchd {
    pub launchctl: PathBuf,
}

impl Launchd {
    pub fn system() -> Launchd {
        Launchd { launchctl: PathBuf::from(switcher::LAUNCHCTL) }
    }
}

impl Scheduler for Launchd {
    fn loaded(&self) -> Option<bool> {
        loaded(&self.launchctl)
    }

    fn legacy_loaded(&self) -> Option<&'static str> {
        legacy_loaded(&self.launchctl)
    }

    fn install(&self, home: &Path, exe: &Path) -> Result<(), AgentError> {
        install(&self.launchctl, home, exe)
    }

    fn remove(&self, home: &Path) -> Result<(), AgentError> {
        remove(&self.launchctl, home)
    }
}

fn launchctl_run(launchctl: &Path, args: &[&str]) -> Result<(), AgentError> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let out = runner::run(launchctl, &args, LAUNCHCTL_TIMEOUT)
        .map_err(|e| AgentError::Launchctl(e.to_string()))?;
    match out.code {
        Some(0) => Ok(()),
        _ => Err(AgentError::Launchctl(runner::first_line(&out.stderr))),
    }
}
