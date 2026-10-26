// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Several ChatGPT accounts for Codex. Codex keeps one sign-in per Codex
//! folder (`auth.json`), so each account the app keeps has a folder of its
//! own, `codex/accounts/<n>/` in the app's folder, and Codex itself writes
//! the sign-in there (`codex login` run with that folder as `CODEX_HOME`).
//! The account Codex uses is the one whose `auth.json` sits in the Codex
//! folder; a switch moves two files: the one there back to its account's
//! folder, the chosen one in. Nothing is ever copied, so no sign-in lives in
//! two places, where one of the two would be logged out at its next renewal.
//!
//! A sign-in is read only for who it is (the ids, email and plan in its
//! `id_token`'s claims), then dropped; the app never refreshes, writes or
//! sends a token from here. `codex/accounts.json` keeps the list: numbers,
//! those ids, nicknames, and the move under way, so a switch cut short is
//! put back when the app next looks.
//!
//! Every step that depends on the system goes through [`Platform`], and
//! every step that moves or reads the account folders holds one lock, so a
//! quota read never meets a switch half done.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::platform::Platform;
use crate::app_dir::AppDir;

/// The account folders, in the app's folder.
pub const DIR: &str = "codex/accounts";
/// The list, in the app's folder.
pub const REGISTRY: &str = "codex/accounts.json";
const VERSION: u32 = 1;
/// A sign-in under way lives in `pending-<uuid>` until it is known.
const PENDING: &str = "pending-";
/// The `config.toml` of an account's folder: Codex there keeps the sign-in
/// in `auth.json`, whatever the Mac's own config says.
pub const FILE_STORE: &str = "cli_auth_credentials_store = \"file\"\n";

/// One look at, or move in, the account folders at a time, across the
/// app's threads.
static GUARD: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

/// Who a sign-in is: ChatGPT's user and the workspace (a Plus and a Team
/// workspace of one person are two accounts, as Codex counts them).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub user: Option<String>,
    pub account: Option<String>,
}

impl Identity {
    /// The same account: some id is known on both sides, and every id
    /// known on both sides agrees.
    pub fn same(&self, other: &Identity) -> bool {
        let pairs = [(&self.user, &other.user), (&self.account, &other.account)];
        let known: Vec<_> =
            pairs.iter().filter_map(|(a, b)| Some((a.as_ref()?, b.as_ref()?))).collect();
        !known.is_empty() && known.iter().all(|(a, b)| a == b)
    }
}

/// What a sign-in says about itself, read from `auth.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Who {
    pub identity: Identity,
    pub email: Option<String>,
    /// `plus`, `pro`, `team`… as the token's claims name it.
    pub plan: Option<String>,
}

/// One ChatGPT account the app keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// Its number: `chatgpt:<n>` to the windows, `codex/accounts/<n>/`.
    pub number: u32,
    pub identity: Identity,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    /// Unix seconds; `None` for the sign-in Codex already had.
    #[serde(default)]
    pub added_at: Option<i64>,
}

/// A switch under way: `out` goes to its folder, `into` comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Moving {
    pub out: Option<u32>,
    pub into: u32,
}

/// `codex/accounts.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    #[serde(default)]
    pub version: u32,
    /// The next number to give.
    #[serde(default)]
    pub next: u32,
    #[serde(default)]
    pub accounts: Vec<Entry>,
    /// The owner's nicknames of the custom providers, by table id.
    #[serde(default)]
    pub nicknames: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moving: Option<Moving>,
}

/// Where an account's sign-in is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    /// In the Codex folder: the one Codex uses.
    Home,
    /// In its own folder.
    Folder,
    /// Nowhere: it has to sign in again.
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub entry: Entry,
    pub place: Place,
}

/// What a sign-in ended as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Added {
    Added {
        number: u32,
    },
    /// An account already in the list: the new sign-in was deleted, not
    /// signed out, so the one kept stays good.
    Dup {
        number: u32,
    },
    /// The account signed in again: its sign-in was replaced.
    Renewed {
        number: u32,
    },
    /// The folder holds no ChatGPT sign-in.
    NoLogin,
}

