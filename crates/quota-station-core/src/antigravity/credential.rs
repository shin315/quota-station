// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Ported from Cockpit Tools `src-tauri/src/modules/antigravity_credential.rs`
// (1.3.70, commit 244b957); see NOTICE for the changes.

//! The login Antigravity and the `agy` CLI share, which both read when they
//! start. On macOS it is one generic password in the login Keychain, service
//! `gemini`, account `antigravity`, its value the payload in go-keyring's
//! base64; on Windows one generic credential in Credential Manager, target
//! `gemini:antigravity`, user `antigravity`, its value the payload's JSON as
//! it is, as go-keyring keeps it there. Switching the Antigravity account
//! means replacing it, and this module is the only place the app names it.
//!
//! The app only ever writes the item, never reads it, and only when the
//! owner confirms a switch. The write updates the item in place
//! (`security add-generic-password -U`, `CredWriteW`), so a failed write
//! leaves the login that was there as it was.

#[cfg(not(windows))]
use std::path::PathBuf;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use time::OffsetDateTime;

use super::token::TokenData;
#[cfg(not(windows))]
use crate::runner::{self, RunError};

/// The shared item's service and account, as Antigravity's Go keyring
/// names them.
pub const SERVICE: &str = "gemini";
pub const ACCOUNT: &str = "antigravity";
/// The shared credential's target in Windows' Credential Manager:
/// go-keyring's `<service>:<account>`.
pub const TARGET: &str = "gemini:antigravity";
/// What go-keyring puts before a base64 value.
const PREFIX: &str = "go-keyring-base64:";
/// The write's timeout: `security` can hang while
/// the Keychain is locked, and is stopped then.
pub const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Serialize)]
struct AntigravityCredentialToken<'a> {
    access_token: &'a str,
    token_type: &'a str,
    refresh_token: &'a str,
    expiry: String,
}

#[derive(Serialize)]
struct AntigravityCredentialPayload<'a> {
    token: AntigravityCredentialToken<'a>,
    auth_method: &'a str,
}

/// The JSON Antigravity keeps for its login: the access token, the refresh
/// token and the access token's expiry, RFC 3339 in UTC to the microsecond.
pub fn build_payload(token: &TokenData) -> String {
    // A time out of range falls back to an expiry already passed,
    // so Antigravity refreshes the access token with the refresh token.
    let t = OffsetDateTime::from_unix_timestamp(token.expiry_timestamp)
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let expiry = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.microsecond()
    );
    serde_json::to_string(&AntigravityCredentialPayload {
        token: AntigravityCredentialToken {
            access_token: &token.access_token,
            token_type: "Bearer",
            refresh_token: &token.refresh_token,
            expiry,
        },
        auth_method: "consumer",
    })
    .expect("the payload serializes")
}

/// The value go-keyring reads back: the prefix and the payload in base64.
pub fn keychain_value(payload_json: &str) -> String {
    format!("{PREFIX}{}", STANDARD.encode(payload_json))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    /// `security` did not answer in time and was stopped.
    Timeout,
    Spawn(String),
    /// `security` refused: its exit code and first stderr line.
    Failed {
        code: Option<i32>,
        message: String,
    },
    /// Credential Manager refused the write (Windows): its own words.
    Refused(String),
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialError::Timeout => write!(f, "security timed out and was stopped"),
            CredentialError::Spawn(e) => write!(f, "could not run security: {e}"),
            CredentialError::Failed { code: Some(c), message } => {
                write!(f, "security exited {c}: {message}")
            }
            CredentialError::Failed { code: None, message } => {
                write!(f, "security was stopped: {message}")
            }
            CredentialError::Refused(message) => {
                write!(f, "Credential Manager refused the write: {message}")
            }
        }
    }
}

impl std::error::Error for CredentialError {}

/// Writes the shared item.
#[cfg(not(windows))]
pub struct SharedLogin {
    program: PathBuf,
    timeout: Duration,
}

