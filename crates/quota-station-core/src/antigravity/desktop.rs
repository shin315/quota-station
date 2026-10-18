// The version rule is adapted from Cockpit Tools
// `src-tauri/src/commands/account.rs` (`compare_versions`,
// `resolve_antigravity_desktop_auth_mode_from_info`) and
// `src-tauri/src/commands/system_config_types.rs`
// (`read_antigravity_macos_bundle_metadata`) (1.3.70, commit 244b957, CC
// BY-NC-SA 4.0); changes: only Antigravity.app 2.0 and later are switched
// (the old SQLite login is not ported, so an older app is refused instead of
// written another way), and the version is read with `plutil -extract`. The
// quit and the reopen are the app's own: Cockpit closes the app's processes
// by their data folder; this app asks macOS to quit it by its bundle id and
// opens it again the same way.

//! Antigravity.app, as a switch sees it: is it installed, which version, is
//! it running, quit it politely, open it again. Every question is a short
//! program run, never a file this app opens:
//!
//! - installed and version: `plutil -extract CFBundleShortVersionString raw`
//!   on its `Info.plist`;
//! - running: `pgrep -f /Applications/Antigravity.app/Contents/MacOS/Antigravity`;
//! - quit: `osascript -e 'tell application id "com.google.antigravity" to quit'`,
//!   the same as choosing Quit in its menu, so the app may save its work or
//!   ask first. Nothing here ever kills it;
//! - open: `open -b com.google.antigravity`.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::runner;

pub const BUNDLE_ID: &str = "com.google.antigravity";
pub const APP_PATH: &str = "/Applications/Antigravity.app";
/// What `pgrep -f` looks for: the main executable's path on the command line.
pub const PROCESS: &str = "/Applications/Antigravity.app/Contents/MacOS/Antigravity";
/// The first version that keeps its login in the shared Keychain item.
pub const MIN_VERSION: &str = "2.0.0";
/// How long a quit may take before the switch gives up (Cockpit's 20 s).
pub const QUIT_WAIT: Duration = Duration::from_secs(20);
/// Each `pgrep`, `plutil` or `open`.
const STEP: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(250);

/// What is installed at [`APP_PATH`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Installed {
    /// No Antigravity.app: a switch only changes agy's login.
    Missing,
    /// 2.0 or later, or a version the rule cannot read (Cockpit treats that
    /// as 2.0 too).
    Supported { version: String },
    /// Older than 2.0: it keeps its login elsewhere, and the app does not
    /// switch it.
    TooOld { version: String },
}

/// Cockpit's version compare: the numbers in each, part by part.
fn parse_version_parts(value: &str) -> Vec<u64> {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left_parts = parse_version_parts(left);
    let right_parts = parse_version_parts(right);
    if left_parts.is_empty() || right_parts.is_empty() {
        return None;
    }
    let max_len = left_parts.len().max(right_parts.len());
    for index in 0..max_len {
        let left_value = left_parts.get(index).copied().unwrap_or(0);
        let right_value = right_parts.get(index).copied().unwrap_or(0);
        match left_value.cmp(&right_value) {
            std::cmp::Ordering::Equal => {}
            ordering => return Some(ordering),
        }
    }
    Some(std::cmp::Ordering::Equal)
}

/// What a version string says about switching.
pub fn classify(version: &str) -> Installed {
    let version = version.trim().to_string();
    match compare_versions(&version, MIN_VERSION) {
        Some(std::cmp::Ordering::Less) => Installed::TooOld { version },
        _ => Installed::Supported { version },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuitError {
    /// `pgrep` did not say whether it runs, so a quit cannot be confirmed.
    Unknown(String),
    /// Still running when the wait ran out: it may be asking to save, or
    /// refused to quit.
    StillRunning,
}

impl std::fmt::Display for QuitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuitError::Unknown(e) => write!(f, "could not tell whether Antigravity runs: {e}"),
            QuitError::StillRunning => write!(f, "Antigravity.app did not quit"),
        }
    }
}

impl std::error::Error for QuitError {}

/// The programs the questions run, and where the app is.
#[derive(Debug, Clone)]
pub struct Desktop {
    plutil: PathBuf,
    pgrep: PathBuf,
    osascript: PathBuf,
    open: PathBuf,
    app: PathBuf,
    process: String,
}