/// What a switch did to the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Moved {
    /// The account's sign-in is already the one in the Codex folder.
    Already,
    Moved {
        out: Option<u32>,
        into: u32,
    },
}

/// Why a switch or a removal cannot run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Refusal {
    /// No account by that number.
    Unknown,
    /// Its sign-in is nowhere: it signs in again first.
    NoLogin,
    /// The account's sign-in is the one in the Codex folder.
    InUse,
    /// Something is where a sign-in would go: nothing was moved.
    Conflict,
    /// Codex keeps its sign-in in the Keychain: there is no file to move.
    Keychain,
    /// The nickname is too long or has a line break.
    Name,
    Io {
        detail: String,
    },
}

/// The Codex accounts of one Codex folder.
pub struct Store<'a> {
    app: &'a AppDir,
    home: &'a Path,
    platform: &'a dyn Platform,
}

impl<'a> Store<'a> {
    pub fn new(app: &'a AppDir, codex_home: &'a Path, platform: &'a dyn Platform) -> Self {
        Store { app, home: codex_home, platform }
    }

    fn root(&self) -> PathBuf {
        self.app.path().join(DIR)
    }

    /// `codex/accounts/<n>/`.
    pub fn folder(&self, number: u32) -> PathBuf {
        self.root().join(number.to_string())
    }

    fn home_auth(&self) -> PathBuf {
        self.home.join(super::AUTH)
    }

    fn registry_path(&self) -> PathBuf {
        self.app.path().join(REGISTRY)
    }

    pub fn registry(&self) -> Registry {
        self.app
            .read_to_string(&self.registry_path())
            .ok()
            .and_then(|t| serde_json::from_str::<Registry>(&t).ok())
            .unwrap_or_default()
    }

    fn save(&self, registry: &Registry) -> io::Result<()> {
        let mut r = registry.clone();
        r.version = VERSION;
        let text = serde_json::to_string_pretty(&r).map_err(io::Error::other)?;
        self.app.write_private(&self.registry_path(), &(text + "\n"))
    }

    /// The account folders' parent, made private and kept out of backups
    /// the first time.
    fn ensure_root(&self) -> io::Result<PathBuf> {
        let root = self.root();
        self.app.inside(&root)?;
        let fresh = !root.is_dir();
        self.platform.private_dir(&root)?;
        if fresh {
            self.platform.exclude_from_backup(&root);
        }
        Ok(root)
    }

    /// An account's own folder, made private.
    fn ensure_folder(&self, number: u32) -> io::Result<PathBuf> {
        self.ensure_root()?;
        let dir = self.folder(number);
        self.app.inside(&dir)?;
        self.platform.private_dir(&dir)?;
        Ok(dir)
    }

    /// Adds `who` with the next number.
    fn adopt(&self, registry: &mut Registry, who: &Who, added_at: Option<i64>) -> u32 {
        let number = registry
            .next
            .max(registry.accounts.iter().map(|e| e.number + 1).max().unwrap_or(1))
            .max(1);
        registry.next = number + 1;
        registry.accounts.push(Entry {
            number,
            identity: who.identity.clone(),
            email: who.email.clone(),
            plan: who.plan.clone(),
            nickname: None,
            added_at,
        });
        number
    }

    /// The listed account `who` is, with its email and plan brought up to date.
    fn known(registry: &mut Registry, who: &Who) -> Option<u32> {
        let entry = registry.accounts.iter_mut().find(|e| e.identity.same(&who.identity))?;
        if who.email.is_some() {
            entry.email = who.email.clone();
        }
        if who.plan.is_some() {
            entry.plan = who.plan.clone();
        }
        Some(entry.number)
    }

    /// A switch cut short between its two moves goes back to before it.
    fn recover(&self, registry: &mut Registry) -> io::Result<()> {
        let Some(m) = registry.moving.take() else { return Ok(()) };
        let home = self.home_auth();
        let into = self.folder(m.into).join(super::AUTH);
        if !home.exists() && into.exists() {
            if let Some(out) = m.out {
                let kept = self.folder(out).join(super::AUTH);
                if kept.exists() {
                    self.platform.move_file(&kept, &home)?;
                }
            }
        }
        self.save(registry)
    }

