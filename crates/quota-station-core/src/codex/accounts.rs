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
//! Every step that depends on the system goes through [`Platform`].

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

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

/// Who a sign-in is: ChatGPT's user and the workspace (a Plus and a Team
/// workspace of one person are two accounts, as Codex counts them).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub user: Option<String>,
    pub account: Option<String>,
}

impl Identity {
    /// The same account: every id known on both sides agrees, and one does.
    pub fn same(&self, other: &Identity) -> bool {
        todo!("{other:?}")
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
    Added { number: u32 },
    /// An account already in the list: the new sign-in was deleted, not
    /// signed out, so the one kept stays good.
    Dup { number: u32 },
    /// The account signed in again: its sign-in was replaced.
    Renewed { number: u32 },
    /// The folder holds no ChatGPT sign-in.
    NoLogin,
}

/// What a switch did to the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Moved {
    /// The account's sign-in is already the one in the Codex folder.
    Already,
    Moved { out: Option<u32>, into: u32 },
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
    Io { detail: String },
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

    /// `codex/accounts/<n>/`.
    pub fn folder(&self, number: u32) -> PathBuf {
        self.app.path().join(DIR).join(number.to_string())
    }

    pub fn registry(&self) -> Registry {
        todo!()
    }

    /// The accounts and where each sign-in is. Puts back a switch cut short,
    /// and adds the ChatGPT sign-in Codex has when it is not in the list.
    pub fn list(&self, now: OffsetDateTime) -> io::Result<Vec<Account>> {
        todo!("{now}")
    }

    /// A new, empty folder for `codex login` to sign in to.
    pub fn begin_login(&self) -> io::Result<PathBuf> {
        todo!()
    }

    /// After `codex login` in `pending`: a new account, one already listed,
    /// or `again` signed in again. `pending` is gone afterwards.
    pub fn finish_login(
        &self,
        pending: &Path,
        again: Option<u32>,
        now: OffsetDateTime,
    ) -> io::Result<Added> {
        todo!("{pending:?} {again:?} {now}")
    }

    /// A sign-in that did not finish: its folder goes.
    pub fn discard(&self, pending: &Path) {
        todo!("{pending:?}")
    }

    /// Makes `number` the account Codex uses. `store_is_file`: Codex keeps
    /// its sign-in in `auth.json` (not the Keychain).
    pub fn switch(&self, number: u32, store_is_file: bool, now: OffsetDateTime) -> Result<Moved, Refusal> {
        todo!("{number} {store_is_file} {now}")
    }

    /// Sets the nickname of `id` (`chatgpt:<n>` or a provider's table id);
    /// empty clears it.
    pub fn rename(&self, id: &str, nickname: &str) -> Result<(), Refusal> {
        todo!("{id} {nickname}")
    }

    /// The folder of an account that can be removed: not the one in the
    /// Codex folder. The caller may sign it out there first.
    pub fn removable(&self, number: u32) -> Result<PathBuf, Refusal> {
        todo!("{number}")
    }

    /// Deletes the account's folder and its entry.
    pub fn remove(&self, number: u32) -> Result<(), Refusal> {
        todo!("{number}")
    }
}

/// Who the sign-in in `auth` is, when it is a ChatGPT sign-in.
pub fn who(auth: &Path) -> Option<Who> {
    todo!("{auth:?}")
}
