// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Linux's side of the seams in [`crate::os`], for the command line
//! (`quota-station-cli`), which keeps and switches the Claude accounts
//! there without the app:
//!
//! - [`Secrets`]: the app's [`crate::os::SecretStore`]. Claude Code for
//!   Linux keeps its login in `~/.claude/.credentials.json` alone and no
//!   item anywhere ([`NoItems`]); the app's own secrets (each account's
//!   login) are files of mode 600 in folders of mode 700 under
//!   `~/.local/share/com.shin315.quota-station/secrets`, as Claude Code
//!   keeps its own ([`Plain`]): a headless machine, or Linux in WSL, has no
//!   keyring the app could count on.
//! - the rotation every three minutes, from a `systemd --user` timer
//!   ([`crate::claude::systemd::Systemd`], [`crate::os::scheduler`]).

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::claude::keychain::KeychainError;
use crate::os::{SecretStore, Split};
use crate::sealed::{Seal, SealedFiles};

/// The app's secrets and Claude Code's (absent) login items, each in its
/// place.
pub type Secrets = Split<NoItems, SealedFiles<Plain>>;

/// The folder of the app's own secrets for `home`: the app's folder under
/// `~/.local/share`, as Tauri's `app_local_data_dir` names it on Linux.
pub fn secrets_dir(home: &Path) -> PathBuf {
    home.join(".local/share").join(crate::app_dir::ID).join("secrets")
}

impl Secrets {
    pub fn system() -> Secrets {
        // The tests keep their secrets in memory; one that reached for the
        // real stores stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake secret store");
        }
        let home = home().expect("no home folder: HOME is not set and the user has none");
        Split { claude_code: NoItems, own: SealedFiles::new(secrets_dir(&home), Plain) }
    }
}

/// The user's home folder: `HOME` when it is an absolute path, else the one
/// the user database gives. Never a relative path, which would put the
/// secrets in whatever folder the program runs in.
pub fn home() -> Option<PathBuf> {
    let from_env = std::env::var_os("HOME").map(PathBuf::from).filter(|h| h.is_absolute());
    from_env.or_else(|| {
        // SAFETY: `getpwuid` returns a pointer into static storage or null;
        // the folder is copied out at once.
        unsafe {
            let pw = libc::getpwuid(libc::getuid());
            if pw.is_null() || (*pw).pw_dir.is_null() {
                return None;
            }
            use std::os::unix::ffi::OsStrExt;
            let dir = std::ffi::CStr::from_ptr((*pw).pw_dir).to_bytes();
            Some(PathBuf::from(std::ffi::OsStr::from_bytes(dir))).filter(|d| d.is_absolute())
        }
    })
}

/// No seal: the bytes as they are, in a file only its owner reads
/// ([`SealedFiles`] writes mode 600 in folders of mode 700).
#[derive(Debug, Clone, Copy, Default)]
pub struct Plain;

impl Seal for Plain {
    fn seal(&self, plain: &[u8]) -> io::Result<Vec<u8>> {
        Ok(plain.to_vec())
    }

    fn open(&self, sealed: &[u8]) -> io::Result<Vec<u8>> {
        Ok(sealed.to_vec())
    }
}

/// Claude Code's login items on Linux: there are none, its login is its
/// `.credentials.json` ([`crate::claude::cc::write_login`] writes the file
/// there). Reads find nothing; a write is refused, so a login is never
/// put where Claude Code would not read it.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoItems;

impl SecretStore for NoItems {
    fn get_within(
        &self,
        _service: &str,
        _account: &str,
        _wait: Duration,
    ) -> Result<Option<String>, KeychainError> {
        Ok(None)
    }

    fn get(&self, _service: &str, _account: &str) -> Result<Option<String>, KeychainError> {
        Ok(None)
    }

    fn exists(&self, _service: &str, _account: &str) -> Result<bool, KeychainError> {
        Ok(false)
    }

    fn set(&self, _service: &str, _account: &str, _value: &str) -> Result<(), KeychainError> {
        Err(KeychainError::Failed {
            code: None,
            message: "Claude Code keeps its login in .credentials.json on Linux".into(),
        })
    }

    fn delete(&self, _service: &str, _account: &str) -> Result<(), KeychainError> {
        Ok(())
    }
}
