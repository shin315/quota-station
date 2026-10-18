//! claude-swap: switching the active Claude account (F04), and reading what
//! each account is and how much of each window it used.
//!
//! The app runs two claude-swap commands. `cswap switch N --json` runs only
//! when the owner confirms it; claude-swap holds Claude Code's credential
//! locks while it writes, so a switch never lands in the middle of a token
//! refresh or of another switch. `cswap list --json` runs with every read:
//! it only reads claude-swap's own usage cache, and it is the one place that
//! has every window of every account (quota-axi keeps only the tighter
//! number) and the account's email and organization.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::runner::{self, RunError};

/// A switch rewrites the login and Claude Code's config; claude-swap waits
/// for the credential locks first.
pub const SWITCH_TIMEOUT: Duration = Duration::from_secs(60);

/// Where claude-swap is usually installed, in the order the app tries them.
/// `uv tool install` puts it in `~/.local/bin`.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".local/bin/cswap"),
        PathBuf::from("/opt/homebrew/bin/cswap"),
        PathBuf::from("/usr/local/bin/cswap"),
    ]
}

pub fn discover(home: &Path) -> Option<PathBuf> {
    candidates(home).into_iter().find(|p| p.is_file())
}

/// `cswap list --json` reads a cache and answers in well under a second.
pub const LIST_TIMEOUT: Duration = Duration::from_secs(10);

pub fn list_args() -> Vec<String> {
    vec!["list".into(), "--json".into()]
}

pub fn switch_args(slot: u32) -> Vec<String> {
    vec!["switch".into(), slot.to_string(), "--json".into()]
}

/// What `cswap switch N --json` reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switched {
    pub switched: bool,
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchError {
    NotFound,
    Spawn(String),
    Timeout,
    /// claude-swap answered and did not switch: its error type or no-op
    /// reason, and its message with any email address taken out.
    Refused {
        kind: String,
        message: String,
    },
    /// Exited without a JSON answer.
    Exit {
        code: Option<i32>,
        message: String,
    },
}

impl std::fmt::Display for SwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwitchError::NotFound => write!(f, "cswap not found"),
            SwitchError::Spawn(e) => write!(f, "could not start cswap: {e}"),
            SwitchError::Timeout => {
                write!(f, "cswap did not answer within {}s", SWITCH_TIMEOUT.as_secs())
            }
            SwitchError::Refused { kind, message } if message.is_empty() => write!(f, "{kind}"),
            SwitchError::Refused { kind, message } => write!(f, "{kind}: {message}"),
            SwitchError::Exit { code: Some(c), message } => {
                write!(f, "cswap exited {c}: {message}")
            }
            SwitchError::Exit { code: None, message } => write!(f, "cswap was stopped: {message}"),
        }
    }
}

impl std::error::Error for SwitchError {}

impl SwitchError {
    /// A short name for the UI to choose its words by.
    pub fn kind(&self) -> &'static str {
        match self {
            SwitchError::NotFound => "notFound",
            SwitchError::Spawn(_) => "spawn",
            SwitchError::Timeout => "timeout",
            SwitchError::Refused { .. } => "refused",
            SwitchError::Exit { .. } => "exit",
        }
    }
}

// Only the account numbers are read from `from` and `to`: claude-swap puts
// the email next to them, and serde drops it.
#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    switched: Option<bool>,
    #[serde(default)]
    from: Option<AccountRef>,
    #[serde(default)]
    to: Option<AccountRef>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<ErrorBody>,
}

