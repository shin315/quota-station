// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// Logging a new account in through `claude auth login` in a folder of its
// own, and keeping a setup token as a credential with only an access token,
// follow claude-swap (`cswap add`, `cswap add --token`, MIT,
// https://github.com/realiti4/claude-swap).

//! Adding, removing and changing the app's Claude accounts: a new login in
//! a folder of its own (Claude Code's default login is never touched), the
//! login Claude Code is on now, a setup token, and each account's alias.
//! Every change holds the app's account lock.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::keychain::{user_account, KeychainError};
use super::locks::{AppLock, LockError};
use super::store::{self, Account, Kind, Store, StoreError, Who};
use super::{cc, session, switch, usage};

/// How long a login in the browser may take.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug)]
pub enum ManageError {
    UnknownSlot,
    /// The account is Claude Code's login now: switch to another first.
    Active,
    /// No login to add: no credential, or no account in it.
    NoLogin,
    /// The setup token is empty or not one line.
    BadToken,
    BadAlias(&'static str),
    /// `claude auth login` did not finish: why.
    Login(String),
    Cancelled,
    Lock(LockError),
    Store(StoreError),
    Keychain(KeychainError),
    Io(std::io::Error),
}

impl ManageError {
    /// A short name for the UI to choose its words by.
    pub fn kind(&self) -> &'static str {
        match self {
            ManageError::UnknownSlot => "unknownSlot",
            ManageError::Active => "active",
            ManageError::NoLogin => "noLogin",
            ManageError::BadToken => "badToken",
            ManageError::BadAlias(_) => "badAlias",
            ManageError::Login(_) => "login",
            ManageError::Cancelled => "cancelled",
            ManageError::Lock(LockError::Busy(_)) => "busy",
            ManageError::Lock(LockError::Io(_)) => "write",
            ManageError::Store(_) => "store",
            ManageError::Keychain(_) => "keychain",
            ManageError::Io(_) => "write",
        }
    }
}

impl std::fmt::Display for ManageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManageError::UnknownSlot => write!(f, "no such account"),
            ManageError::Active => write!(f, "the account is Claude Code's login now"),
            ManageError::NoLogin => write!(f, "no login to add"),
            ManageError::BadToken => write!(f, "the token is empty or not one line"),
            ManageError::BadAlias(why) => write!(f, "{why}"),
            ManageError::Login(why) => write!(f, "claude auth login: {why}"),
            ManageError::Cancelled => write!(f, "cancelled"),
            ManageError::Lock(e) => write!(f, "{e}"),
            ManageError::Store(e) => write!(f, "{e}"),
            ManageError::Keychain(e) => write!(f, "{e}"),
            ManageError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ManageError {}

impl From<LockError> for ManageError {
    fn from(e: LockError) -> Self {
        ManageError::Lock(e)
    }
}

impl From<StoreError> for ManageError {
    fn from(e: StoreError) -> Self {
        ManageError::Store(e)
    }
}

impl From<KeychainError> for ManageError {
    fn from(e: KeychainError) -> Self {
        ManageError::Keychain(e)
    }
}

impl From<std::io::Error> for ManageError {
    fn from(e: std::io::Error) -> Self {
        ManageError::Io(e)
    }
}

/// What an add did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Added {
    pub slot: u32,
    /// A new account; otherwise the account the app had got the newer login.
    pub new: bool,
}

/// Puts a login in the app: the account it already has with this identity
/// gets it in its slot, a new one the next free slot. The caller holds the
/// app's lock.
fn put(store: &Store, credential: &str, oauth: &Value, kind: Kind) -> Result<Added, ManageError> {
    let who = Who::of(oauth);
    if who.email.is_empty() || !cc::usable(credential) {
        return Err(ManageError::NoLogin);
    }
    let identity = who.identity();
    let mut list = store.load()?;
    if let Some(slot) = list.find(&identity).map(|a| a.slot) {
        store.set_credential(slot, credential)?;
        store.set_oauth_account(slot, oauth)?;
        session::refresh_profile(store, slot);
        usage::login_replaced(store, slot);
        return Ok(Added { slot, new: false });
    }
    let slot = list.next_slot();
    store.set_credential(slot, credential)?;
    if let Err(e) = store.set_oauth_account(slot, oauth) {
        let _ = store.forget(slot);
        return Err(e.into());
    }
    list.put(Account {
        slot,
        alias: None,
        #[cfg(feature = "author-tools")]
        disabled: false,
        kind,
        account_uuid: who.account_uuid,
        organization_uuid: who.organization_uuid,
        identity,
        added: OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default(),
    });
    store.save(&list)?;
    Ok(Added { slot, new: true })
}

/// Drops the account's reading. The caller holds the app's lock.
fn forget_reading(store: &Store, slot: u32) {
    let mut cache = usage::load(store);
    cache.accounts.remove(&slot.to_string());
    let _ = usage::save(store, &cache);
}

/// Adds the login Claude Code is on now.
pub fn adopt_live(store: &Store) -> Result<Added, ManageError> {
    let _app = AppLock::take(&store.paths)?;
    let live = cc::read_live(&store.paths, &*store.security)?;
    let (Some(credential), Some(oauth)) = (live.credential, live.oauth) else {
        return Err(ManageError::NoLogin);
    };
    put(store, &credential, &oauth, Kind::Login)
}

