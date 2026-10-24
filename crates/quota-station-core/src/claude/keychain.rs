// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// The `security` commands are claude-swap's (`macos_keychain.py`, MIT,
// https://github.com/realiti4/claude-swap): the read with `-w` and exit 44
// for a missing item, the write piped to `security -i` with the value as hex
// (`-X`) so it stays off the command line, the command-line fallback for a
// line too long for `security -i`, and the 5-second limit.

//! Generic passwords in the login Keychain, through macOS's own
//! `/usr/bin/security`, the way claude-swap and Claude Code keep theirs: an
//! item `security` made is one `security` reads without a prompt.
//!
//! A value is never printed or logged here: errors carry `security`'s exit
//! code and its first line on stderr, which never holds the value.

use std::path::PathBuf;
use std::time::Duration;

use crate::runner::{self, RunError};

/// Each `security` call. Short on purpose: a switch runs inside Claude Code's
/// locks, which Claude Code waits about 9 seconds for.
pub const TIMEOUT: Duration = Duration::from_secs(5);
/// `security`'s exit code for "the specified item could not be found".
const NOT_FOUND: i32 = 44;
/// The longest line `security -i` reads whole (claude-swap's limit).
const STDIN_LINE_LIMIT: usize = 4096 - 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeychainError {
    Timeout,
    Spawn(String),
    /// `security` failed: its exit code and first stderr line.
    Failed {
        code: Option<i32>,
        message: String,
    },
}

impl std::fmt::Display for KeychainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeychainError::Timeout => {
                write!(f, "security did not answer within {}s", TIMEOUT.as_secs())
            }
            KeychainError::Spawn(e) => write!(f, "could not run security: {e}"),
            KeychainError::Failed { code: Some(c), message } => {
                write!(f, "security exited {c}: {message}")
            }
            KeychainError::Failed { code: None, message } => {
                write!(f, "security was stopped: {message}")
            }
        }
    }
}

impl std::error::Error for KeychainError {}

/// The `security` tool the app runs.
#[derive(Debug, Clone)]
pub struct Security {
    program: PathBuf,
}

impl Security {
    /// macOS's `security`.
    pub fn system() -> Self {
        // The tests run a fake `security`; one that reached for the real
        // Keychain stops here.
        if cfg!(feature = "test-support") {
            panic!("tests must use a fake security");
        }
        Security { program: PathBuf::from("/usr/bin/security") }
    }

    /// A fake `security` for the tests.
    #[cfg(feature = "test-support")]
    pub fn fake(program: impl Into<PathBuf>) -> Self {
        Security { program: program.into() }
    }

    fn run(&self, args: &[&str], input: Option<&str>) -> Result<runner::Output, KeychainError> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        runner::run_with(&self.program, &args, &[], input, TIMEOUT).map_err(|e| match e {
            RunError::Timeout(_) => KeychainError::Timeout,
            other => KeychainError::Spawn(other.to_string()),
        })
    }

    fn failed(out: &runner::Output) -> KeychainError {
        KeychainError::Failed { code: out.code, message: runner::first_line(&out.stderr) }
    }

    /// The item's value, or `None` when there is no such item.
    pub fn get(&self, service: &str, account: &str) -> Result<Option<String>, KeychainError> {
        let out = self.run(&["find-generic-password", "-a", account, "-w", "-s", service], None)?;
        match out.code {
            Some(0) => {
                let text = String::from_utf8(out.stdout).map_err(|_| KeychainError::Failed {
                    code: Some(0),
                    message: "the value is not text".into(),
                })?;
                // `-w` ends the value with one newline.
                Ok(Some(text.strip_suffix('\n').unwrap_or(&text).to_string()))
            }
            Some(NOT_FOUND) => Ok(None),
            _ => Err(Self::failed(&out)),
        }
    }

    /// Whether the item is there, without reading its value.
    pub fn exists(&self, service: &str, account: &str) -> Result<bool, KeychainError> {
        let out = self.run(&["find-generic-password", "-a", account, "-s", service], None)?;
        match out.code {
            Some(0) => Ok(true),
            Some(NOT_FOUND) => Ok(false),
            _ => Err(Self::failed(&out)),
        }
    }

    /// Adds or replaces the item (`-U`). On a failure the item that was
    /// there stays as it was.
    pub fn set(&self, service: &str, account: &str, value: &str) -> Result<(), KeychainError> {
        let hex: String = value.bytes().map(|b| format!("{b:02x}")).collect();
        let line = format!(
            "add-generic-password -U -a {} -s {} -X {hex}\n",
            quote(account),
            quote(service)
        );
        let out = if line.len() <= STDIN_LINE_LIMIT {
            self.run(&["-i"], Some(&line))?
        } else {
            // Too long for one `security -i` line: the hex goes on the
            // command line instead, as claude-swap does.
            self.run(
                &["add-generic-password", "-U", "-a", account, "-s", service, "-X", &hex],
                None,
            )?
        };
        match out.code {
            Some(0) => Ok(()),
            _ => Err(Self::failed(&out)),
        }
    }

    /// Removes the item; one that is not there counts as removed.
    pub fn delete(&self, service: &str, account: &str) -> Result<(), KeychainError> {
        let out = self.run(&["delete-generic-password", "-a", account, "-s", service], None)?;
        match out.code {
            Some(0) | Some(NOT_FOUND) => Ok(()),
            _ => Err(Self::failed(&out)),
        }
    }
}

/// A value for a `security -i` line, which is read shell-style.
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The account name of Claude Code's own item: `$USER`, else the user's
/// name, as Claude Code's `getUsername()` gives it.
pub fn user_account() -> String {
    if let Some(user) = std::env::var_os("USER").filter(|u| !u.is_empty()) {
        return user.to_string_lossy().into_owned();
    }
    // SAFETY: `getpwuid` returns a pointer into static storage or null; the
    // name is copied out at once.
    unsafe {
        let pw = libc::getpwuid(libc::geteuid());
        if !pw.is_null() && !(*pw).pw_name.is_null() {
            if let Ok(name) = std::ffi::CStr::from_ptr((*pw).pw_name).to_str() {
                return name.to_string();
            }
        }
    }
    "claude-code-user".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_quoted_for_a_security_line() {
        assert_eq!(quote("Claude Code-credentials"), "\"Claude Code-credentials\"");
        assert_eq!(quote(r#"a"b\c"#), r#""a\"b\\c""#);
    }
}
