// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Codex's session logs ("rollouts"): one JSON object per line in
//! `sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` in the Codex folder, and
//! in `archived_sessions/` once a session is archived (moved, so the index
//! knows it by its inode and does not read it twice).
//!
//! Each line is `{"timestamp", "type", "payload"}`. `session_meta` gives the
//! folder (`cwd`) and Codex's version (`cli_version`), `turn_context` the
//! folder and the model, and an `event_msg` whose payload is `token_count`
//! the session's running totals in `info.total_token_usage` (and the last
//! call's in `info.last_token_usage`): `input_tokens` with
//! `cached_input_tokens` inside it, and `output_tokens`. The tokens of a call
//! are the growth of the totals since the line before, so a count Codex
//! repeats adds nothing; when the totals start again (or a file's first
//! count), the last call's usage is taken.
//!
//! Codex before the rollout format above wrote a header line with `id` and
//! `timestamp` and then the items themselves, without any tokens: those
//! lines are known and quiet. A `token_count` whose `info` is not numbers,
//! a line with no `type` that is not such a header, and a line that is not
//! JSON are [`Line::Unknown`].

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{unix, Line, Tokens, Usage};

/// Under the Codex folder.
pub const SESSIONS: &str = "sessions";
pub const ARCHIVED: &str = "archived_sessions";

/// What a file has told so far, kept between reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The session's totals at the last count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<Count>,
}

/// Codex's own counts: `input` includes `cached`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Count {
    pub input: u64,
    pub cached: u64,
    pub output: u64,
}

impl Count {
    fn covers(&self, before: &Count) -> bool {
        self.input >= before.input && self.cached >= before.cached && self.output >= before.output
    }

    fn minus(&self, before: &Count) -> Count {
        Count {
            input: self.input - before.input,
            cached: self.cached - before.cached,
            output: self.output - before.output,
        }
    }

    fn tokens(&self) -> Tokens {
        Tokens {
            input: self.input.saturating_sub(self.cached),
            cache_write: 0,
            cache_read: self.cached.min(self.input),
            output: self.output,
        }
    }
}

/// The fields read from a line; serde skips every other one unread.
#[derive(Deserialize)]
struct Raw<'a> {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(borrow)]
    payload: Option<&'a RawValue>,
    /// The old header line.
    #[serde(borrow)]
    id: Option<&'a RawValue>,
    /// The old format's state lines.
    record_type: Option<String>,
}

#[derive(Deserialize)]
struct Meta {
    cwd: Option<String>,
    cli_version: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct Event<'a> {
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(borrow)]
    info: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Info {
    total_token_usage: Option<RawCount>,
    last_token_usage: Option<RawCount>,
}

#[derive(Deserialize)]
struct RawCount {
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
    output_tokens: u64,
}

impl From<RawCount> for Count {
    fn from(c: RawCount) -> Self {
        Count { input: c.input_tokens, cached: c.cached_input_tokens, output: c.output_tokens }
    }
}

impl File {
    /// One line, without its newline.
    pub fn line(&mut self, bytes: &[u8]) -> Line {
        let Ok(raw) = serde_json::from_slice::<Raw>(bytes) else { return Line::Unknown };
        let (Some(kind), Some(payload)) = (raw.kind.as_deref(), raw.payload) else {
            // The old format: a header, its items, its state lines.
            let old = (raw.id.is_some() && raw.timestamp.is_some())
                || raw.record_type.is_some()
                || raw.kind.is_some();
            return if old { Line::Quiet } else { Line::Unknown };
        };
        match kind {
            "session_meta" | "turn_context" => {
                if let Ok(meta) = serde_json::from_str::<Meta>(payload.get()) {
                    if meta.cwd.as_deref().is_some_and(|c| !c.is_empty()) {
                        self.cwd = meta.cwd;
                    }
                    if meta.cli_version.is_some() {
                        self.version = meta.cli_version;
                    }
                    if meta.model.as_deref().is_some_and(|m| !m.is_empty()) {
                        self.model = meta.model;
                    }
                }
                Line::Quiet
            }
            "event_msg" => self.event(raw.timestamp.as_deref(), payload),
            _ => Line::Quiet,
        }
    }

    fn event(&mut self, at: Option<&str>, payload: &RawValue) -> Line {
        let Ok(event) = serde_json::from_str::<Event>(payload.get()) else { return Line::Unknown };
        if event.kind.as_deref() != Some("token_count") {
            return Line::Quiet;
        }
        // A count with only the rate limits.
        let Some(info) = event.info.filter(|i| i.get() != "null") else { return Line::Quiet };
        let Ok(info) = serde_json::from_str::<Info>(info.get()) else { return Line::Unknown };
        let total = info.total_token_usage.map(Count::from);
        let last = info.last_token_usage.map(Count::from);
        let call = match (total, self.total) {
            (Some(now), Some(before)) if now.covers(&before) => Some(now.minus(&before)),
            (Some(now), _) => Some(last.unwrap_or(now)),
            (None, _) => last,
        };
        if total.is_some() {
            self.total = total;
        }
        let Some(call) = call else { return Line::Unknown };
        let tokens = call.tokens();
        if tokens.is_empty() {
            return Line::Quiet;
        }
        let Some(at) = at.and_then(unix) else { return Line::Unknown };
        Line::Usage(Usage {
            at,
            model: self.model.clone().unwrap_or_default(),
            project: self.cwd.clone(),
            tokens,
            key: None,
        })
    }
}
