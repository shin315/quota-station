// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// The switch follows claude-swap's `_perform_switch` (`switcher.py`, MIT,
// https://github.com/realiti4/claude-swap): the same four locks in the same
// order, the outgoing account's live credential kept first (never an empty
// or wiped one, never one of an account it does not manage), then the
// target's credential and `oauthAccount`, and everything put back when a
// step fails. Unlike claude-swap it reads both back before saying it
// switched.

//! Switching Claude Code's active login to one of the app's accounts. It
//! makes no network call and never refreshes a token.

use serde::Serialize;

use super::cc;
use super::keychain::{user_account, KeychainError};
use super::locks::{self, AppLock, LockError};
use super::session;
use super::store::{Store, StoreError, Who};

/// What a switch did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Switched {
    /// The app's account Claude Code was on, when it was one of them.
    pub from: Option<u32>,
    pub to: u32,
    /// It was on that account already; nothing was written but its backup.
    pub already: bool,
}

#[derive(Debug)]
pub enum SwitchError {
    /// No such account in the app's list.
    UnknownSlot,
    /// The account has no usable credential or no `oauthAccount` stored.
    NoCredential,
    /// Claude Code is logged in to an account the app does not keep: the
    /// switch would lose that login.
    Unmanaged,
    Lock(LockError),
    Keychain(KeychainError),
    Store(StoreError),
    Io(std::io::Error),
    /// The login or the config read back is not what was written, or a
    /// write failed and they could not be put back; `restored` is false
    /// when putting them back failed too. `detail` says which step, never
    /// with a value of the login.
    NotConfirmed {
        restored: bool,
        detail: String,
    },
}

impl SwitchError {
    /// A short name for the UI to choose its words by.
    pub fn kind(&self) -> &'static str {
        match self {
            SwitchError::UnknownSlot => "unknownSlot",
            SwitchError::NoCredential => "noCredential",
            SwitchError::Unmanaged => "unmanaged",
            SwitchError::Lock(LockError::Busy(_)) => "busy",
            SwitchError::Lock(LockError::Io(_)) => "write",
            SwitchError::Keychain(_) => "keychain",
            SwitchError::Store(_) => "store",
            SwitchError::Io(_) => "write",
            SwitchError::NotConfirmed { .. } => "unconfirmed",
        }
    }
}

impl std::fmt::Display for SwitchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwitchError::UnknownSlot => write!(f, "no such account"),
            SwitchError::NoCredential => {
                write!(f, "the account has no usable stored login (none, or expired)")
            }
            SwitchError::Unmanaged => {
                write!(f, "Claude Code is logged in to an account Quota Station does not keep")
            }
            SwitchError::Lock(e) => write!(f, "{e}"),
            SwitchError::Keychain(e) => write!(f, "{e}"),
            SwitchError::Store(e) => write!(f, "{e}"),
            SwitchError::Io(e) => write!(f, "{e}"),
            SwitchError::NotConfirmed { restored: true, detail } => {
                write!(f, "{detail}; the old login is back")
            }
            SwitchError::NotConfirmed { restored: false, detail } => {
                write!(f, "{detail}, and putting the old login back failed")
            }
        }
    }
}

impl std::error::Error for SwitchError {}

impl From<LockError> for SwitchError {
    fn from(e: LockError) -> Self {
        SwitchError::Lock(e)
    }
}

impl From<KeychainError> for SwitchError {
    fn from(e: KeychainError) -> Self {
        SwitchError::Keychain(e)
    }
}

impl From<StoreError> for SwitchError {
    fn from(e: StoreError) -> Self {
        SwitchError::Store(e)
    }
}

impl From<std::io::Error> for SwitchError {
    fn from(e: std::io::Error) -> Self {
        SwitchError::Io(e)
    }
}

/// The app's account Claude Code is on now, by the identity in
/// `~/.claude.json`; none when it is on no account, or one the app does not
/// keep.
pub fn active_slot(store: &Store) -> Option<u32> {
    let who = cc::read_oauth(&store.paths).map(|o| Who::of(&o)).filter(|w| !w.email.is_empty())?;
    store.load().ok()?.find(&who.identity()).map(|a| a.slot)
}

