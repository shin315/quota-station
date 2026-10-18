//! One manual switch of the Claude account, start to finish (F04): run
//! `cswap switch N --json`, pause the switcher when the owner chose to hold
//! the account, wait for things to settle, then read again which account is
//! active and say what really happened.
//!
//! The re-read matters because the switcher may run at the same moment.
//! claude-swap's locks keep the two writes apart, so the later one wins; the
//! switcher's log says whether it was the switcher. It skips quota-axi's
//! cache, which may still hold the reading of the account before the switch.

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::axi::Response;
use crate::cswap::{self, SwitchError};
use crate::local_time;
use crate::pause::{self, Flag};
use crate::runner::{self, ProviderId};
use crate::switcher::{self, LogEvent};

/// How long after the switch the active account is read again.
pub const SETTLE: Duration = Duration::from_secs(5);

/// The programs and files one switch uses.
#[derive(Debug, Clone)]
pub struct Tools {
    pub cswap: Option<PathBuf>,
    pub quota_axi: Option<PathBuf>,
    pub pause_file: PathBuf,
    pub log_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    /// When the switcher may switch again, RFC 3339.
    pub until: String,
    /// The pause flag could not be written: the switcher is not held back.
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Outcome {
    /// The re-read shows `slot` active.
    Switched { slot: u32, hold: Option<Hold> },
    /// claude-swap switched, but the re-read shows `active`: a later switch
    /// won. `by_switcher_at` is set when the switcher's log has that switch.
    Raced { slot: u32, active: u32, by_switcher_at: Option<String>, hold: Option<Hold> },
    /// claude-swap switched, and the active account could not be read again.
    Unconfirmed { slot: u32, hold: Option<Hold> },
    /// The switch did not land. `active` is the account read again when
    /// claude-swap stopped without an answer (timed out, or ended without
    /// one), to show what is in use; `None` when claude-swap answered.
    Failed { slot: u32, error: String, detail: String, active: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    /// The Claude report read after settling, to show at once.
    pub report: Option<Response>,
}

/// Switches to `slot`. With `hold_until`, the switcher is paused until then
/// after the switch, so it does not switch straight back. Blocks for
/// `settle` before the re-read.
pub fn switch_claude(
    tools: &Tools,
    slot: u32,
    hold_until: Option<OffsetDateTime>,
    settle: Duration,
) -> Done {
    let failed = |error: &SwitchError, active: Option<u32>, report: Option<Response>| Done {
        outcome: Outcome::Failed {
            slot,
            error: error.kind().to_string(),
            detail: match error {
                SwitchError::NotFound => String::new(),
                other => other.to_string(),
            },
            active,
        },
        report,
    };
    let Some(cswap_path) = &tools.cswap else {
        return failed(&SwitchError::NotFound, None, None);
    };
    let started = OffsetDateTime::now_utc();
    if let Err(e) = cswap::switch(cswap_path, slot) {
        if !matches!(e, SwitchError::Timeout | SwitchError::Exit { .. }) {
            return failed(&e, None, None);
        }
        // claude-swap was stopped mid-switch, or ended without saying how it
        // went: the switch may have landed, so look before saying it did not.
        thread::sleep(settle);
        let report = fresh_read(tools);
        let active = active_slot(report.as_ref());
        if active != Some(slot) {
            return failed(&e, active, report);
        }
        let hold = hold(tools, slot, hold_until, started);
        return Done { outcome: Outcome::Switched { slot, hold }, report };
    }

    let hold = hold(tools, slot, hold_until, started);
    thread::sleep(settle);
    let report = fresh_read(tools);
    let outcome = match active_slot(report.as_ref()) {
        Some(a) if a == slot => Outcome::Switched { slot, hold },
        Some(active) => Outcome::Raced {
            slot,
            active,
            by_switcher_at: switcher_switch(&tools.log_file, active, started),
            hold,
        },
        None => Outcome::Unconfirmed { slot, hold },
    };
    Done { outcome, report }
}

/// Pauses the switcher until `until` for a manual switch to `slot` that
/// started at `started`. Nothing is written for a hold already over, or when
/// a pause that lasts at least as long is on: a hold never shortens the
/// owner's pause.
///
/// The flag keeps local time to the minute, and the sheet shows the end to
/// the minute (`đến 15:20`), so the hold ends at the minute the owner saw:
/// up to 59 seconds before a duration counted from the second.
fn hold(
    tools: &Tools,
    slot: u32,
    until: Option<OffsetDateTime>,
    started: OffsetDateTime,
) -> Option<Hold> {
    let now = OffsetDateTime::now_utc();
    let until = until.filter(|until| *until > now)?;
    let minute = local_time::minute(until);
    if let Ok(Some(current)) = pause::read(&tools.pause_file) {
        if current.holds(&local_time::stamp(now)) && current.holds(&minute) {
            return None;
        }
    }
    let flag = Flag {
        until: Some(minute.clone()),
        reason: Some(pause::hold_reason(slot, &local_time::clock(started))),
    };
    Some(Hold {
        until: local_time::instant(&minute).unwrap_or(until).format(&Rfc3339).unwrap_or_default(),
        failed: pause::write(&tools.pause_file, &flag).is_err(),
    })
}

fn fresh_read(tools: &Tools) -> Option<Response> {
    tools.quota_axi.as_ref().and_then(|p| runner::read_fresh(p, ProviderId::Claude).ok())
}

fn active_slot(report: Option<&Response>) -> Option<u32> {
    report?.primary("claude")?.claude_swap.as_ref()?.active_slot
}

/// When the switcher's last log line is its switch to `slot`, written after
/// `since`: the time of that switch.
fn switcher_switch(log: &std::path::Path, slot: u32, since: OffsetDateTime) -> Option<String> {
    let line = switcher::last_log_line(log)?;
    let at = line.at?;
    let when = OffsetDateTime::parse(&at, &Rfc3339).ok()?;
    // The log is written to the second; allow the switch's own second.
    let fresh = when >= since.replace_nanosecond(0).unwrap_or(since);
    (line.event == LogEvent::Switch { slot } && fresh).then_some(at)
}
