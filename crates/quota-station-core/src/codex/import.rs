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

use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::accounts::{text, who_of, Account, Who};

/// The three tokens Codex needs to use a sign-in as it is.
const TOKENS: [&str; 3] = ["id_token", "access_token", "refresh_token"];

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
    let mut out = Vec::new();
    for value in serde_json::Deserializer::from_slice(bytes).into_iter::<Value>() {
        match value {
            Ok(Value::Array(items)) => out.extend(items.iter().map(one)),
            Ok(v) => out.push(one(&v)),
            // What follows cannot be told apart: one unreadable entry.
            Err(_) => {
                out.push(found(None, Kind::Broken));
                break;
            }
        }
    }
    if out.is_empty() {
        out.push(found(None, Kind::Broken));
    }
    out
}

fn found(format: Option<Format>, kind: Kind) -> Found {
    Found { format, kind, who: None, last_refresh: None, auth: Vec::new() }
}

/// One sign-in, from one JSON object.
fn one(v: &Value) -> Found {
    let Some(obj) = v.as_object() else { return found(None, Kind::Broken) };
    let key = text(obj.get("OPENAI_API_KEY"));
    let (format, tokens) = if let Some(t) = obj.get("tokens").and_then(Value::as_object) {
        (Format::Codex, t)
    } else if TOKENS.iter().any(|k| obj.contains_key(*k)) {
        // A CLIProxyAPI file of another provider (`claude`, `gemini`…).
        if obj.get("type").and_then(Value::as_str).is_some_and(|t| t != "codex") {
            return found(None, Kind::Broken);
        }
        (Format::CliProxy, obj)
    } else if key.is_some() {
        return found(Some(Format::Codex), Kind::ApiKey);
    } else {
        return found(None, Kind::Broken);
    };
    let has = |k: &str| text(tokens.get(k)).is_some();
    if !TOKENS.iter().all(|k| has(k)) {
        let kind = if has("refresh_token") {
            Kind::RefreshOnly
        } else if key.is_some() {
            Kind::ApiKey
        } else {
            Kind::Broken
        };
        return found(Some(format), kind);
    }
    let auth = match format {
        Format::Codex => v.clone(),
        Format::CliProxy => codex_shape(obj),
    };
    let who = who_of(&auth).map(|mut w| {
        // The id token's claims first; the file's own fields when it has none.
        w.email = w.email.or_else(|| text(obj.get("email")));
        w.plan = w.plan.or_else(|| text(obj.get("plan_type")));
        w
    });
    let Some(who) = who.filter(|w| w.identity.user.is_some() || w.identity.account.is_some())
    else {
        return found(Some(format), Kind::Broken);
    };
    let last_refresh = text(obj.get("last_refresh"))
        .and_then(|t| OffsetDateTime::parse(&t, &Rfc3339).ok())
        .map(OffsetDateTime::unix_timestamp);
    let auth = serde_json::to_vec_pretty(&auth).unwrap_or_default();
    Found { format: Some(format), kind: Kind::Login, who: Some(who), last_refresh, auth }
}

/// A CLIProxyAPI sign-in as Codex's `auth.json`: the tokens as they are,
/// and when they were last renewed. Its other fields stay behind.
fn codex_shape(flat: &Map<String, Value>) -> Value {
    let mut tokens = Map::new();
    for k in TOKENS.iter().chain(&["account_id"]) {
        if let Some(v) = flat.get(*k).filter(|v| v.is_string()) {
            tokens.insert((*k).to_string(), v.clone());
        }
    }
    let mut auth = Map::new();
    auth.insert("OPENAI_API_KEY".into(), Value::Null);
    auth.insert("tokens".into(), Value::Object(tokens));
    if let Some(at) = flat.get("last_refresh").filter(|v| v.is_string()) {
        auth.insert("last_refresh".into(), at.clone());
    }
    Value::Object(auth)
}

/// What importing each of `found` would do, given the accounts here and
/// which of them must sign in again.
pub fn plan(found: &[Found], accounts: &[Account], relogin: &dyn Fn(u32) -> bool) -> Vec<Fate> {
    fn login(f: &Found) -> Option<&Who> {
        f.who.as_ref().filter(|_| f.kind == Kind::Login)
    }
    found
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let Some(who) = login(f) else { return Fate::Left };
            let here = accounts
                .iter()
                .find(|a| a.entry.identity.same(&who.identity))
                .map(|a| a.entry.number);
            // The same account twice in one import: the newer sign-in, else
            // the first, is the one taken.
            let beaten = found.iter().enumerate().any(|(j, g)| {
                j != i
                    && login(g).is_some_and(|w| w.identity.same(&who.identity))
                    && (g.last_refresh > f.last_refresh
                        || (g.last_refresh == f.last_refresh && j < i))
            });
            match here {
                _ if beaten => Fate::Dup { number: here },
                Some(n) if relogin(n) => Fate::Renew { number: n },
                Some(n) => Fate::Dup { number: Some(n) },
                None => Fate::New,
            }
        })
        .collect()
}
