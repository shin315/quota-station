// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switch history (F08), Claude and Antigravity, each line with its
//! reason and where it comes from:
//!
//! - the app's own journal, `history/history.jsonl` in its folder: a switch of the
//!   Claude account the owner made here, an account in use that changed between two
//!   readings with nothing here to explain it, and an Antigravity switch
//!   that Antigravity.app undid (the six-hour watch of M4c); the line of a
//!   switch the owner made keeps the keys an extension gives it
//!   ([`crate::switching::AroundSwitch::journal`]), and the History tab
//!   shows the extension's keys from them ([`Hooks::manual_line`]);
//! - the app's Antigravity switch log of M4c, `antigravity/switch-log.jsonl`;
//! - the app's Codex switch log of M8, `codex/switch-log.jsonl`: the owner's
//!   switches and the changes of provider made outside the app;
//! - an extension's own lines ([`Hooks::history_lines`]), each with the
//!   source and the reason the extension gives ([`ExtLine`]); they too
//!   explain a change of the account in use.
//!
//! Accounts are kept by slot number (Claude) and by the app's account id
//! (Antigravity, shown by its number in the app's list), Codex providers by
//! id; never an email.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, Serializer};
use serde_json::value::RawValue;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::switch::{self as agy_switch, How};
#[cfg(feature = "antigravity-accounts")]
use crate::antigravity::{index, watch};
use crate::app_dir::AppDir;
use crate::codex::log as codex_log;
use crate::ext::{Fields, Hooks};

/// In the app's folder, `history/history.jsonl`.
pub const DIR: &str = "history";
pub const FILE: &str = "history.jsonl";
/// The journal keeps this many lines.
const KEEP: usize = 500;
/// A change of the account in use is explained by a switch to that account
/// this long before the reading that shows it: the app reads every five
/// minutes while hidden.
pub const EXPLAINED_WITHIN: i64 = 15 * 60;
/// The most lines the windows get.
pub const SHOWN: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum App {
    Claude,
    Antigravity,
    Codex,
}

/// Where a line comes from: `app`, `observed`, or an extension's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The owner, in this app.
    App,
    /// Seen between two readings.
    Observed,
    /// An extension's line ([`ExtLine`]), by the name it gives its lines'
    /// source.
    Ext(&'static str),
}

/// A source is its name.
impl Serialize for Source {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Source::App => "app",
            Source::Observed => "observed",
            Source::Ext(name) => name,
        })
    }
}

/// One line of `history.jsonl`. `from` and `to` are a slot number for
/// Claude and the app's account id for Antigravity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stored {
    /// Unix seconds.
    pub at: i64,
    pub app: App,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: String,
    pub what: Kept,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Kept {
    /// The owner switched Claude here; `result` is what the check after it
    /// found (`switched`, `raced`, `unconfirmed`).
    Manual {
        /// The extension's keys for the switch, before `result`; read back,
        /// every key the line has that is not the app's.
        #[serde(flatten)]
        ext: Fields,
        result: String,
    },
    Observed,
    /// Antigravity.app wrote the old login back.
    Overwrite {
        overwrites: u32,
    },
}

/// One line as the History tab shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    /// RFC 3339.
    pub at: String,
    pub app: App,
    pub source: Source,
    /// Slot or account number before, when known.
    pub from: Option<u32>,
    /// Slot or account number after; `None` for an Antigravity account no
    /// longer logged in here.
    pub to: Option<u32>,
    pub cause: Cause,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Cause {
    Manual {
        /// The extension's keys ([`Hooks::manual_line`]), before `result`.
        #[serde(flatten)]
        ext: Fields,
        result: String,
    },
    Observed,
    /// A switch from this app (M4c): `switched` or `unconfirmed`.
    #[cfg(feature = "antigravity-accounts")]
    AgySwitch {
        how: How,
        result: String,
        reopened: Option<bool>,
    },
    AgyOverwrite {
        overwrites: u32,
    },
    /// A Codex switch from this app (M8), by provider id (`openai` is the
    /// ChatGPT login); `restarted` when the background process was
    /// restarted after it.
    CodexSwitch {
        from: String,
        to: String,
        restarted: Option<bool>,
    },
    /// `config.toml` chose another provider and the app did not write it.
    CodexObserved {
        from: String,
        to: String,
    },
    /// The reason an extension gives for one of its lines, written as it
    /// gives it (its own `kind` and keys).
    #[serde(untagged)]
    Ext(ExtCause),
}

/// The reason an extension gives for one of its lines ([`ExtLine`]): what
/// the windows read as the line's cause, an object with the extension's
/// own `kind` and keys, written as the extension wrote it, and whether the
/// line is a switch of the account in use to the line's `to`.
#[derive(Debug, Clone)]
pub struct ExtCause {
    json: Box<RawValue>,
    switch: bool,
}

