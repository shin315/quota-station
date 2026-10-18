//! One manual switch of the Claude account, start to finish (F04): run
//! `cswap switch N --json`, pause the switcher when the owner chose to hold
//! the account, wait for things to settle, then read again which account is
//! active and say what really happened.
//!
//! The re-read matters because the switcher may run at the same moment.
//! claude-swap's locks keep the two writes apart, so the later one wins; the
//! switcher's log says whether it was the switcher.

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
    /// Nothing changed.
    Failed { slot: u32, error: String, detail: String },
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
    let failed = |error: SwitchError| Done {
        outcome: Outcome::Failed {
            slot,
            error: error.kind().to_string(),
            detail: match &error {
                SwitchError::NotFound => String::new(),
                other => other.to_string(),
            },
        },
        report: None,
    };
    let Some(cswap_path) = &tools.cswap else { return failed(SwitchError::NotFound) };
    let started = OffsetDateTime::now_utc();
    if let Err(e) = cswap::switch(cswap_path, slot) {
        return failed(e);
    }

    // A hold already over by now holds nothing.
    let hold = hold_until.filter(|until| *until > started).map(|until| {
        let minute = local_time::minute(until);
        let flag = Flag {
            until: Some(minute.clone()),
            reason: Some(pause::hold_reason(slot, &local_time::clock(started))),
        };
        Hold {
            until: local_time::instant(&minute)
                .unwrap_or(until)
                .format(&Rfc3339)
                .unwrap_or_default(),
            failed: pause::write(&tools.pause_file, &flag).is_err(),
        }
    });

    thread::sleep(settle);
    let report = tools.quota_axi.as_ref().and_then(|p| runner::read(p, ProviderId::Claude).ok());
    let active = report
        .as_ref()
        .and_then(|r| r.primary("claude"))
        .and_then(|row| row.claude_swap.as_ref())
        .and_then(|swap| swap.active_slot);
    let outcome = match active {
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