#[cfg(not(windows))]
impl SharedLogin {
    /// macOS's `security`.
    pub fn system() -> Self {
        // The tests run a fake `security`; one that reached for the real
        // Keychain stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake security");
        }
        SharedLogin { program: PathBuf::from("/usr/bin/security"), timeout: TIMEOUT }
    }

    /// A fake `security` for the tests, with its own timeout.
    #[cfg(feature = "test-support")]
    pub fn fake(program: impl Into<PathBuf>, timeout: Duration) -> Self {
        SharedLogin { program: program.into(), timeout }
    }

    /// Puts `token` on the shared login. Antigravity.app and agy see it the
    /// next time they start.
    pub fn write(&self, token: &TokenData) -> Result<(), CredentialError> {
        self.write_payload(&build_payload(token))
    }

    /// Updates the item in place (`-U`) so a failed write leaves the existing login intact, and
    /// lets any app read it (`-A`), as Antigravity's own keyring does.
    ///
    /// The value goes on `security`'s command line (`-w`); the form that
    /// prompts for it needs a terminal. A process of the same user that lists
    /// arguments can see it for the moment `security` runs; that user can read the item (`-A`)
    /// anyway.
    fn write_payload(&self, payload_json: &str) -> Result<(), CredentialError> {
        let keychain_value = keychain_value(payload_json);
        let args = [
            "add-generic-password",
            "-U",
            "-s",
            SERVICE,
            "-a",
            ACCOUNT,
            "-w",
            &keychain_value,
            "-A",
        ]
        .map(String::from);
        let output = runner::run(&self.program, &args, self.timeout).map_err(|e| match e {
            RunError::Timeout(_) => CredentialError::Timeout,
            other => CredentialError::Spawn(other.to_string()),
        })?;
        if output.code != Some(0) {
            return Err(CredentialError::Failed {
                code: output.code,
                message: runner::first_line(&output.stderr),
            });
        }
        Ok(())
    }
}

/// Writes the shared credential (Windows).
#[cfg(windows)]
pub struct SharedLogin {
    store: std::sync::Arc<dyn crate::os::Credentials>,
}

#[cfg(windows)]
impl SharedLogin {
    /// Windows' Credential Manager.
    pub fn system() -> Self {
        // The tests give a fake store; one that reached for the real
        // Credential Manager stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake Credential Manager");
        }
        SharedLogin { store: std::sync::Arc::new(CredentialManager) }
    }

    /// A fake store for the tests.
    #[cfg(feature = "test-support")]
    pub fn fake(store: impl crate::os::Credentials + 'static) -> Self {
        SharedLogin { store: std::sync::Arc::new(store) }
    }

    /// Puts `token` on the shared login. Antigravity and agy see it the
    /// next time they start.
    pub fn write(&self, token: &TokenData) -> Result<(), CredentialError> {
        self.write_payload(&build_payload(token))
    }

    /// The Windows write: the payload's bytes as they are, which
    /// `CredWriteW` puts in place of the credential that was there.
    fn write_payload(&self, payload_json: &str) -> Result<(), CredentialError> {
        self.store
            .write(TARGET, ACCOUNT, payload_json.as_bytes())
            .map_err(|e| CredentialError::Refused(e.to_string()))
    }
}

/// The shared credential in Credential Manager, written as go-keyring
/// writes it: a generic credential kept for this user on this
/// computer (`CRED_PERSIST_LOCAL_MACHINE`, which does not roam). The app
/// never reads or deletes it: both refuse.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CredentialManager;

#[cfg(windows)]
impl crate::os::Credentials for CredentialManager {
    fn read(&self, _target: &str) -> std::io::Result<Option<Vec<u8>>> {
        Err(std::io::Error::other("the app never reads the shared Antigravity login"))
    }

    fn write(&self, target: &str, account: &str, blob: &[u8]) -> std::io::Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Security::Credentials::{
            CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
        };
        let wide = |text: &str| -> Vec<u16> {
            std::ffi::OsStr::new(text).encode_wide().chain(std::iter::once(0)).collect()
        };
        let mut name = wide(target);
        let mut user = wide(account);
        let size = u32::try_from(blob.len()).map_err(|_| std::io::Error::other("too large"))?;
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: size,
            CredentialBlob: blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: user.as_mut_ptr(),
            ..Default::default()
        };
        // SAFETY: every pointer in `credential` is to a live buffer, which
        // CredWriteW only reads. It replaces an existing credential without
        // deleting it first, so a refused write leaves the old one.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    fn delete(&self, _target: &str) -> std::io::Result<()> {
        Err(std::io::Error::other("the app never deletes the shared Antigravity login"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_is_antigravitys_json_in_go_keyrings_base64() {
        let mut token = TokenData::new(
            "fake-access".into(),
            "fake-refresh".into(),
            3599,
            Some("someone@example.com".into()),
            Some("project-x".into()),
            None,
            1_768_548_000,
        );
        token.id_token = Some("fake-id".into());
        let payload = build_payload(&token);
        assert_eq!(
            payload,
            r#"{"token":{"access_token":"fake-access","token_type":"Bearer","refresh_token":"fake-refresh","expiry":"2026-01-16T08:19:59.000000Z"},"auth_method":"consumer"}"#
        );
        let value = keychain_value(&payload);
        let b64 = value.strip_prefix("go-keyring-base64:").unwrap();
        assert_eq!(STANDARD.decode(b64).unwrap(), payload.as_bytes());
    }
}
