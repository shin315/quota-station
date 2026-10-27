// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's own secrets as files, each sealed by the system: the
//! [`SecretStore`] of the Claude slots, the MCP secrets and the Antigravity
//! tokens on Windows, where DPAPI seals them (`win::Dpapi`) for this user
//! on this computer, as the login Keychain keeps them on a Mac.
//!
//! One file per item, `<dir>/<service>/<account>.bin`, each name written
//! with `%XX` for any byte other than a letter, a digit, `.`, `_` or `-`
//! (an MCP secret's account holds a `:`). A file is written whole to a
//! temporary file first and renamed over the old one, so a failed write
//! leaves the item as it was. The folders are for their owner alone.
//!
//! Credential Manager is not used for these: one item holds 2 560 bytes at
//! most, less than a Claude login with its MCP secrets, and it follows a
//! roaming profile to other computers, where DPAPI's seal does not open.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::claude::keychain::KeychainError;
use crate::os::SecretStore;

/// Seals and opens a secret's bytes.
pub trait Seal: Send + Sync + std::fmt::Debug {
    fn seal(&self, plain: &[u8]) -> io::Result<Vec<u8>>;
    fn open(&self, sealed: &[u8]) -> io::Result<Vec<u8>>;
}

/// The items as sealed files under one folder.
#[derive(Debug, Clone)]
pub struct SealedFiles<S> {
    dir: PathBuf,
    seal: S,
}

impl<S: Seal> SealedFiles<S> {
    pub fn new(dir: impl Into<PathBuf>, seal: S) -> Self {
        SealedFiles { dir: dir.into(), seal }
    }

    /// The item's file.
    pub fn file(&self, service: &str, account: &str) -> PathBuf {
        self.dir.join(name(service)).join(format!("{}.bin", name(account)))
    }

    fn read(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        let sealed = match std::fs::read(self.file(service, account)) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(failed(e)),
        };
        let mut plain = self.seal.open(&sealed).map_err(failed)?;
        let text = String::from_utf8(plain.clone()).map_err(|_| KeychainError::Failed {
            code: Some(0),
            message: "the value is not text".into(),
        });
        plain.fill(0);
        text.map(Some)
    }
}

impl<S: Seal> SecretStore for SealedFiles<S> {
    fn get_within(
        &self,
        service: &str,
        account: &str,
        _wait: Duration,
    ) -> Result<Option<String>, KeychainError> {
        self.read(service, account)
    }

    fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        self.read(service, account)
    }

    fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError> {
        Ok(self.file(service, account).is_file())
    }

    fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError> {
        let file = self.file(service, account);
        let sealed = self.seal.seal(value.as_bytes()).map_err(failed)?;
        write_new(&file, &sealed).map_err(failed)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError> {
        match std::fs::remove_file(self.file(service, account)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(failed(e)),
            _ => Ok(()),
        }
    }
}

fn failed(e: io::Error) -> KeychainError {
    KeychainError::Failed { code: e.raw_os_error(), message: e.to_string() }
}

/// `text` as a file name: `%XX` for every byte but a letter, a digit, `.`,
/// `_` and `-`, and for the first letter of a name Windows keeps for a
/// device (`CON`, `NUL`, `COM1`).
pub fn name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if device || out.is_empty() || out.starts_with('.') {
        let first = out.bytes().next().map_or(String::new(), |b| format!("%{b:02X}"));
        out = first + out.get(1..).unwrap_or("");
    }
    out
}

/// `bytes` at `file` through a temporary file beside it and a rename, in
/// folders for their owner alone.
fn write_new(file: &Path, bytes: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = file.parent().ok_or_else(|| io::Error::other("no folder"))?;
    crate::os::owner_dir(dir)?;
    let tmp =
        dir.join(format!(".{}.{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let written =
        crate::os::with_mode(std::fs::OpenOptions::new().write(true).create_new(true), 0o600)
            .open(&tmp)
            .and_then(|mut f| {
                f.write_all(bytes)?;
                f.sync_all()
            })
            .and_then(|()| crate::os::replace(&tmp, file));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a seal: the bytes reversed, so a test sees that the file is not
    /// the value.
    #[derive(Debug)]
    struct Mirror;

    impl Seal for Mirror {
        fn seal(&self, plain: &[u8]) -> io::Result<Vec<u8>> {
            Ok(plain.iter().rev().copied().collect())
        }

        fn open(&self, sealed: &[u8]) -> io::Result<Vec<u8>> {
            self.seal(sealed)
        }
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qs-sealed-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn names_hold_only_plain_characters() {
        assert_eq!(name("com.shin315.quota-station.claude"), "com.shin315.quota-station.claude");
        assert_eq!(name("github:env:GITHUB_TOKEN"), "github%3Aenv%3AGITHUB_TOKEN");
        assert_eq!(name("a/b\\c"), "a%2Fb%5Cc");
        assert_eq!(name("nul"), "%6Eul");
        assert_eq!(name("COM1.x"), "%43OM1.x");
        assert_eq!(name("COMX"), "COMX");
        assert_eq!(name(".."), "%2E.");
        assert_eq!(name("khoá"), "kho%C3%A1");
    }

    #[test]
    fn an_item_is_written_read_and_removed_as_a_sealed_file() {
        let dir = temp("items");
        let store = SealedFiles::new(&dir, Mirror);
        assert_eq!(store.get("svc", "slot-1").unwrap(), None);
        assert!(!store.exists("svc", "slot-1").unwrap());
        store.set("svc", "slot-1", r#"{"claudeAiOauth":{"accessToken":"t"}}"#).unwrap();
        let file = dir.join("svc").join("slot-1.bin");
        assert!(!std::fs::read_to_string(&file).unwrap().starts_with('{'));
        assert_eq!(
            store.get("svc", "slot-1").unwrap().as_deref(),
            Some(r#"{"claudeAiOauth":{"accessToken":"t"}}"#)
        );
        store.set("svc", "slot-1", "two").unwrap();
        assert_eq!(store.get("svc", "slot-1").unwrap().as_deref(), Some("two"));
        assert!(store.exists("svc", "slot-1").unwrap());
        let left: Vec<_> = std::fs::read_dir(dir.join("svc")).unwrap().flatten().collect();
        assert_eq!(left.len(), 1, "no temporary file left");
        store.delete("svc", "slot-1").unwrap();
        store.delete("svc", "slot-1").unwrap();
        assert_eq!(store.get("svc", "slot-1").unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_does_not_open_is_an_error_not_an_empty_item() {
        #[derive(Debug)]
        struct Broken;
        impl Seal for Broken {
            fn seal(&self, plain: &[u8]) -> io::Result<Vec<u8>> {
                Ok(plain.to_vec())
            }
            fn open(&self, _: &[u8]) -> io::Result<Vec<u8>> {
                Err(io::Error::other("the seal does not open here"))
            }
        }
        let dir = temp("broken");
        let store = SealedFiles::new(&dir, Broken);
        store.set("svc", "a", "x").unwrap();
        assert!(matches!(store.get("svc", "a"), Err(KeychainError::Failed { .. })));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
