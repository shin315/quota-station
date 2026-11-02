// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! One manual switch of the Claude account, start to finish (F04): switch
//! Claude Code's login with the app's own [`crate::claude::switch`], wait
//! for things to settle, then look again at which account is active and say
//! what really happened: another program may have switched right after.
//!
//! An extension may act around the switch ([`Hooks::manual_switch`],
//! [`AroundSwitch`]): refuse it before anything is switched, act once it
//! landed, add its own keys to the outcome the window gets and to the
//! switch's line in the history journal.

use std::thread;
use std::time::Duration;

use serde::Serialize;
use time::OffsetDateTime;

use crate::claude::locks::LockError;
use crate::claude::state;
use crate::claude::store::Store;
use crate::claude::switch::{self, SwitchError};
use crate::ext::{Fields, Hooks};
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
    Switched {
        slot: u32,
        /// The extension's keys ([`AroundSwitch::looked`]), at the end.
        #[serde(flatten)]
        ext: Fields,
    },
    /// The switch landed, but the second look shows `active`: a later switch
    /// won.
    Raced {
        slot: u32,
        active: u32,
        /// The extension's keys ([`AroundSwitch::looked`]), at the end.
        #[serde(flatten)]
        ext: Fields,
    },
    /// The switch landed, and the active account could not be told after.
    Unconfirmed {
        slot: u32,
        /// The extension's keys ([`AroundSwitch::looked`]), at the end.
        #[serde(flatten)]
        ext: Fields,
    },
    /// The switch did not land: [`SwitchError::kind`], or the error of the
    /// extension's refusal ([`Refusal`]). `active` is the account in use
    /// when the login could not be put back as it was.
    Failed { slot: u32, error: String, detail: String, active: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    /// The Claude report after settling, from the app's own readings, to
    /// show at once.
    pub report: Option<Response>,
    /// The extension's keys for the switch's line in the history journal
    /// ([`AroundSwitch::journal`]); none when the switch did not land.
    pub journal: Fields,
}

/// An extension's refusal of a manual switch ([`AroundSwitch::before`]):
/// nothing is switched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The error the window gets ([`Outcome::Failed`]'s `error`).
    pub error: String,
    /// Its words for the window ([`Outcome::Failed`]'s `detail`).
    pub detail: String,
    /// What `quota-station claude switch` prints on stderr after its own
    /// `quota-station claude switch: `, before it exits 1.
    pub message: String,
}

/// What an extension does around one manual switch of the Claude account,
/// from the moment the owner asks for it ([`Hooks::manual_switch`]). Every
/// method has a default that does nothing.
pub trait AroundSwitch: Send {
    /// Asked first, before anything is switched, by the app's switch and by
    /// `quota-station claude switch`: a refusal stops the switch. The
    /// default refuses nothing.
    fn before(&mut self, store: &Store) -> Option<Refusal> {
        let _ = store;
        None
    }

    /// The switch to `slot` landed, and the app is about to wait before it
    /// looks again ([`switch_claude`]'s `settle`).
    fn landed(&mut self, store: &Store, slot: u32) {
        let _ = (store, slot);
    }

    /// The look after the wait: `active` is the account in use then, `None`
    /// when it could not be told. The keys returned go at the end of the
    /// outcome the window gets ([`Outcome`]). The default adds none.
    fn looked(&mut self, store: &Store, slot: u32, active: Option<u32>) -> Fields {
        let _ = (store, slot, active);
        Fields::new()
    }

    /// The keys the switch's line in the history journal keeps, before its
    /// `result` ([`crate::history::manual_claude`]); asked once the switch
    /// landed and was looked at. The default keeps none.
    fn journal(&self) -> Fields {
        Fields::new()
    }
}

/// Switches to `slot`. `settle` runs between the switch and the second
/// look: [`settle`] waits [`SETTLE`]. `around` is what the extension does
/// around this switch ([`Hooks::manual_switch`]), none for nothing. The
/// report after is read with the pool's policy `hooks` give
/// ([`Hooks::claude_pool`]).
pub fn switch_claude(
    tools: &Tools,
    slot: u32,
    mut around: Option<Box<dyn AroundSwitch>>,
    settle: &dyn Fn(),
    hooks: &dyn Hooks,
) -> Done {
    let failed = |error: &str, detail: String, active: Option<u32>| Done {
        outcome: Outcome::Failed { slot, error: error.to_string(), detail, active },
        report: None,
        journal: Fields::new(),
    };
    if let Some(refusal) = around.as_mut().and_then(|a| a.before(&tools.store)) {
        return failed(&refusal.error, refusal.detail, None);
    }
    if let Err(e) = switch::switch(&tools.store, slot) {
        let active = matches!(e, SwitchError::NotConfirmed { restored: false, .. })
            .then(|| switch::active_slot(&tools.store))
            .flatten();
        return failed(e.kind(), detail(&e), active);
    }
    let _ = state::write(&tools.store, Some(slot), OffsetDateTime::now_utc());

    if let Some(a) = around.as_mut() {
        a.landed(&tools.store, slot);
    }
    settle();
    let active = switch::active_slot(&tools.store);
    let ext = around.as_mut().map_or_else(Fields::new, |a| a.looked(&tools.store, slot, active));
    let outcome = match active {
        Some(a) if a == slot => Outcome::Switched { slot, ext },
        Some(active) => Outcome::Raced { slot, active, ext },
        None => Outcome::Unconfirmed { slot, ext },
    };
    Done {
        outcome,
        report: quota_claude::own_now(&tools.store, hooks, OffsetDateTime::now_utc()),
        journal: around.as_ref().map_or_else(Fields::new, |a| a.journal()),
    }
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