    /// The registry with a switch cut short put back and Codex's own
    /// sign-in listed; the number of the account in the Codex folder.
    fn settle(&self, now: OffsetDateTime) -> io::Result<(Registry, Option<u32>)> {
        let _ = now;
        let mut registry = self.registry();
        let before = registry.clone();
        self.recover(&mut registry)?;
        let home = match who(&self.home_auth()) {
            Some(w) => match Self::known(&mut registry, &w) {
                Some(n) => Some(n),
                None => {
                    let n = self.adopt(&mut registry, &w, None);
                    self.ensure_folder(n)?;
                    Some(n)
                }
            },
            None => None,
        };
        if registry != before {
            self.save(&registry)?;
        }
        Ok((registry, home))
    }

    /// The accounts and where each sign-in is. Puts back a switch cut short,
    /// and adds the ChatGPT sign-in Codex has when it is not in the list.
    pub fn list(&self, now: OffsetDateTime) -> io::Result<Vec<Account>> {
        let _held = guard();
        self.listed(now)
    }

    /// [`Self::list`], then `look` with nothing moved meanwhile: the quota
    /// read takes each account's sign-in into memory here.
    pub fn with_list<T>(
        &self,
        now: OffsetDateTime,
        look: impl FnOnce(&[Account]) -> T,
    ) -> io::Result<T> {
        let _held = guard();
        Ok(look(&self.listed(now)?))
    }

    fn listed(&self, now: OffsetDateTime) -> io::Result<Vec<Account>> {
        let (registry, home) = self.settle(now)?;
        Ok(registry
            .accounts
            .into_iter()
            .map(|entry| {
                let place = if Some(entry.number) == home {
                    Place::Home
                } else if self.folder(entry.number).join(super::AUTH).is_file() {
                    Place::Folder
                } else {
                    Place::Missing
                };
                Account { entry, place }
            })
            .collect())
    }

    /// A new, empty folder for `codex login` to sign in to.
    pub fn begin_login(&self) -> io::Result<PathBuf> {
        let _held = guard();
        self.pending_folder()
    }

    fn pending_folder(&self) -> io::Result<PathBuf> {
        let root = self.ensure_root()?;
        let dir = root.join(format!("{PENDING}{}", uuid::Uuid::new_v4().simple()));
        self.app.inside(&dir)?;
        self.platform.private_dir(&dir)?;
        Ok(dir)
    }

