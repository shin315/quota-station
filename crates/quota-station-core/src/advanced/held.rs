// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The accounts the owner holds out of the rotation (still switched to by
//! hand), as the advanced part keeps them on the app's Claude accounts
//! through the hooks ([`crate::ext::Hooks::claude_account_keys`],
//! [`AccountKeys`]): `disabled` on each account of `claude/accounts.json`
//! (written on every account, `false` for one in the rotation; a file
//! without it reads `false`), `enabled` on each account of
//! `claude/state.json` and `quota-station claude status --json`, and
//! `held out` before the name on `quota-station claude status`'s line. The
//! read of the accounts copies `disabled` onto each account's slot
//! ([`slot`]), where the pool policy (`pool.rs`), the snapshot
//! (`snapshot.rs`) and the rotation (`claude::rotate`) read it.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::claude::manage::{self, ManageError};
use crate::claude::state::AccountOut;
use crate::claude::store::{Account, AccountKeys, Store};
use crate::ext::{Fields, Hooks};
use crate::quota::Slot;

/// The key of an account held out, in the accounts file and on the read's
/// slots.
pub const DISABLED: &str = "disabled";
/// The key of an account in the rotation, in the state file.
pub const ENABLED: &str = "enabled";
/// The words before a held-out account's name on `claude status`.
pub const WORDS: &str = "held out";

/// The advanced part's keys of the Claude accounts.
#[derive(Debug, Clone, Copy, Default)]
pub struct Held;

impl AccountKeys for Held {
    fn keep(&self, read: &Fields) -> Result<Fields, String> {
        let disabled = match read.get(DISABLED) {
            None => false,
            Some(v) => bool::deserialize(v).map_err(|e| e.to_string())?,
        };
        Ok(keys(disabled))
    }

    fn state(&self, kept: &Fields) -> Fields {
        let mut shown = Fields::new();
        shown.insert(ENABLED, Value::Bool(!is_set(kept)));
        shown
    }

    fn status_words(&self, shown: &Fields) -> Option<&'static str> {
        (shown.get(ENABLED) == Some(&Value::Bool(false))).then_some(WORDS)
    }
}

/// The hooks the advanced part gives the Claude accounts' store: [`Held`]
/// on each account.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeldHooks;

impl Hooks for HeldHooks {
    fn claude_account_keys(&self) -> Option<Arc<dyn AccountKeys>> {
        Some(Arc::new(Held))
    }
}

fn keys(disabled: bool) -> Fields {
    let mut kept = Fields::new();
    kept.insert(DISABLED, Value::Bool(disabled));
    kept
}

fn is_set(fields: &Fields) -> bool {
    fields.get(DISABLED) == Some(&Value::Bool(true))
}

/// Whether the account is held out.
pub fn disabled(account: &Account) -> bool {
    is_set(&account.ext)
}

/// Holds the account out (`true`) or puts it back.
pub fn set(account: &mut Account, disabled: bool) {
    account.ext.insert(DISABLED, Value::Bool(disabled));
}

/// Whether the read's slot is of an account held out.
pub fn slot(slot: &Slot) -> bool {
    is_set(&slot.ext)
}

/// Whether the state file's account is in the rotation.
pub fn enabled(account: &AccountOut) -> bool {
    account.ext.get(ENABLED).and_then(Value::as_bool).unwrap_or(true)
}

/// The keys the state file gives an account in the rotation or not.
pub fn shown(enabled: bool) -> Fields {
    Held.state(&keys(!enabled))
}

/// Puts the account in the rotation, or holds it out (it can still be
/// switched to by hand).
pub fn set_enabled(store: &Store, slot: u32, enabled: bool) -> Result<(), ManageError> {
    manage::change(store, slot, |a| set(a, !enabled))
}