impl ExtCause {
    /// `reason` as the windows read it (an object with a `kind`); `switch`
    /// when the line is a switch of the account in use to its `to`, which
    /// the stats' timeline of the accounts counts.
    pub fn new(reason: &impl Serialize, switch: bool) -> serde_json::Result<ExtCause> {
        Ok(ExtCause { json: serde_json::value::to_raw_value(reason)?, switch })
    }

    /// The reason's JSON text.
    pub fn json(&self) -> &str {
        self.json.get()
    }

    /// Whether the line is a switch of the account in use to its `to`.
    pub fn switch(&self) -> bool {
        self.switch
    }
}

impl PartialEq for ExtCause {
    fn eq(&self, other: &ExtCause) -> bool {
        self.json() == other.json() && self.switch == other.switch
    }
}

/// The reason as the extension wrote it.
impl Serialize for ExtCause {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.json.serialize(serializer)
    }
}

/// A line an extension adds to the history ([`Hooks::history_lines`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ExtLine {
    /// Unix seconds.
    pub at: i64,
    pub app: App,
    /// The name the line's source goes by ([`Source::Ext`]); not `app` or
    /// `observed`, which are the app's.
    pub source: &'static str,
    /// Slot or account number before, when known.
    pub from: Option<u32>,
    /// Slot or account number after, when known.
    pub to: Option<u32>,
    pub cause: ExtCause,
}

pub fn path(app: &AppDir) -> PathBuf {
    app.path().join(DIR).join(FILE)
}

/// The journal, oldest first; lines it cannot read are skipped.
pub fn read(app: &AppDir) -> Vec<Stored> {
    let text = app.read_to_string(&path(app)).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

/// Adds one line, keeping the newest [`KEEP`].
pub fn append(app: &AppDir, line: &Stored) -> std::io::Result<()> {
    let mut lines: Vec<String> = app
        .read_to_string(&path(app))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    lines.push(serde_json::to_string(line).map_err(std::io::Error::other)?);
    let skip = lines.len().saturating_sub(KEEP);
    let mut out = lines[skip..].join("\n");
    out.push('\n');
    app.write_private(&path(app), &out)
}

/// A switch of the Claude account the owner made here, with the keys the extension keeps
/// for it (`ext`, [`crate::switching::Done::journal`]).
pub fn manual_claude(
    app: &AppDir,
    from: Option<u32>,
    to: u32,
    ext: Fields,
    result: &str,
    at: i64,
) -> std::io::Result<()> {
    append(
        app,
        &Stored {
            at,
            app: App::Claude,
            from: from.map(|n| n.to_string()),
            to: to.to_string(),
            what: Kept::Manual { ext, result: result.to_string() },
        },
    )
}

fn rfc(at: i64) -> String {
    OffsetDateTime::from_unix_timestamp(at)
        .ok()
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_default()
}

fn unix(at: &str) -> Option<i64> {
    OffsetDateTime::parse(at, &Rfc3339).ok().map(|t| t.unix_timestamp())
}

/// An Antigravity account's number in the app's list, by its id. Without
/// the Antigravity accounts the app keeps no list, so no id has a number.
fn agy_numbers(app: &AppDir) -> impl Fn(&str) -> Option<u32> {
    #[cfg(feature = "antigravity-accounts")]
    let accounts = index::load(app).unwrap_or_default().accounts;
    #[cfg(not(feature = "antigravity-accounts"))]
    let _ = app;
    move |id: &str| {
        #[cfg(feature = "antigravity-accounts")]
        return accounts.iter().position(|a| a.id == id).map(|i| i as u32 + 1);
        #[cfg(not(feature = "antigravity-accounts"))]
        {
            let _ = id;
            None
        }
    }
}

/// Every line, newest first, at most `limit`: the app's, then the lines
/// the extension whose `hooks` are given adds, read under `home`; the
/// owner's switches with the extension's keys ([`Hooks::manual_line`]).
pub fn collect(app: &AppDir, home: &Path, hooks: &dyn Hooks, limit: usize) -> Vec<Entry> {
    let number = agy_numbers(app);
    let slot = |s: &str| s.parse::<u32>().ok();
    let mut out: Vec<(i64, Entry)> = Vec::new();

    for line in read(app) {
        let (from, to) = match line.app {
            App::Claude => (line.from.as_deref().and_then(slot), slot(&line.to)),
            App::Antigravity => (line.from.as_deref().and_then(&number), number(&line.to)),
            // The Codex log keeps its own lines.
            App::Codex => continue,
        };
        if line.app == App::Claude && to.is_none() {
            continue;
        }
        let (source, cause) = match line.what {
            Kept::Manual { ext, result } => {
                (Source::App, Cause::Manual { ext: hooks.manual_line(&ext), result })
            }
            Kept::Observed => (Source::Observed, Cause::Observed),
            Kept::Overwrite { overwrites } => {
                (Source::Observed, Cause::AgyOverwrite { overwrites })
            }
        };
        out.push((line.at, Entry { at: rfc(line.at), app: line.app, source, from, to, cause }));
    }

    #[cfg(feature = "antigravity-accounts")]
    for line in agy_switch::read_log(app) {
        if line.result != "switched" && line.result != "unconfirmed" {
            continue;
        }
        out.push((
            line.at,
            Entry {
                at: rfc(line.at),
                app: App::Antigravity,
                source: Source::App,
                from: line.from.as_deref().and_then(&number),
                to: number(&line.to),
                cause: Cause::AgySwitch {
                    how: line.how,
                    result: line.result,
                    reopened: line.reopened,
                },
            },
        ));
    }

    for line in codex_log::read(app) {
        let cause = match line.result.as_str() {
            "switched" => {
                Cause::CodexSwitch { from: line.from, to: line.to, restarted: line.restarted }
            }
            "observed" => Cause::CodexObserved { from: line.from, to: line.to },
            _ => continue,
        };
        let source = if matches!(cause, Cause::CodexObserved { .. }) {
            Source::Observed
        } else {
            Source::App
        };
        out.push((
            line.at,
            Entry { at: rfc(line.at), app: App::Codex, source, from: None, to: None, cause },
        ));
    }

    for line in hooks.history_lines(home, app) {
        let entry = Entry {
            at: rfc(line.at),
            app: line.app,
            source: Source::Ext(line.source),
            from: line.from,
            to: line.to,
            cause: Cause::Ext(line.cause),
        };
        out.push((line.at, entry));
    }

    // Newest first; at the same second the app's own line goes first.
    out.sort_by_key(|e| std::cmp::Reverse(e.0));
    out.into_iter().take(limit).map(|(_, e)| e).collect()
}

/// What the readings showed in use last, to compare the next ones with.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub claude: Option<u32>,
    /// The app's account id.
    pub agy: Option<String>,
}