    /// `pending` is one of [`Self::begin_login`]'s folders.
    fn check_pending(&self, pending: &Path) -> io::Result<()> {
        self.app.inside(pending)?;
        let ok = pending.parent() == Some(self.root().as_path())
            && pending.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(PENDING));
        if ok {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "not a sign-in folder"))
        }
    }

    /// After `codex login` in `pending`: a new account, one already listed,
    /// or `again` signed in again. `pending` is gone afterwards.
    pub fn finish_login(
        &self,
        pending: &Path,
        again: Option<u32>,
        now: OffsetDateTime,
    ) -> io::Result<Added> {
        let _held = guard();
        self.finish(pending, again, now)
    }

    fn finish(&self, pending: &Path, again: Option<u32>, now: OffsetDateTime) -> io::Result<Added> {
        self.check_pending(pending)?;
        let file = pending.join(super::AUTH);
        let Some(w) = who(&file) else {
            self.drop_pending(pending);
            return Ok(Added::NoLogin);
        };
        let (mut registry, home) = self.settle(now)?;
        let known = Self::known(&mut registry, &w);
        match (known, again) {
            (Some(n), Some(a)) if n == a => {
                let to = if home == Some(n) {
                    self.home_auth()
                } else {
                    self.ensure_folder(n)?.join(super::AUTH)
                };
                self.platform.private_file(&file)?;
                self.platform.replace_file(&file, &to)?;
                self.drop_pending(pending);
                self.save(&registry)?;
                Ok(Added::Renewed { number: n })
            }
            (Some(n), _) => {
                self.drop_pending(pending);
                self.save(&registry)?;
                Ok(Added::Dup { number: n })
            }
            (None, _) => {
                let n = self.adopt(&mut registry, &w, Some(now.unix_timestamp()));
                let dir = self.folder(n);
                self.app.inside(&dir)?;
                if std::fs::symlink_metadata(&dir).is_ok() {
                    // A folder left by an account removed by hand: not reused.
                    std::fs::remove_dir_all(&dir)?;
                }
                std::fs::rename(pending, &dir)?;
                self.platform.private_dir(&dir)?;
                self.platform.private_file(&dir.join(super::AUTH))?;
                self.save(&registry)?;
                Ok(Added::Added { number: n })
            }
        }
    }

    /// A sign-in read from a file ([`super::import`]), as if `codex login`
    /// had written it: a new account, one already listed (nothing written),
    /// or `again` taking it in place of its own.
    pub fn import(
        &self,
        found: &super::import::Found,
        again: Option<u32>,
        now: OffsetDateTime,
    ) -> io::Result<Added> {
        if found.kind != super::import::Kind::Login || found.who.is_none() {
            return Ok(Added::NoLogin);
        }
        let _held = guard();
        let pending = self.pending_folder()?;
        let written = (|| {
            let file = pending.join(super::AUTH);
            std::fs::write(&file, found.auth())?;
            self.platform.private_file(&file)?;
            std::fs::write(pending.join(super::CONFIG), FILE_STORE)
        })();
        if let Err(e) = written {
            self.drop_pending(&pending);
            return Err(e);
        }
        self.finish(&pending, again, now)
    }

    /// A sign-in that did not finish: its folder goes.
    pub fn discard(&self, pending: &Path) {
        let _held = guard();
        self.drop_pending(pending);
    }

    fn drop_pending(&self, pending: &Path) {
        if self.check_pending(pending).is_ok() {
            let _ = std::fs::remove_dir_all(pending);
        }
    }

    /// Makes `number` the account Codex uses. `store_is_file`: Codex keeps
    /// its sign-in in `auth.json` (not the Keychain).
    pub fn switch(
        &self,
        number: u32,
        store_is_file: bool,
        now: OffsetDateTime,
    ) -> Result<Moved, Refusal> {
        if !store_is_file {
            return Err(Refusal::Keychain);
        }
        let _held = guard();
        let io = |e: io::Error| Refusal::Io { detail: e.to_string() };
        let (mut registry, home) = self.settle(now).map_err(io)?;
        if !registry.accounts.iter().any(|e| e.number == number) {
            return Err(Refusal::Unknown);
        }
        if home == Some(number) {
            return Ok(Moved::Already);
        }
        let incoming = self.folder(number).join(super::AUTH);
        if !incoming.is_file() {
            return Err(Refusal::NoLogin);
        }
        let home_auth = self.home_auth();
        let out = match home {
            Some(n) => {
                let to = self.ensure_folder(n).map_err(io)?.join(super::AUTH);
                if std::fs::symlink_metadata(&to).is_ok() {
                    return Err(Refusal::Conflict);
                }
                Some((n, to))
            }
            // Something Codex reads that is not a ChatGPT sign-in (an API
            // key): it is not the app's to move.
            None if std::fs::symlink_metadata(&home_auth).is_ok() => return Err(Refusal::Conflict),
            None => None,
        };
        registry.moving = Some(Moving { out: out.as_ref().map(|(n, _)| *n), into: number });
        self.save(&registry).map_err(io)?;
        let done = (|| {
            if let Some((_, to)) = &out {
                self.platform.move_file(&home_auth, to)?;
                self.platform.private_file(to)?;
            }
            if let Err(e) = self.platform.move_file(&incoming, &home_auth) {
                if let Some((_, to)) = &out {
                    let _ = self.platform.move_file(to, &home_auth);
                }
                return Err(e);
            }
            self.platform.private_file(&home_auth)
        })();
        registry.moving = None;
        let saved = self.save(&registry);
        done.map_err(io)?;
        saved.map_err(io)?;
        Ok(Moved::Moved { out: out.map(|(n, _)| n), into: number })
    }

    /// Sets the nickname of `id` (`chatgpt:<n>` or a provider's table id);
    /// empty clears it.
    pub fn rename(&self, id: &str, nickname: &str) -> Result<(), Refusal> {
        let name = nickname.trim();
        if name.chars().count() > NICKNAME_MAX || name.chars().any(char::is_control) {
            return Err(Refusal::Name);
        }
        let name = (!name.is_empty()).then(|| name.to_string());
        let _held = guard();
        let mut registry = self.registry();
        match id.strip_prefix(CHATGPT_ID).map(str::parse::<u32>) {
            Some(Ok(n)) => {
                let entry =
                    registry.accounts.iter_mut().find(|e| e.number == n).ok_or(Refusal::Unknown)?;
                entry.nickname = name;
            }
            Some(Err(_)) => return Err(Refusal::Unknown),
            None => match name {
                Some(n) => {
                    registry.nicknames.insert(id.to_string(), n);
                }
                None => {
                    registry.nicknames.remove(id);
                }
            },
        }
        self.save(&registry).map_err(|e| Refusal::Io { detail: e.to_string() })
    }

    /// The folder of an account that can be removed: not the one in the
    /// Codex folder. The caller may sign it out there first.
    pub fn removable(&self, number: u32) -> Result<PathBuf, Refusal> {
        let _held = guard();
        self.removable_now(number)
    }

    fn removable_now(&self, number: u32) -> Result<PathBuf, Refusal> {
        let (registry, home) = self
            .settle(OffsetDateTime::now_utc())
            .map_err(|e| Refusal::Io { detail: e.to_string() })?;
        if !registry.accounts.iter().any(|e| e.number == number) {
            return Err(Refusal::Unknown);
        }
        if home == Some(number) {
            return Err(Refusal::InUse);
        }
        Ok(self.folder(number))
    }

    /// Deletes the account's folder and its entry.
    pub fn remove(&self, number: u32) -> Result<(), Refusal> {
        let _held = guard();
        let dir = self.removable_now(number)?;
        let io = |e: io::Error| Refusal::Io { detail: e.to_string() };
        self.app.inside(&dir).map_err(io)?;
        match std::fs::remove_dir_all(&dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io(e)),
            _ => {}
        }
        let mut registry = self.registry();
        registry.accounts.retain(|e| e.number != number);
        self.save(&registry).map_err(io)
    }
}

