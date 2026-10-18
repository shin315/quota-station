//! The `cswap-prefer-team` switcher, the one engine that switches Claude on
//! its own (F07): what it would do now (`--dry-run`), what it did last (its
//! log), and whether launchd runs it.
//!
//! The app never runs the switcher for real. `--dry-run` reads claude-swap's
//! accounts, prints the decision and exits; with the pause guard installed it
//! prints `paused …` instead while the pause flag holds.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::local_time;
use crate::runner::{self, RunError};

pub const SCRIPT: &str = ".local/bin/cswap-prefer-team";
pub const LOG: &str = "Library/Logs/cswap-prefer-team.log";
/// The switcher's LaunchAgent, every 3 minutes.
pub const SERVICE: &str = "com.cswap.prefer-team";
/// claude-swap's own automatic switcher, which must stay unloaded: two
/// engines would switch against each other.
pub const AUTO_SERVICE: &str = "com.cswap.auto";
pub const LAUNCHCTL: &str = "/bin/launchctl";
/// `--dry-run` reads `cswap list --json`, which may ask the usage endpoint.
pub const DRY_RUN_TIMEOUT: Duration = Duration::from_secs(30);
const LAUNCHCTL_TIMEOUT: Duration = Duration::from_secs(5);
/// How much of the end of the log is read for its last line.
const LOG_TAIL: u64 = 8 * 1024;

pub fn script(home: &Path) -> PathBuf {
    home.join(SCRIPT)
}

pub fn log_file(home: &Path) -> PathBuf {
    home.join(LOG)
}

