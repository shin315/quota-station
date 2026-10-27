// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation on a system without launchd (Windows): the app runs it
//! itself, every [`agent::EVERY_SECONDS`] while it is open, as launchd runs
//! `<exe> claude rotate` on macOS. Nothing is registered with the system:
//! turning it on writes a file in the app's own folder, and the app's timer
//! runs only while that file is there. While the app is closed nothing runs
//! and no token is refreshed; the next start catches up with one run.

use std::path::{Path, PathBuf};

use super::agent::{self, AgentError};
use crate::os::Scheduler;

/// The file that says the rotation is on, in the app's `claude` folder.
pub const FLAG: &str = "rotation-timer";

/// The app's own timer, as the app's [`Scheduler`] on Windows.
#[derive(Debug, Clone)]
pub struct AppTimer {
    /// The flag file; `None` when the app's folder cannot be found.
    flag: Option<PathBuf>,
}

impl AppTimer {
    /// The timer of the app whose folder is `app_dir`.
    pub fn at(app_dir: &Path) -> AppTimer {
        AppTimer { flag: Some(app_dir.join(super::DIR).join(FLAG)) }
    }

    /// The app's folder for the owner's home (`USERPROFILE`), as
    /// [`super::app_dir`] names it for the command line.
    pub fn system() -> AppTimer {
        match std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            Some(home) => AppTimer::at(&super::app_dir(Path::new(&home))),
            None => AppTimer { flag: None },
        }
    }

    /// The flag file, for the app's timer.
    pub fn flag(&self) -> Option<&Path> {
        self.flag.as_deref()
    }
}

impl Scheduler for AppTimer {
    fn loaded(&self) -> Option<bool> {
        self.flag.as_ref().map(|f| f.is_file())
    }

    /// The older switchers are macOS's LaunchAgents: none runs here.
    fn legacy_loaded(&self) -> Option<&'static str> {
        None
    }

    /// Turns the timer on; `exe` is not needed, the app runs the rotation
    /// itself.
    fn install(&self, _home: &Path, _exe: &Path) -> Result<(), AgentError> {
        let flag = self.flag.as_ref().ok_or_else(|| AgentError::Io("no home folder".into()))?;
        let dir = flag.parent().ok_or_else(|| AgentError::Io("no folder".into()))?;
        super::private_dir(dir).map_err(|e| AgentError::Io(e.to_string()))?;
        std::fs::write(flag, format!("every {} seconds\n", agent::EVERY_SECONDS))
            .map_err(|e| AgentError::Io(e.to_string()))
    }

    fn remove(&self, _home: &Path) -> Result<(), AgentError> {
        let Some(flag) = &self.flag else { return Ok(()) };
        match std::fs::remove_file(flag) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(AgentError::Io(e.to_string()))
            }
            _ => Ok(()),
        }
    }
}

/// Whether the app's timer runs the rotation now: while it is on, once at
/// the start (or when it was just turned on) and then every
/// [`agent::EVERY_SECONDS`]. `since_last` is the time since the last run,
/// `None` before the first.
pub fn due(on: bool, since_last: Option<std::time::Duration>) -> bool {
    on && since_last.is_none_or(|d| d.as_secs() >= u64::from(agent::EVERY_SECONDS))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn turning_it_on_and_off_is_a_file_in_the_apps_folder() {
        let dir = std::env::temp_dir().join(format!("qs-timer-{}", std::process::id()));
        let timer = AppTimer::at(&dir);
        assert_eq!(timer.loaded(), Some(false));
        assert_eq!(timer.legacy_loaded(), None);
        timer.install(&dir, Path::new("quota-station.exe")).unwrap();
        assert_eq!(timer.loaded(), Some(true));
        assert!(dir.join("claude").join(FLAG).is_file());
        // Turned on twice is still on, and off twice is still off.
        timer.install(&dir, Path::new("quota-station.exe")).unwrap();
        timer.remove(&dir).unwrap();
        timer.remove(&dir).unwrap();
        assert_eq!(timer.loaded(), Some(false));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_home_it_cannot_tell_and_cannot_be_turned_on() {
        let timer = AppTimer { flag: None };
        assert_eq!(timer.loaded(), None);
        assert!(timer.install(Path::new("."), Path::new("x")).is_err());
        assert_eq!(timer.remove(Path::new(".")), Ok(()));
    }

    #[test]
    fn it_runs_at_once_then_every_three_minutes_and_never_while_off() {
        assert!(due(true, None));
        assert!(!due(true, Some(Duration::from_secs(179))));
        assert!(due(true, Some(Duration::from_secs(180))));
        assert!(!due(false, None));
        assert!(!due(false, Some(Duration::from_secs(600))));
    }
}
