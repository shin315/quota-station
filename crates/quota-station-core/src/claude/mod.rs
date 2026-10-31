// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts, kept by Quota Station itself (see
//! `docs/claude-accounts.md`): the account list and its Keychain items
//! ([`store`]), the switch of Claude Code's active login under Claude
//! Code's own locks ([`switch`]), each account's usage ([`usage`]), running
//! one process as a chosen account ([`session`]), the data file other
//! programs read ([`state`]) and the one-time import from claude-swap
//! ([`import`]).
// @private:begin
//! With the `author-tools` feature also the rotation's decision
//! ([`engine`]) and its run ([`rotate`]) from the app's own LaunchAgent
//! ([`agent`]), or on Windows the app's own timer ([`timer`]).
// @private:end
//!
//! What touches Claude Code's login is learned from claude-swap's source
//! (MIT, <https://github.com/realiti4/claude-swap>), credited in
//! `THIRD_PARTY.md`: the Keychain items and their `security` commands, the
//! lock folders and their order, the `oauthAccount` splice, the usage and
//! token endpoints, and the session folder.

#[cfg(feature = "author-tools")]
pub mod agent;
pub mod cc;
pub mod cli;
pub mod credman;
#[cfg(feature = "author-tools")]
pub mod engine;
pub mod import;
pub mod keychain;
pub mod locks;
pub mod manage;
#[cfg(feature = "author-tools")]
pub mod rotate;
pub mod session;
pub mod state;
pub mod store;
pub mod switch;
#[cfg(feature = "author-tools")]
pub mod timer;
pub mod usage;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The app's folder for the Claude accounts, under its Application Support
/// folder.
pub const DIR: &str = "claude";

/// Where everything of the Claude accounts lives on this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// The owner's home folder: Claude Code's files are under it.
    pub home: PathBuf,
    /// `~/Library/Application Support/com.shin315.quota-station/claude`.
    pub dir: PathBuf,
    /// How long a change waits for each of Claude Code's lock folders.
    pub lock_wait: std::time::Duration,
}

impl Paths {
    /// `app_dir` is the app's Application Support folder.
    pub fn new(home: impl Into<PathBuf>, app_dir: &Path) -> Paths {
        Paths { home: home.into(), dir: app_dir.join(DIR), lock_wait: locks::CC_TIMEOUT }
    }

    /// The app's folder as the command line finds it, without Tauri.
    pub fn for_home(home: &Path) -> Paths {
        Paths::new(home, &app_dir(home))
    }

    pub fn accounts(&self) -> PathBuf {
        self.dir.join("accounts.json")
    }

    pub fn usage(&self) -> PathBuf {
        self.dir.join("usage.json")
    }

    #[cfg(feature = "author-tools")]
    pub fn rotation(&self) -> PathBuf {
        self.dir.join("rotation")
    }

    #[cfg(feature = "author-tools")]
    pub fn log(&self) -> PathBuf {
        self.dir.join("rotation.log")
    }

    pub fn state(&self) -> PathBuf {
        self.dir.join("state.json")
    }

    /// The copy in claude-swap's layout that Firstmate's quota-axi reads.
    #[cfg(feature = "author-tools")]
    pub fn swap(&self) -> PathBuf {
        self.dir.join("swap")
    }

    pub fn sessions(&self) -> PathBuf {
        self.dir.join("sessions")
    }

    /// Priming is on while this file exists.
    #[cfg(feature = "author-tools")]
    pub fn prime_flag(&self) -> PathBuf {
        self.dir.join("prime")
    }

    /// When each account was last primed.
    #[cfg(feature = "author-tools")]
    pub fn prime_state(&self) -> PathBuf {
        self.dir.join("prime-state")
    }

    /// The `flock` every change of the accounts, their credentials or Claude
    /// Code's login takes first.
    pub fn lock(&self) -> PathBuf {
        self.dir.join(".lock")
    }

    /// Claude Code's `~/.claude.json`.
    pub fn claude_json(&self) -> PathBuf {
        self.home.join(".claude.json")
    }

    /// Claude Code's folder, `~/.claude`.
    pub fn claude_home(&self) -> PathBuf {
        self.home.join(".claude")
    }

    /// The app keeps the Claude accounts on this Mac: they were imported or
    /// added.
    pub fn in_use(&self) -> bool {
        self.accounts().is_file()
    }

    /// The rotation file the app reads and writes: its own once it keeps
    /// the accounts (or has its own copy), else the old switcher's.
    #[cfg(feature = "author-tools")]
    pub fn rotation_file(&self) -> PathBuf {
        if self.in_use() || self.rotation().exists() {
            self.rotation()
        } else {
            crate::rotation::path(&self.home)
        }
    }

    /// The app's `settings.json`, beside this folder.
    pub fn settings_file(&self) -> PathBuf {
        self.dir
            .parent()
            .map_or_else(|| self.dir.join("settings.json"), |d| d.join("settings.json"))
    }

    /// The copy for Firstmate is written: the app keeps the accounts, and
    /// Cài đặt › Nâng cao shows Firstmate. Only a build with the
    /// `author-tools` feature writes it.
    #[cfg(feature = "author-tools")]
    pub fn firstmate_copy(&self) -> bool {
        self.in_use() && crate::settings::Settings::load(&self.settings_file()).firstmate_shown()
    }

    /// The variables that point Firstmate's quota-axi at the app's copy of
    /// the accounts and its rotation file instead of claude-swap's
    /// (`docs/claude-accounts.md`); none while no copy is written.
    #[cfg(feature = "author-tools")]
    pub fn firstmate_env(&self) -> Vec<(String, std::ffi::OsString)> {
        if !self.firstmate_copy() {
            return Vec::new();
        }
        vec![
            ("QUOTA_AXI_CLAUDE_SWAP_DIR".into(), self.swap().into_os_string()),
            ("QUOTA_AXI_CLAUDE_ROTATION".into(), self.rotation_file().into_os_string()),
        ]
    }
}

/// The app's own folder for `home`, as Tauri's `PathResolver` names it
/// from the bundle id (`app_config_dir`, where `settings.json` is): the
/// Application Support folder on macOS, the roaming AppData folder on
/// Windows. The app takes it from Tauri; the command line, which runs
/// without Tauri, from here.
pub fn app_dir(home: &Path) -> PathBuf {
    let id = crate::app_dir::ID;
    if cfg!(windows) {
        home.join("AppData").join("Roaming").join(id)
    } else {
        home.join("Library/Application Support").join(id)
    }
}

/// Makes `dir` and its parents, then sets it to 700.
pub(crate) fn private_dir(dir: &Path) -> io::Result<()> {
    std::fs::DirBuilder::new().recursive(true).create(dir)?;
    crate::os::set_mode(dir, 0o700)
}

/// Writes `text` through a temporary file and a rename, mode 600, in a
/// folder of mode 700.
pub(crate) fn write_private(path: &Path, text: &str) -> io::Result<()> {
    use std::io::Write;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = path.parent().ok_or_else(|| io::Error::other("no folder"))?;
    private_dir(dir)?;
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written =
        crate::os::with_mode(std::fs::OpenOptions::new().write(true).create_new(true), 0o600)
            .open(&tmp)
            .and_then(|mut file| {
                file.write_all(text.as_bytes())?;
                crate::os::sync_all(&file)
            })
            .and_then(|()| crate::os::replace(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// The file's text, or `None` when it is not there.
pub(crate) fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Seconds since the epoch, with the fraction, as claude-swap keeps times.
pub(crate) fn epoch(t: time::OffsetDateTime) -> f64 {
    t.unix_timestamp_nanos() as f64 / 1e9
}
