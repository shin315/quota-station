// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's Keychain items for MCP secrets (M9): one generic password per
//! secret, service [`SERVICE`], account `<MCP name>:<slot>` (the slot as
//! [`super::def::Slot`] names it: `env:GITHUB_TOKEN`, `header:Authorization`),
//! value the secret. Kept with macOS's `security`, as the Antigravity items
//! are (`antigravity::store`, which shares [`StoreError`]), and added without `-A`, so another
//! program needs the owner's consent to read one.
//!
//! An item is written when a secret would otherwise exist nowhere on this
//! Mac but in the app: an MCP typed into the add form, one turned off in
//! Claude Code (which has no off, so the app removes it and keeps it), one
//! put away in the store, a key pasted from Bitwarden. Copying an MCP from
//! one agent to another reads the secret from the first agent's file.

use std::path::PathBuf;
use std::time::Duration;

use crate::runner::{self, RunError};

pub const SERVICE: &str = "com.shin315.quota-station.mcp-secret";
/// `security`'s exit code for "the specified item could not be found".
const NOT_FOUND: i32 = 44;
/// Each `security` call; the tool can hang while the Keychain is locked.
pub const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound,
    Timeout,
    Spawn(String),
    /// `security` failed: its exit code and first stderr line.
    Failed {
        code: Option<i32>,
        message: String,
    },
    /// The item holds something that is not a token.
    Corrupt,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::NotFound => write!(f, "no Keychain item for this account"),
            StoreError::Timeout => {
                write!(f, "security did not answer within {}s", TIMEOUT.as_secs())
            }
            StoreError::Spawn(e) => write!(f, "could not run security: {e}"),
            StoreError::Failed { code: Some(c), message } => {
                write!(f, "security exited {c}: {message}")
            }
            StoreError::Failed { code: None, message } => {
                write!(f, "security was stopped: {message}")
            }
            StoreError::Corrupt => write!(f, "the Keychain item does not hold a token"),
        }
    }
}

impl std::error::Error for StoreError {}

/// The item's account for one secret of one MCP.
pub fn account(mcp: &str, slot: &str) -> String {
    format!("{mcp}:{slot}")
}

#[derive(Debug, Clone)]
pub struct Secrets {
    program: PathBuf,
}

impl Secrets {
    /// macOS's `security`.
    pub fn system() -> Self {
        // The tests run a fake `security`; one that reached for the real
        // Keychain stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake security");
        }
        Secrets { program: PathBuf::from("/usr/bin/security") }
    }

    /// A fake `security` for the tests.
    #[cfg(feature = "test-support")]
    pub fn fake(program: impl Into<PathBuf>) -> Self {
        Secrets { program: program.into() }
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>, StoreError> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let out = runner::run(&self.program, &args, TIMEOUT).map_err(|e| match e {
            RunError::Timeout(_) => StoreError::Timeout,
            other => StoreError::Spawn(other.to_string()),
        })?;
        match out.code {
            Some(0) => Ok(out.stdout),
            Some(NOT_FOUND) => Err(StoreError::NotFound),
            code => Err(StoreError::Failed { code, message: runner::first_line(&out.stderr) }),
        }
    }

    /// Adds or replaces the item (`-U`). On a failure the item that was
    /// there stays as it was.
    pub fn save(&self, mcp: &str, slot: &str, value: &str) -> Result<(), StoreError> {
        let account = account(mcp, slot);
        let value = ascii(value);
        self.run(&["add-generic-password", "-U", "-s", SERVICE, "-a", &account, "-w", &value])
            .map(|_| ())
    }

    pub fn read(&self, mcp: &str, slot: &str) -> Result<String, StoreError> {
        let account = account(mcp, slot);
        let out = self.run(&["find-generic-password", "-s", SERVICE, "-a", &account, "-w"])?;
        let text = String::from_utf8(out).map_err(|_| StoreError::Corrupt)?;
        serde_json::from_str(text.trim_end_matches('\n')).map_err(|_| StoreError::Corrupt)
    }

    /// Whether the item is there, without reading its value.
    pub fn has(&self, mcp: &str, slot: &str) -> Result<bool, StoreError> {
        let account = account(mcp, slot);
        match self.run(&["find-generic-password", "-s", SERVICE, "-a", &account]) {
            Ok(_) => Ok(true),
            Err(StoreError::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Removes the item; one that is not there counts as removed.
    pub fn delete(&self, mcp: &str, slot: &str) -> Result<(), StoreError> {
        let account = account(mcp, slot);
        match self.run(&["delete-generic-password", "-s", SERVICE, "-a", &account]) {
            Ok(_) | Err(StoreError::NotFound) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// The value as a JSON string with every non-ASCII character escaped:
/// `security -w` prints a value it finds unprintable as hex.
fn ascii(value: &str) -> String {
    let text = serde_json::to_string(value).expect("a string serializes");
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_value_is_ascii_and_reads_back_the_same() {
        let text = ascii("khoá-😀\"x");
        assert!(text.is_ascii(), "{text}");
        assert_eq!(serde_json::from_str::<String>(&text).unwrap(), "khoá-😀\"x");
    }
}
