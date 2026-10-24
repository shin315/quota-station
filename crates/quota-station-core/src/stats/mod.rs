// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Thống kê (M16, J): the tokens Claude Code and Codex used on this Mac,
//! counted from their own session logs.
//!
//! Only four things are read from a log line: the token counts, the model,
//! the time and the project folder. A line is deserialized into a struct
//! that names those fields and nothing else, so serde skips the
//! conversation, the prompts and the tool output without ever copying
//! them, and the line's bytes are zeroed once it has been read. Nothing
//! that came from the conversation is kept, logged, indexed, exported or
//! shown, and nothing is sent anywhere: the logs read are this Mac's own.
//!
//! - [`claude_log`] reads `~/.claude/projects/**/*.jsonl`.
//! - [`codex_log`] reads `sessions/**/rollout-*.jsonl` and
//!   `archived_sessions/` in the Codex folder.
//! - [`index`] keeps, in the app's folder, how far each log was read and
//!   the counts per quarter hour, model and project; later reads take only
//!   what was appended, and a truncated, replaced or deleted log is handled
//!   without counting a line twice.
//! - [`report`] adds the counts up for Hôm nay, 7 ngày or 30 ngày, with an
//!   approximate split by Claude account from the switch history.
//! - [`prices`] is the one table of API prices, for "Giá API tương đương".
//!
//! A line or a file in a format the app does not know is skipped and
//! counted, never guessed. Antigravity keeps no token log on the Mac that
//! the app can read, so it has no row of numbers.

pub mod claude_log;
pub mod codex_log;
pub mod index;
pub mod prices;
pub mod report;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub use crate::agents::Agent;

/// The tokens of one or more API calls, by how they are priced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    /// Input not read from a cache.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub input: u64,
    /// Input written to a cache (Claude only).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cache_write: u64,
    /// Input read from a cache.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cache_read: u64,
    /// Output, reasoning included.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub output: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl Tokens {
    pub fn total(&self) -> u64 {
        self.input + self.cache_write + self.cache_read + self.output
    }

    pub fn add(&mut self, other: &Tokens) {
        self.input += other.input;
        self.cache_write += other.cache_write;
        self.cache_read += other.cache_read;
        self.output += other.output;
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// One API call's usage, as a log line gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// Unix seconds.
    pub at: i64,
    pub model: String,
    /// The folder the session ran in, when the log says.
    pub project: Option<String>,
    pub tokens: Tokens,
    /// Claude Code writes one API response over several lines, and a
    /// resumed session copies earlier ones: the same key is counted once.
    pub key: Option<u64>,
}

/// What a log line held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Usage(Usage),
    /// A line of a known kind without tokens: a prompt, a tool call, a
    /// session's header.
    Quiet,
    /// Not a format the app knows: skipped and counted.
    Unknown,
}

/// RFC 3339 to Unix seconds.
pub(crate) fn unix(text: &str) -> Option<i64> {
    OffsetDateTime::parse(text, &Rfc3339).ok().map(|t| t.unix_timestamp())
}

/// A key that cannot be turned back into what it was made from.
pub(crate) fn key(parts: &[&str]) -> u64 {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update([0]);
    }
    let d = h.finalize();
    u64::from_le_bytes(d[..8].try_into().expect("eight bytes"))
}