/// Keeps what the newest readings say happened: an overwrite the six-hour
/// watch found that is not in the journal yet, and a change of the account
/// in use since `seen` that no switch explains (the extension's lines,
/// read under `home` with its `hooks`, explain one too). `claude` and `agy`
/// are the accounts the readings show in use, when they tell; the first
/// reading after the app starts only sets `seen`.
pub fn track(
    app: &AppDir,
    home: &Path,
    hooks: &dyn Hooks,
    seen: &mut Seen,
    claude: Option<u32>,
    agy: Option<&str>,
    now: i64,
) {
    #[cfg(feature = "antigravity-accounts")]
    if let Some(w) = watch::load(app) {
        if let Some(at) = w.alert() {
            let kept = read(app).iter().any(|l| {
                l.app == App::Antigravity && l.at == at && matches!(l.what, Kept::Overwrite { .. })
            });
            if !kept {
                let _ = append(
                    app,
                    &Stored {
                        at,
                        app: App::Antigravity,
                        from: Some(w.to.clone()),
                        to: w.from.clone(),
                        what: Kept::Overwrite { overwrites: w.overwrites },
                    },
                );
            }
        }
    }

    let claude_change = match (seen.claude, claude) {
        (Some(before), Some(after)) if before != after => Some((before, after)),
        _ => None,
    };
    let agy_change = match (seen.agy.as_deref(), agy) {
        (Some(before), Some(after)) if before != after => {
            Some((before.to_string(), after.to_string()))
        }
        _ => None,
    };
    if claude_change.is_some() || agy_change.is_some() {
        let recent = collect(app, home, hooks, usize::MAX);
        let number = agy_numbers(app);
        // A switch to that account (by the owner, the overwrite just kept,
        // or an extension's line) shortly before: not a change of its own.
        let explained = |which: App, to: Option<u32>| {
            to.is_some()
                && recent.iter().any(|e| {
                    e.app == which
                        && e.to == to
                        && (e.source != Source::Observed
                            || matches!(e.cause, Cause::AgyOverwrite { .. }))
                        && unix(&e.at).is_some_and(|t| t <= now + 60 && now - t <= EXPLAINED_WITHIN)
                })
        };
        if let Some((before, after)) = claude_change {
            if !explained(App::Claude, Some(after)) {
                let _ = append(
                    app,
                    &Stored {
                        at: now,
                        app: App::Claude,
                        from: Some(before.to_string()),
                        to: after.to_string(),
                        what: Kept::Observed,
                    },
                );
            }
        }
        if let Some((before, after)) = agy_change {
            if !explained(App::Antigravity, number(&after)) {
                let _ = append(
                    app,
                    &Stored {
                        at: now,
                        app: App::Antigravity,
                        from: Some(before),
                        to: after,
                        what: Kept::Observed,
                    },
                );
            }
        }
    }
    if claude.is_some() {
        seen.claude = claude;
    }
    if let Some(id) = agy {
        seen.agy = Some(id.to_string());
    }
}