/// The switcher's decision line, in parts the UI can put into words. Times
/// are the switcher's `HH:MM`, local.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Decision {
    /// `stay`: the active account still has room.
    Stay,
    /// `stay (slot N usage unknown)`: no fresh reading to decide on.
    Unknown {
        slot: u32,
    },
    /// `stay (waiting for slot N at HH:MM)`: no room anywhere, and the active
    /// account resets first.
    Waiting {
        slot: u32,
        at: String,
    },
    Switch {
        slot: u32,
        why: Why,
    },
    /// `wait (no account with room)`.
    Wait,
    /// The pause guard: `paused until 2026-10-16T18:30: reason`.
    Paused {
        until: Option<String>,
        reason: Option<String>,
    },
    /// A line this build does not know; the UI shows it as written.
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Why {
    /// `ends for good at HH:MM, use it first`: a slot leaving the rotation.
    Expiring {
        at: String,
    },
    /// `ends HH:MM with P% left, before slot A`: early switching.
    Early {
        at: String,
        left: u32,
        before: u32,
    },
    /// `P%, ends HH:MM` or `P%, window not started`: the target's use so far
    /// and when its five-hour window ends.
    Room {
        used: f64,
        ends: Option<String>,
    },
    /// `P%, no Team slot with room`: daytime overflow to the personal slot.
    Personal {
        used: f64,
    },
    /// `park: no room anywhere, it resets first at HH:MM`.
    Park {
        at: String,
    },
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRun {
    pub decision: Decision,
    /// The decision line as the switcher printed it.
    pub raw: String,
}

pub fn parse_decision(line: &str) -> Decision {
    let line = line.trim();
    // Only the guard's two forms: other text after `paused` is not the guard.
    if let Some(rest) = line.strip_prefix("paused ") {
        let until = rest.strip_prefix("until ").is_some_and(|u| !u.trim().is_empty());
        if until || rest == "indefinitely" || rest.starts_with("indefinitely:") {
            return paused(rest);
        }
        return Decision::Other;
    }
    let (head, detail) = match line.split_once(" (") {
        Some((head, rest)) => (head, rest.strip_suffix(')').unwrap_or(rest)),
        None => (line, ""),
    };
    let mut words = head.split_whitespace();
    let verb = words.next();
    let slot = words.next().and_then(|w| w.parse::<u32>().ok());
    match (verb, slot) {
        (Some("stay"), None) if detail.is_empty() => Decision::Stay,
        (Some("stay"), None) => stay(detail),
        (Some("switch"), Some(slot)) => Decision::Switch { slot, why: why(detail) },
        (Some("wait"), None) => Decision::Wait,
        _ => Decision::Other,
    }
}

fn stay(detail: &str) -> Decision {
    if let Some(slot) = detail
        .strip_prefix("slot ")
        .and_then(|r| r.strip_suffix(" usage unknown"))
        .and_then(|n| n.parse().ok())
    {
        return Decision::Unknown { slot };
    }
    if let Some((slot, at)) =
        detail.strip_prefix("waiting for slot ").and_then(|r| r.split_once(" at "))
    {
        if let Ok(slot) = slot.parse() {
            return Decision::Waiting { slot, at: at.to_string() };
        }
    }
    Decision::Other
}

fn why(detail: &str) -> Why {
    if let Some(rest) = detail.strip_prefix("ends for good at ") {
        let at = rest.split(',').next().unwrap_or(rest);
        return Why::Expiring { at: at.to_string() };
    }
    if let Some(rest) = detail.strip_prefix("park: ") {
        return match rest.rsplit(' ').next() {
            Some(at) => Why::Park { at: at.to_string() },
            None => Why::Other,
        };
    }
    if let Some(rest) = detail.strip_prefix("ends ") {
        // `HH:MM with P% left, before slot A`
        let parts = rest.split_once(" with ").and_then(|(at, r)| {
            r.split_once("% left, before slot ").map(|(left, before)| (at, left, before))
        });
        if let Some((at, left, before)) = parts {
            if let (Ok(left), Ok(before)) = (left.parse(), before.parse()) {
                return Why::Early { at: at.to_string(), left, before };
            }
        }
        return Why::Other;
    }
    let Some((used, rest)) = detail.split_once("%, ") else { return Why::Other };
    let Ok(used) = used.parse::<f64>() else { return Why::Other };
    match rest {
        "window not started" => Why::Room { used, ends: None },
        "no Team slot with room" => Why::Personal { used },
        _ => match rest.strip_prefix("ends ") {
            Some(at) => Why::Room { used, ends: Some(at.to_string()) },
            None => Why::Other,
        },
    }
}

/// `until 2026-10-16T18:30: reason`, `indefinitely: reason`, or either
/// without the reason.
fn paused(rest: &str) -> Decision {
    let (until, reason) = if let Some(r) = rest.strip_prefix("until ") {
        let (until, reason) = r.split_once(' ').unwrap_or((r, ""));
        (Some(until.trim_end_matches(':').to_string()), reason)
    } else {
        (None, rest.strip_prefix("indefinitely").unwrap_or(rest).trim_start_matches(':'))
    };
    let reason = reason.trim();
    Decision::Paused { until, reason: (!reason.is_empty()).then(|| reason.to_string()) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DryRunError {
    NotFound,
    Timeout,
    Failed(String),
    /// Ran and printed no decision: the switcher prints nothing when
    /// `cswap list --json` fails.
    NoDecision,
}

impl DryRunError {
    pub fn kind(&self) -> &'static str {
        match self {
            DryRunError::NotFound => "notFound",
            DryRunError::Timeout => "timeout",
            DryRunError::Failed(_) => "failed",
            DryRunError::NoDecision => "noDecision",
        }
    }
}

/// `cswap-prefer-team --dry-run`: the decision it would act on now.
pub fn dry_run(script: &Path) -> Result<DryRun, DryRunError> {
    let out =
        runner::run(script, &["--dry-run".to_string()], DRY_RUN_TIMEOUT).map_err(|e| match e {
            RunError::NotFound(_) | RunError::NotConfigured => DryRunError::NotFound,
            RunError::Timeout(_) => DryRunError::Timeout,
            other => DryRunError::Failed(other.to_string()),
        })?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().map(str::trim).find(|l| !l.is_empty());
    match line {
        Some(line) => Ok(DryRun { decision: parse_decision(line), raw: line.to_string() }),
        None if out.code == Some(0) => Err(DryRunError::NoDecision),
        None => Err(DryRunError::Failed(runner::first_line(&out.stderr))),
    }
}

/// One line of the switcher's log.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// When it was written, RFC 3339.
    pub at: Option<String>,
    pub event: LogEvent,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LogEvent {
    /// `switch to slot N (…)`
    Switch {
        slot: u32,
    },
    /// `switch to slot N failed`
    SwitchFailed {
        slot: u32,
    },
    /// `primed slot N (exit X)`
    Primed {
        slot: u32,
    },
    /// `pause ended (until …)`, written by the pause guard.
    PauseEnded,
    Other,
}

/// `2026-10-16 12:51:44 switch to slot 4 (4.0%, ends 15:00)`
pub fn parse_log_line(line: &str) -> LogLine {
    let line = line.trim();
    let at = local_time::instant(line).and_then(|t| t.format(&Rfc3339).ok());
    let text = if at.is_some() { line.get(20..).unwrap_or("") } else { line };
    let slot_after = |prefix: &str| {
        text.strip_prefix(prefix)
            .and_then(|r| r.split_whitespace().next())
            .and_then(|n| n.parse::<u32>().ok())
    };
    let event = if let Some(slot) = slot_after("switch to slot ") {
        if text.ends_with(" failed") {
            LogEvent::SwitchFailed { slot }
        } else {
            LogEvent::Switch { slot }
        }
    } else if let Some(slot) = slot_after("primed slot ") {
        LogEvent::Primed { slot }
    } else if text.starts_with("pause ended") {
        LogEvent::PauseEnded
    } else {
        LogEvent::Other
    };
    LogLine { at, event, raw: line.to_string() }
}

/// The last line of the switcher's log, reading only its end.
pub fn last_log_line(file: &Path) -> Option<LogLine> {
    let mut f = std::fs::File::open(file).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(LOG_TAIL))).ok()?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    text.lines().rev().find(|l| !l.trim().is_empty()).map(parse_log_line)
}

/// Whether launchd has `label` loaded for this user: `launchctl list` exits
/// 0 when it has and 113 when it does not. `None` when that cannot be told.
pub fn service_loaded(launchctl: &Path, label: &str) -> Option<bool> {
    let out =
        runner::run(launchctl, &["list".to_string(), label.to_string()], LAUNCHCTL_TIMEOUT).ok()?;
    match out.code {
        Some(0) => Some(true),
        Some(113) => Some(false),
        _ => None,
    }
}
