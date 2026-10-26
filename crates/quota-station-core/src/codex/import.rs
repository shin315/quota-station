// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Sign-ins brought in from a file: Codex's own `auth.json`, or the
//! `codex-*.json` CLIProxyAPI keeps (the same flat fields Cockpit exports as
//! "CPA"). A file may hold one sign-in, an array of them or one a line.
//!
//! Nothing here sends, renews or prints a token: a sign-in is read for who
//! it is, as [`super::accounts::who`] reads one, and kept in memory only
//! until [`super::accounts::Store::import`] writes it into an account's
//! folder, in the shape Codex writes. A file that only has a refresh token
//! is never taken: using it means trading it at once, which signs out the
//! place it came from.

use super::accounts::{Account, Who};

/// Which tool wrote the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Codex's `auth.json`: the tokens under `tokens`.
    Codex,
    /// CLIProxyAPI's `codex-*.json`: the tokens at the top, `type: "codex"`.
    CliProxy,
}

/// What a sign-in read from a file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A ChatGPT sign-in Codex can use as it is.
    Login,
    /// Only a refresh token (or no access or id token): not taken.
    RefreshOnly,
    /// An API key, which belongs in a custom provider.
    ApiKey,
    /// Not a sign-in the app can read, or one whose owner cannot be told.
    Broken,
}

/// One sign-in read from a file.
pub struct Found {
    pub format: Option<Format>,
    pub kind: Kind,
    pub who: Option<Who>,
    /// Unix seconds: when the tokens were last renewed, as the file says.
    pub last_refresh: Option<i64>,
    /// `auth.json` as Codex would have written it; empty unless a login.
    auth: Vec<u8>,
}

impl Found {
    /// The `auth.json` to write.
    pub(crate) fn auth(&self) -> &[u8] {
        &self.auth
    }
}

impl Drop for Found {
    fn drop(&mut self) {
        self.auth.iter_mut().for_each(|b| *b = 0);
    }
}

/// What importing one sign-in would do here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    /// Added as a new account.
    New,
    /// An account here that must sign in again takes this sign-in.
    Renew { number: u32 },
    /// Already here (`number`), or another file of this import is newer.
    Dup { number: Option<u32> },
    /// Not a sign-in to take (see [`Kind`]).
    Left,
}

/// The sign-ins in one file's bytes; one [`Kind::Broken`] when none parse.
pub fn read(bytes: &[u8]) -> Vec<Found> {
    let _ = bytes;
    todo!()
}

/// What importing each of `found` would do, given the accounts here and
/// which of them must sign in again.
pub fn plan(found: &[Found], accounts: &[Account], relogin: &dyn Fn(u32) -> bool) -> Vec<Fate> {
    let _ = (found, accounts, relogin);
    todo!()
}
