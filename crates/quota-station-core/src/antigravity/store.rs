//! The app's own Keychain items for Antigravity tokens: one generic password
//! per account, service [`SERVICE`], account the app's account id, value
//! the account's [`TokenData`] as JSON. Kept with macOS's `security` tool,
//! the way claude-swap and Cockpit Tools keep theirs, so a rebuilt app with
//! a new ad-hoc signature gets no Keychain prompt.
//!
//! Items are added without `-A`, so other programs need the owner's consent
//! to read them. The app never names another service: not the
//! `gemini`/`antigravity` item that Antigravity.app and agy share, not
//! claude-swap's or Claude Code's.

use std::path::PathBuf;
use std::time::Duration;

use super::token::TokenData;
use crate::runner::{self, RunError};

pub const SERVICE: &str = "com.shin315.quota-station.antigravity";
/// Each `security` call; the tool can hang while the Keychain is locked.
pub const TIMEOUT: Duration = Duration::from_secs(15);
/// `security`'s exit code for "the specified item could not be found".
const NOT_FOUND: i32 = 44;

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

#[derive(Debug, Clone)]
pub struct Keychain {
    program: PathBuf,
}

impl Keychain {
    /// macOS's `security`.
    pub fn system() -> Self {
        // The tests run a fake `security`; one that reached for the real
        // Keychain stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake security");
        }
        Keychain { program: PathBuf::from("/usr/bin/security") }
    }

    /// A fake `security` for the tests.
    #[cfg(feature = "test-support")]
    pub fn fake(program: impl Into<PathBuf>) -> Self {
        Keychain { program: program.into() }
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

    /// Adds or replaces the account's item (`-U`). On a failure the item
    /// that was there stays as it was.
    pub fn save(&self, account: &str, token: &TokenData) -> Result<(), StoreError> {
        let value = ascii_json(token);
        self.run(&["add-generic-password", "-U", "-s", SERVICE, "-a", account, "-w", &value])
            .map(|_| ())
    }

    pub fn read(&self, account: &str) -> Result<TokenData, StoreError> {
        let out = self.run(&["find-generic-password", "-s", SERVICE, "-a", account, "-w"])?;
        let text = String::from_utf8(out).map_err(|_| StoreError::Corrupt)?;
        serde_json::from_str(text.trim_end_matches('\n')).map_err(|_| StoreError::Corrupt)
    }

    /// Removes the account's item; one that is not there counts as removed.
    pub fn delete(&self, account: &str) -> Result<(), StoreError> {
        match self.run(&["delete-generic-password", "-s", SERVICE, "-a", account]) {
            Ok(_) | Err(StoreError::NotFound) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// The token as JSON with every non-ASCII character escaped: `security -w`
/// prints a value it finds unprintable as hex, and a name or email may
/// carry accents.
fn ascii_json(token: &TokenData) -> String {
    let text = serde_json::to_string(token).expect("a token serializes");
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
        let mut token = TokenData::new(
            "a".into(),
            "r".into(),
            3600,
            Some("ngọc@example.com".into()),
            None,
            None,
            100,
        );
        token.id_token = Some("x😀".into());
        let text = ascii_json(&token);
        assert!(text.is_ascii(), "{text}");
        let back: TokenData = serde_json::from_str(&text).unwrap();
        assert_eq!(back, token);
    }
}
