// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The popover's Vitest fixtures: the reports each one is built from, and
//! the details the app adds to the fresh one.

use quota_station_core::detail::{self, AgyDetail};
use quota_station_core::Snapshot;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::fixture_text;

/// (UI fixture, claude report, agy report, codex report)
pub const SCENARIOS: &[(&str, &str, &str, Option<&str>)] = &[
    ("fresh", "claude-pool.json", "agy-list.json", Some("codex-chatgpt.json")),
    ("single", "claude-pool.json", "agy-single.json", None),
    ("keychain", "claude-keychain-required.json", "agy-single.json", None),
    ("stale", "claude-stale.json", "agy-list.json", None),
    ("reset-pending", "claude-reset-pending.json", "agy-list.json", None),
    ("exhausted", "claude-exhausted.json", "agy-list.json", None),
    ("unnamed", "claude-unnamed.json", "agy-single.json", None),
    (
        "unavailable",
        "claude-unavailable.json",
        "agy-list-unavailable.json",
        Some("codex-unavailable.json"),
    ),
    ("no-antigravity", "claude-pool.json", "agy-not-set-up.json", Some("codex-not-set-up.json")),
];

/// Every window of every account, with names, as the app adds them.
pub fn add_detail(snapshot: &mut Snapshot) {
    let now = snapshot
        .claude
        .generated_at
        .as_deref()
        .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok());
    let list = super::claude_details("claude-list-pool.json");
    detail::apply_claude(&mut snapshot.claude, &list, now);
    let reading = |name: &str| serde_json::from_str(&fixture_text(name)).ok();
    let details = [
        AgyDetail {
            number: 1,
            email: Some("lanpham.ultra@example.com".into()),
            payload: reading("agy-reading-1.json"),
        },
        AgyDetail {
            number: 2,
            email: Some("lan.ultra2@example.com".into()),
            payload: reading("agy-reading-2.json"),
        },
    ];
    detail::apply_agy(&mut snapshot.agy, &details, now);
}