/// Switches Claude Code to account `slot`.
pub fn switch(store: &Store, slot: u32) -> Result<Switched, SwitchError> {
    let paths = &store.paths;
    let _app = AppLock::take(paths)?;
    let list = store.load()?;
    let target = list.get(slot).ok_or(SwitchError::UnknownSlot)?.clone();
    // A session folder of the target that Claude Code refreshed holds the
    // newest copy of its login.
    session::adopt_newer(store, slot);
    let target_cred =
        store.credential(slot)?.filter(|c| cc::usable(c)).ok_or(SwitchError::NoCredential)?;
    let target_oauth = store.oauth_account(slot)?.ok_or(SwitchError::NoCredential)?;

    let _login = locks::credentials(paths)?;
    let _config = locks::config(paths)?;
    let live = cc::read_live(paths, &store.security)?;
    let live_cred = live.credential.clone().filter(|c| cc::usable(c));
    let from = match (live.who(), live_cred.as_deref()) {
        (Some(who), _) => match list.find(&who.identity()) {
            Some(a) => Some(a.slot),
            // An unknown login with a token in it would be lost.
            None if live_cred.is_some() => return Err(SwitchError::Unmanaged),
            None => None,
        },
        // No `oauthAccount` to tell whose it is: the login is one of the
        // app's only when its refresh token is.
        (None, Some(cred)) => {
            let token = cc::refresh_token(cred).ok_or(SwitchError::Unmanaged)?;
            let mut owner = None;
            for a in &list.accounts {
                if store.credential(a.slot)?.as_deref().and_then(cc::refresh_token).as_ref()
                    == Some(&token)
                {
                    owner = Some(a.slot);
                    break;
                }
            }
            Some(owner.ok_or(SwitchError::Unmanaged)?)
        }
        (None, None) => None,
    };

    // Keep the outgoing login: Claude Code may have refreshed it since the
    // app last stored it.
    if let (Some(f), Some(cred)) = (from, live_cred.as_deref()) {
        if store.credential(f)?.as_deref() != Some(cred) {
            store.set_credential(f, cred)?;
            session::refresh_profile(store, f);
            super::usage::login_replaced(store, f);
        }
        if let Some(oauth) = live.oauth.as_ref().filter(|_| live.who().is_some()) {
            if store.oauth_account(f)?.as_ref() != Some(oauth) {
                store.set_oauth_account(f, oauth)?;
            }
        }
    }
    if from == Some(slot) && live.who().is_some() {
        return Ok(Switched { from, to: slot, already: true });
    }

    let config_before = super::read_optional(&paths.claude_json())?;
    let composed = cc::merge_shared(&target_cred, live.credential.as_deref());
    let login_back = || cc::read_credential(paths, &store.security).ok().flatten();
    let put_back = || -> bool {
        if login_back() != live.credential {
            let _ = match live.credential.as_deref() {
                Some(c) => cc::write_credential(paths, &store.security, c).is_ok(),
                None => store.security.delete(cc::ACTIVE_SERVICE, &user_account()).is_ok(),
            };
        }
        login_back() == live.credential
            && cc::restore_config(paths, config_before.as_deref()).is_ok()
    };
    let failed = |step: &str| {
        let step = step.to_string();
        move |e: std::io::Error| SwitchError::NotConfirmed {
            restored: put_back(),
            detail: format!("{step} failed ({e})"),
        }
    };
    cc::write_credential(paths, &store.security, &composed).map_err(failed("writing the login"))?;
    cc::write_oauth(paths, &target_oauth).map_err(failed("writing the account to .claude.json"))?;

    let who_back = cc::read_oauth(paths).map(|o| Who::of(&o).identity());
    let login_kept = login_back().as_deref() == Some(composed.as_str());
    let who_kept = who_back.as_deref() == Some(target.identity.as_str());
    if !login_kept || !who_kept {
        let detail = match (login_kept, who_kept) {
            (false, true) => "the login read back from the Keychain is not the one written",
            (true, false) => "the account read back from .claude.json is not the target's",
            _ => "neither the login nor the account read back as written",
        };
        return Err(SwitchError::NotConfirmed { restored: put_back(), detail: detail.into() });
    }
    Ok(Switched { from, to: slot, already: false })
}
