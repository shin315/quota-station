// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Claude Code's session logs: one JSON object per line in
//! `~/.claude/projects/<folder>/<session>.jsonl` (and the subagents' logs
//! under a session's folder).
//!
//! A line of `"type": "assistant"` is one API response; its
//! `message.usage` has `input_tokens`, `output_tokens`,
//! `cache_creation_input_tokens` and `cache_read_input_tokens`, its
//! `message.model` the model, and the line's `timestamp` and `cwd` the time
//! and the project. Claude Code writes a response over several lines (one
//! per content block, each with the same usage) and copies earlier ones into
//! a resumed session, so `message.id` with `requestId` makes the key that
//! counts it once. Every other `type` (the prompts, tool results, summaries,
//! snapshots) carries no tokens.
//!
//! An assistant line whose usage is missing or not numbers, a line with no
//! `type`, and a line that is not JSON are [`Line::Unknown`].

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{key, unix, Line, Tokens, Usage};

/// `~/.claude/projects`, under the user's home.
pub const DIR: &str = ".claude/projects";

/// What a file has told so far, kept between reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    /// Claude Code's version that wrote the newest line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The fields read from a line; serde skips every other one unread.
#[derive(Deserialize)]
struct Raw<'a> {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    version: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    #[serde(borrow)]
    message: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Message<'a> {
    id: Option<String>,
    model: Option<String>,
    #[serde(borrow)]
    usage: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct RawUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

impl File {
    /// One line, without its newline.
    pub fn line(&mut self, bytes: &[u8]) -> Line {
        let Ok(raw) = serde_json::from_slice::<Raw>(bytes) else { return Line::Unknown };
        if let Some(v) = raw.version {
            self.version = Some(v);
        }
        match raw.kind.as_deref() {
            None => Line::Unknown,
            Some("assistant") => assistant(&raw.timestamp, raw.cwd, raw.request_id, raw.message),
            Some(_) => Line::Quiet,
        }
    }
}

fn assistant(
    at: &Option<String>,
    cwd: Option<String>,
    request: Option<String>,
    message: Option<&RawValue>,
) -> Line {
    let Some(message) = message.and_then(|m| serde_json::from_str::<Message>(m.get()).ok()) else {
        return Line::Unknown;
    };
    // A message Claude Code made up itself (an error shown as a reply) is
    // not an API call.
    if message.model.as_deref() == Some("<synthetic>") {
        return Line::Quiet;
    }
    let Some(usage) = message.usage.and_then(|u| serde_json::from_str::<RawUsage>(u.get()).ok())
    else {
        return Line::Unknown;
    };
    let Some(at) = at.as_deref().and_then(unix) else { return Line::Unknown };
    let tokens = Tokens {
        input: usage.input_tokens,
        cache_write: usage.cache_creation_input_tokens.unwrap_or(0),
        cache_read: usage.cache_read_input_tokens.unwrap_or(0),
        output: usage.output_tokens,
    };
    let key =
        message.id.as_deref().map(|id| key(&["claude", id, request.as_deref().unwrap_or("")]));
    Line::Usage(Usage {
        at,
        model: message.model.unwrap_or_default(),
        project: cwd.filter(|c| !c.is_empty()),
        tokens,
        key,
    })
}