#[derive(Deserialize)]
struct AccountRef {
    #[serde(default)]
    number: Option<u32>,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// Runs `cswap switch <slot> --json`. Landing on `slot` counts as done even
/// when it was already active; any other answer is an error.
pub fn switch(path: &Path, slot: u32) -> Result<Switched, SwitchError> {
    let out = runner::run(path, &switch_args(slot), SWITCH_TIMEOUT).map_err(|e| match e {
        RunError::NotFound(_) | RunError::NotConfigured => SwitchError::NotFound,
        RunError::Timeout(_) => SwitchError::Timeout,
        other => SwitchError::Spawn(other.to_string()),
    })?;
    // One JSON object on stdout, pretty-printed; notices go to stderr.
    let text = String::from_utf8_lossy(&out.stdout);
    let answer = serde_json::from_str::<Answer>(text.trim()).ok();
    let Some(answer) = answer else {
        return Err(SwitchError::Exit {
            code: out.code,
            message: redact(&runner::first_line(&out.stderr)),
        });
    };
    if let Some(error) = answer.error {
        return Err(SwitchError::Refused {
            kind: redact(error.kind.as_deref().unwrap_or("error")),
            message: redact(error.message.as_deref().unwrap_or("")),
        });
    }
    let switched = Switched {
        switched: answer.switched.unwrap_or(false),
        from: answer.from.and_then(|r| r.number),
        to: answer.to.and_then(|r| r.number),
        reason: answer.reason.unwrap_or_default(),
    };
    if out.code == Some(0) && switched.to == Some(slot) {
        Ok(switched)
    } else {
        Err(SwitchError::Refused {
            kind: if switched.reason.is_empty() {
                "not-switched".into()
            } else {
                redact(&switched.reason)
            },
            message: redact(answer.message.as_deref().unwrap_or("")),
        })
    }
}

/// Takes email addresses out of a message before the UI shows it: claude-swap
/// names accounts by email, and the app names them by slot.
pub fn redact(text: &str) -> String {
    text.split(' ')
        .map(|word| if word.contains('@') { "…" } else { word })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// One window of a claude-swap account: the share used and when it resets.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    /// Percent used, 0-100.
    pub used: f64,
    /// RFC 3339; none for a five-hour window that has not started.
    pub resets_at: Option<String>,
}

/// One account as `cswap list --json` reports it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SwapAccount {
    pub number: u32,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub five_hour: Option<Usage>,
    /// None: claude-swap reports no weekly window for this account (Team seats).
    pub seven_day: Option<Usage>,
    /// The weekly window scoped to Fable.
    pub fable: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListError {
    NotFound,
    Spawn(String),
    Timeout,
    /// Exited without a list it could read.
    Unreadable(Option<i32>),
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListError::NotFound => write!(f, "cswap not found"),
            ListError::Spawn(e) => write!(f, "could not start cswap: {e}"),
            ListError::Timeout => {
                write!(f, "cswap list did not answer within {}s", LIST_TIMEOUT.as_secs())
            }
            ListError::Unreadable(Some(c)) => write!(f, "cswap list exited {c} without a list"),
            ListError::Unreadable(None) => write!(f, "cswap list was stopped"),
        }
    }
}

impl std::error::Error for ListError {}

#[derive(Deserialize)]
struct ListAnswer {
    accounts: Vec<ListAccount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListAccount {
    number: u32,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    organization_name: Option<String>,
    #[serde(default)]
    usage: Option<ListUsage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListUsage {
    #[serde(default)]
    five_hour: Option<ListWindow>,
    #[serde(default)]
    seven_day: Option<ListWindow>,
    #[serde(default)]
    scoped: Vec<ListWindow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListWindow {
    #[serde(default)]
    pct: Option<f64>,
    #[serde(default)]
    resets_at: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

impl ListWindow {
    fn usage(&self) -> Option<Usage> {
        Some(Usage { used: self.pct?.clamp(0.0, 100.0), resets_at: self.resets_at.clone() })
    }
}

/// Reads `cswap list --json`. Fields the app does not use (tokens are never
/// in it; organization ids, login expiry, pace) are dropped.
pub fn parse_list(text: &str) -> Option<Vec<SwapAccount>> {
    let answer = serde_json::from_str::<ListAnswer>(text.trim()).ok()?;
    Some(
        answer
            .accounts
            .into_iter()
            .map(|a| {
                let usage = a.usage.as_ref();
                SwapAccount {
                    number: a.number,
                    email: a.email.filter(|e| !e.trim().is_empty()),
                    organization: a.organization_name.filter(|o| !o.trim().is_empty()),
                    five_hour: usage.and_then(|u| u.five_hour.as_ref()).and_then(ListWindow::usage),
                    seven_day: usage.and_then(|u| u.seven_day.as_ref()).and_then(ListWindow::usage),
                    fable: usage
                        .and_then(|u| {
                            u.scoped.iter().find(|w| {
                                w.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case("fable"))
                            })
                        })
                        .and_then(ListWindow::usage),
                }
            })
            .collect(),
    )
}

/// Runs `cswap list --json`.
pub fn list(path: &Path) -> Result<Vec<SwapAccount>, ListError> {
    let out = runner::run(path, &list_args(), LIST_TIMEOUT).map_err(|e| match e {
        RunError::NotFound(_) | RunError::NotConfigured => ListError::NotFound,
        RunError::Timeout(_) => ListError::Timeout,
        other => ListError::Spawn(other.to_string()),
    })?;
    let text = String::from_utf8_lossy(&out.stdout);
    match parse_list(&text) {
        Some(list) if out.code == Some(0) => Ok(list),
        _ => Err(ListError::Unreadable(out.code)),
    }
}