impl Desktop {
    /// The real Antigravity.app, through macOS's own tools.
    pub fn system() -> Self {
        // The tests run fakes; one that reached for the real app stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake Antigravity.app");
        }
        Desktop {
            plutil: PathBuf::from("/usr/bin/plutil"),
            pgrep: PathBuf::from("/usr/bin/pgrep"),
            osascript: PathBuf::from("/usr/bin/osascript"),
            open: PathBuf::from("/usr/bin/open"),
            app: PathBuf::from(APP_PATH),
            process: PROCESS.to_string(),
        }
    }

    /// Fakes for the tests: `plutil`, `pgrep`, `osascript` and `open` in
    /// `bin`, and an app bundle at `app`.
    #[cfg(feature = "test-support")]
    pub fn fake(bin: &Path, app: &Path) -> Self {
        Desktop {
            plutil: bin.join("plutil"),
            pgrep: bin.join("pgrep"),
            osascript: bin.join("osascript"),
            open: bin.join("open"),
            app: app.to_path_buf(),
            process: app.join("Contents/MacOS/Antigravity").display().to_string(),
        }
    }

    fn run(&self, program: &Path, args: &[&str]) -> Result<runner::Output, String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        runner::run(program, &args, STEP).map_err(|e| e.to_string())
    }

    /// Whether Antigravity.app is installed, and which version. An app whose
    /// `Info.plist` has no version counts as supported, as in Cockpit.
    pub fn installed(&self) -> Installed {
        let plist = self.app.join("Contents/Info.plist").display().to_string();
        let read = |key: &str| match self
            .run(&self.plutil, &["-extract", key, "raw", "-o", "-", &plist])
        {
            Ok(out) if out.code == Some(0) => {
                let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                (!v.is_empty()).then_some(v)
            }
            _ => None,
        };
        match read("CFBundleShortVersionString").or_else(|| read("CFBundleVersion")) {
            Some(version) => classify(&version),
            // A bundle whose plist has no version: there, of a kind not known.
            None if read("CFBundleIdentifier").is_some() => {
                Installed::Supported { version: String::new() }
            }
            None => Installed::Missing,
        }
    }

    /// Whether it runs: `pgrep` exits 0 for a match and 1 for none.
    pub fn running(&self) -> Result<bool, String> {
        let out = self.run(&self.pgrep, &["-f", &self.process])?;
        match out.code {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            code => Err(format!("pgrep exited {code:?}: {}", runner::first_line(&out.stderr))),
        }
    }

    /// Asks it to quit and waits up to `wait` for it to be gone. It is never
    /// killed: one that does not quit is left running and reported.
    pub fn quit(&self, wait: Duration) -> Result<(), QuitError> {
        let deadline = Instant::now() + wait;
        let script = format!("tell application id \"{BUNDLE_ID}\" to quit");
        // osascript waits for the app's answer; a dialog in the app can hold
        // it, so it gets only the time left, and its answer is not trusted:
        // the process list decides.
        let args = ["-e".to_string(), script];
        let _ = runner::run(&self.osascript, &args, wait);
        loop {
            match self.running() {
                Ok(false) => return Ok(()),
                Ok(true) if Instant::now() >= deadline => return Err(QuitError::StillRunning),
                Ok(true) => thread::sleep(POLL),
                Err(e) => return Err(QuitError::Unknown(e)),
            }
        }
    }

    /// Opens it again.
    pub fn open(&self) -> Result<(), String> {
        let out = self.run(&self.open, &["-b", BUNDLE_ID])?;
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
    fn only_versions_from_2_0_are_switched() {
        assert_eq!(classify("2.24.0"), Installed::Supported { version: "2.24.0".into() });
        assert_eq!(classify("2.0"), Installed::Supported { version: "2.0".into() });
        assert_eq!(classify("v10.1"), Installed::Supported { version: "v10.1".into() });
        assert_eq!(classify("1.20.5"), Installed::TooOld { version: "1.20.5".into() });
        assert_eq!(classify("1.99.99"), Installed::TooOld { version: "1.99.99".into() });
        // Cockpit treats a version it cannot read as the new kind.
        assert_eq!(classify("dev"), Installed::Supported { version: "dev".into() });
    }
}