/// Adds a token from `claude setup-token`. It names no account, so the
/// owner may give its email; without one it gets a placeholder.
pub fn add_token(store: &Store, token: &str, email: Option<&str>) -> Result<Added, ManageError> {
    let token = token.trim();
    if token.is_empty() || token.chars().any(char::is_whitespace) || !token.is_ascii() {
        return Err(ManageError::BadToken);
    }
    let _app = AppLock::take(&store.paths)?;
    let next = store.load()?.next_slot();
    let email = email
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(String::from)
        .unwrap_or_else(|| format!("setup-token-{next}@token.local"));
    let credential =
        json!({ "claudeAiOauth": { "accessToken": token, "scopes": ["user:inference"] } });
    let oauth = json!({
        "emailAddress": email,
        "accountUuid": "",
        "organizationUuid": null,
        "organizationName": null,
    });
    put(store, &credential.to_string(), &oauth, Kind::SetupToken)
}

/// The folder a new login is made in.
pub fn login_folder(store: &Store) -> PathBuf {
    store.paths.dir.join("adding")
}

/// Empties the login folder and its Keychain item.
fn clear_login(store: &Store, dir: &Path) {
    if let Some(service) = cc::profile_service(dir) {
        let _ = store.security.delete(&service, &user_account());
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Adds the login Claude Code made in `dir` (`CLAUDE_CONFIG_DIR`), then
/// removes it from there.
pub fn add_from_folder(store: &Store, dir: &Path) -> Result<Added, ManageError> {
    let _app = AppLock::take(&store.paths)?;
    let service = cc::profile_service(dir).ok_or(ManageError::NoLogin)?;
    let credential =
        cc::read_login(&*store.security, &service, dir)?.ok_or(ManageError::NoLogin)?;
    let oauth = super::read_optional(&dir.join(".claude.json"))?
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|c| c.get("oauthAccount").cloned())
        .ok_or(ManageError::NoLogin)?;
    let added = put(store, &credential, &oauth, Kind::Login);
    clear_login(store, dir);
    added
}

/// Logs a new account in with `claude auth login` in a folder of its own,
/// then adds it. `interactive` lets it use the terminal (the command line);
/// otherwise it gets no input and waits for the browser, until `cancel` is
/// set or [`LOGIN_TIMEOUT`].
pub fn login(
    store: &Store,
    claude: &Path,
    interactive: bool,
    cancel: &AtomicBool,
) -> Result<Added, ManageError> {
    let dir = login_folder(store);
    clear_login(store, &dir);
    super::private_dir(&dir)?;
    let mut command = Command::new(claude);
    command.args(["auth", "login"]).env("CLAUDE_CONFIG_DIR", &dir).current_dir(&dir);
    for name in session::SCRUBBED {
        command.env_remove(name);
    }
    if !interactive {
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    }
    let mut child = command.spawn()?;
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            clear_login(store, &dir);
            return Err(if cancel.load(Ordering::Relaxed) {
                ManageError::Cancelled
            } else {
                ManageError::Login("timed out".into())
            });
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    if !status.success() {
        let why = child
            .stderr
            .take()
            .and_then(|mut e| {
                let mut text = String::new();
                std::io::Read::read_to_string(&mut e, &mut text).ok().map(|_| text)
            })
            .map(|t| crate::runner::first_line(t.as_bytes()))
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| format!("exited {}", status.code().unwrap_or(-1)));
        clear_login(store, &dir);
        return Err(ManageError::Login(why));
    }
    add_from_folder(store, &dir)
}

/// Removes an account from the app: its Keychain items, its session folder
/// and its reading. Claude Code's login is never touched, so the account in
/// use cannot be removed.
pub fn remove(store: &Store, slot: u32) -> Result<(), ManageError> {
    let _app = AppLock::take(&store.paths)?;
    let mut list = store.load()?;
    list.get(slot).ok_or(ManageError::UnknownSlot)?;
    if switch::active_slot(store) == Some(slot) {
        return Err(ManageError::Active);
    }
    store.forget(slot)?;
    list.accounts.retain(|a| a.slot != slot);
    store.save(&list)?;
    let folder = session::folder(store, slot);
    if let Some(service) = cc::profile_service(&folder) {
        let _ = store.security.delete(&service, &user_account());
    }
    let _ = std::fs::remove_dir_all(&folder);
    forget_reading(store, slot);
    Ok(())
}

/// Sets or clears the account's alias.
pub fn set_alias(store: &Store, slot: u32, alias: &str) -> Result<(), ManageError> {
    let alias = store::clean_alias(alias).map_err(ManageError::BadAlias)?;
    change(store, slot, |a| a.alias = alias)
}

/// Puts the account in the rotation, or holds it out (it can still be
/// switched to by hand).
#[cfg(feature = "author-tools")]
pub fn set_enabled(store: &Store, slot: u32, enabled: bool) -> Result<(), ManageError> {
    change(store, slot, |a| a.disabled = !enabled)
}

fn change(store: &Store, slot: u32, f: impl FnOnce(&mut Account)) -> Result<(), ManageError> {
    let _app = AppLock::take(&store.paths)?;
    let mut list = store.load()?;
    f(list.get_mut(slot).ok_or(ManageError::UnknownSlot)?);
    store.save(&list)?;
    Ok(())
}
