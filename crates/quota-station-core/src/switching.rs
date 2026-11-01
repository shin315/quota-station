// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! One manual switch of the Claude account, start to finish (F04): switch
//! Claude Code's login with the app's own [`crate::claude::switch`], wait
//! for things to settle, then look again at which account is active and say
//! what really happened: another program may have switched right after.

use std::thread;
use std::time::Duration;

use serde::Serialize;
use time::OffsetDateTime;

use crate::claude::locks::LockError;
use crate::claude::state;
use crate::claude::store::Store;
use crate::claude::switch::{self, SwitchError};
use crate::quota::{claude as quota_claude, Response};

/// How long after the switch the active account is looked at again.
pub const SETTLE: Duration = Duration::from_secs(5);

/// What one switch uses.
#[derive(Debug, Clone)]
pub struct Tools {
    pub store: Store,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Outcome {
    /// The second look shows `slot` active.
    Switched { slot: u32 },
    /// The switch landed, but the second look shows `active`: a later switch
    /// won.
    Raced { slot: u32, active: u32 },
    /// The switch landed, and the active account could not be told after.
    Unconfirmed { slot: u32 },
    /// The switch did not land: [`SwitchError::kind`]. `active` is the
    /// account in use when the login could not be put back as it was.
    Failed { slot: u32, error: String, detail: String, active: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    /// The Claude report after settling, from the app's own readings, to
    /// show at once.
    pub report: Option<Response>,
}

/// Switches to `slot`. `settle` runs between the switch and the second
/// look: [`settle`] waits [`SETTLE`].
pub fn switch_claude(tools: &Tools, slot: u32, settle: &dyn Fn()) -> Done {
    let failed = |error: &str, detail: String, active: Option<u32>| Done {
        outcome: Outcome::Failed { slot, error: error.to_string(), detail, active },
        report: None,
    };
    if let Err(e) = switch::switch(&tools.store, slot) {
        let active = matches!(e, SwitchError::NotConfirmed { restored: false, .. })
            .then(|| switch::active_slot(&tools.store))
            .flatten();
        return failed(e.kind(), detail(&e), active);
    }
    let _ = state::write(&tools.store, Some(slot), OffsetDateTime::now_utc());

    settle();
    let outcome = match switch::active_slot(&tools.store) {
        Some(a) if a == slot => Outcome::Switched { slot },
        Some(active) => Outcome::Raced { slot, active },
        None => Outcome::Unconfirmed { slot },
    };
    Done { outcome, report: quota_claude::own_now(&tools.store, OffsetDateTime::now_utc()) }
}

/// Waits [`SETTLE`].
pub fn settle() {
    thread::sleep(SETTLE);
}

/// The error's words for the sheet: none for a held lock, whose message
/// names a path on this Mac.
fn detail(e: &SwitchError) -> String {
    match e {
        SwitchError::Lock(LockError::Busy(_)) => String::new(),
        other => other.to_string(),
    }
}