/// The windows' id of a ChatGPT account: `chatgpt:<n>`.
pub const CHATGPT_ID: &str = "chatgpt:";
/// The longest nickname kept.
const NICKNAME_MAX: usize = 40;
/// `auth.json` is a few kilobytes; anything far larger is not Codex's.
const MAX_AUTH: u64 = 1024 * 1024;

pub(crate) fn text(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn claims(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let part = token.split('.').nth(1)?;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(part.trim_end_matches('=')).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Who the sign-in in `auth` is, when it is a ChatGPT sign-in. The file is
/// read into memory, its `id_token`'s claims looked at, and the bytes zeroed.
pub fn who(auth: &Path) -> Option<Who> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(auth).ok()?.take(MAX_AUTH).read_to_end(&mut bytes).ok()?;
    let found = serde_json::from_slice::<serde_json::Value>(&bytes).ok().as_ref().and_then(who_of);
    // The values go no further than this function.
    bytes.iter_mut().for_each(|b| *b = 0);
    found
}

/// Who the sign-in in `auth.json`'s data is, when it is a ChatGPT sign-in.
pub(crate) fn who_of(data: &serde_json::Value) -> Option<Who> {
    let tokens = data.get("tokens").filter(|t| t.is_object())?;
    let has = |k: &str| text(tokens.get(k)).is_some();
    if !(has("id_token") || has("access_token") || has("refresh_token")) {
        return None;
    }
    let claims = text(tokens.get("id_token")).as_deref().and_then(claims).unwrap_or_default();
    let auth = claims.get("https://api.openai.com/auth");
    let from_auth = |k: &str| text(auth.and_then(|a| a.get(k)));
    Some(Who {
        identity: Identity {
            user: from_auth("chatgpt_user_id")
                .or_else(|| from_auth("user_id"))
                .or_else(|| text(claims.get("sub"))),
            account: text(tokens.get("account_id")).or_else(|| from_auth("chatgpt_account_id")),
        },
        email: text(claims.get("email")),
        plan: from_auth("chatgpt_plan_type"),
    })
}
