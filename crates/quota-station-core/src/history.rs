//! The switch history (F08), Claude and Antigravity, each line with its
//! reason and where it comes from:
//!
//! - the app's own journal, `history/history.jsonl` in its folder: a Claude switch
//!   the owner made here (with the hold it set), an account in use that
//!   changed between two readings with nothing here or in the switcher's log
//!   to explain it, and an Antigravity switch that Antigravity.app undid (the
//!   six-hour watch of M4c);
//! - the switcher's log: its switches and primes;
//! - the app's Antigravity switch log of M4c, `antigravity/switch-log.jsonl`;
//! - the app's Codex switch log of M8, `codex/switch-log.jsonl`: the owner's
//!   switches and the changes of provider made outside the app.
//!
//! Accounts are kept by slot number (Claude) and by the app's account id
//! (Antigravity, shown by its number in the app's list), Codex providers by
//! id; never an email.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::antigravity::fs::AppDir;
use crate::antigravity::switch::{self as agy_switch, How};
use crate::antigravity::{index, watch};
use crate::codex::log as codex_log;
use crate::switcher::{self, LogEvent, Why};

/// In the app's folder, `history/history.jsonl`.
pub const DIR: &str = "history";
pub const FILE: &str = "history.jsonl";
/// The journal keeps this many lines.
const KEEP: usize = 500;
/// How much of the end of the switcher's log is read.
const SWITCHER_TAIL: u64 = 64 * 1024;
/// A change of the account in use is explained by a switch to that account
/// this long before the reading that shows it: the switcher runs every
/// three minutes and the app reads every five while hidden.
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

/// Where a line comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// The owner, in this app.
    App,
    Switcher,
    /// Seen between two readings.
    Observed,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hold_until: Option<String>,
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
        hold_until: Option<String>,
        result: String,
    },
    /// `switch to slot N (…)` in the switcher's log.
    Switcher {
        why: Why,
        raw: String,
    },
    /// `primed slot N (exit X)`: a five-hour window started early.
    Primed {
        raw: String,
    },
    Observed,
    /// A switch from this app (M4c): `switched` or `unconfirmed`.
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

/// A Claude switch the owner made here.
pub fn manual_claude(
    app: &AppDir,
    from: Option<u32>,
    to: u32,
    hold_until: Option<String>,
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
            what: Kept::Manual { hold_until, result: result.to_string() },
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

/// The switcher's switches and primes in the end of its log, oldest first.
pub fn switcher_lines(log: &Path) -> Vec<switcher::LogLine> {
    let Ok(mut f) = std::fs::File::open(log) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    // One byte before the tail too, to tell whether it starts a line.
    let start = len.saturating_sub(SWITCHER_TAIL + 1);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    if f.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    // A tail that starts mid-line drops that line; one that starts a line keeps it.
    let tail = if start == 0 {
        &bytes[..]
    } else {
        match bytes.iter().position(|b| *b == b'\n') {
            Some(i) => &bytes[i + 1..],
            None => &[][..],
        }
    };
    let text = String::from_utf8_lossy(tail);
    text.lines()
        .map(switcher::parse_log_line)
        .filter(|l| {
            l.at.is_some() && matches!(l.event, LogEvent::Switch { .. } | LogEvent::Primed { .. })
        })
        .collect()
}

/// `switch to slot 4 (4.0%, ends 15:00)` → the switcher's reason, read the
/// way its dry-run decision is.
fn switch_why(raw: &str) -> Why {
    let text = raw.get(20..).unwrap_or("");
    let rest = text.strip_prefix("switch to slot ").unwrap_or("");
    match switcher::parse_decision(&format!("switch {rest}")) {
        switcher::Decision::Switch { why, .. } => why,
        _ => Why::Other,
    }
}

/// Every line, newest first, at most `limit`.
pub fn collect(app: &AppDir, switcher_log: &Path, limit: usize) -> Vec<Entry> {
    let accounts = index::load(app).unwrap_or_default().accounts;
    let number = |id: &str| accounts.iter().position(|a| a.id == id).map(|i| i as u32 + 1);
    let slot = |s: &str| s.parse::<u32>().ok();
    let mut out: Vec<(i64, Entry)> = Vec::new();

    for line in read(app) {
        let (from, to) = match line.app {
            App::Claude => (line.from.as_deref().and_then(slot), slot(&line.to)),
            App::Antigravity => (line.from.as_deref().and_then(number), number(&line.to)),
            // The Codex log keeps its own lines.
            App::Codex => continue,
        };
        if line.app == App::Claude && to.is_none() {
            continue;
        }
        let (source, cause) = match line.what {
            Kept::Manual { hold_until, result } => {
                (Source::App, Cause::Manual { hold_until, result })
            }
            Kept::Observed => (Source::Observed, Cause::Observed),
            Kept::Overwrite { overwrites } => {
                (Source::Observed, Cause::AgyOverwrite { overwrites })
            }
        };
        out.push((line.at, Entry { at: rfc(line.at), app: line.app, source, from, to, cause }));
    }

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
                from: line.from.as_deref().and_then(number),
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

    for line in switcher_lines(switcher_log) {
        let Some(at) = line.at.as_deref().and_then(unix) else { continue };
        let (to, cause) = match line.event {
            LogEvent::Switch { slot } => {
                (slot, Cause::Switcher { why: switch_why(&line.raw), raw: line.raw.clone() })
            }
            LogEvent::Primed { slot } => (slot, Cause::Primed { raw: line.raw.clone() }),
            _ => continue,
        };
        out.push((
            at,
            Entry {
                at: rfc(at),
                app: App::Claude,
                source: Source::Switcher,
                from: None,
                to: Some(to),
                cause,
            },
        ));
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
/// in use since `seen` that no switch explains. `claude` and `agy` are the
/// accounts the readings show in use, when they tell; the first reading
/// after the app starts only sets `seen`.
pub fn track(
    app: &AppDir,
    switcher_log: &Path,
    seen: &mut Seen,
    claude: Option<u32>,
    agy: Option<&str>,
    now: i64,
) {
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
        let recent = collect(app, switcher_log, usize::MAX);
        let accounts = index::load(app).unwrap_or_default().accounts;
        let number = |id: &str| accounts.iter().position(|a| a.id == id).map(|i| i as u32 + 1);
        // A switch to that account (by the owner, the switcher or the
        // overwrite just kept) shortly before: not a change of its own.
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
