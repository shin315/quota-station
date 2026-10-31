// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The popover's Vitest fixtures are snapshots this crate builds from the
//! report fixtures, so the UI is tested against what Rust really sends.
//! `UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test ui_fixtures`
//! rewrites them after a change to the snapshot shape.

mod common;

use std::path::PathBuf;

use common::{fixture, fixture_text};
use quota_station_core::detail::{self, AgyDetail};
use quota_station_core::Snapshot;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// (UI fixture, claude report, agy report, codex report)
const SCENARIOS: &[(&str, &str, &str, Option<&str>)] = &[
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

fn ui_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    // @private:begin
    // Without the private part a snapshot has no roles and no room pool:
    // those fixtures are the community tree's (community/make-fixtures.py).
    if cfg!(not(feature = "author-tools")) {
        return root.join("community/overlay/src/fixtures/snapshots");
    }
    // @private:end
    root.join("src/fixtures/snapshots")
}

/// Every window of every account, with names, as the app adds them.
fn add_detail(snapshot: &mut Snapshot) {
    let now = snapshot
        .claude
        .generated_at
        .as_deref()
        .and_then(|t| OffsetDateTime::parse(t, &Rfc3339).ok());
    let list = common::claude_details("cswap-list-pool.json");
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

#[test]
fn ui_fixtures_match_the_snapshots_rust_builds() {
    let update = std::env::var_os("UPDATE_UI_FIXTURES").is_some();
    let mut stale = Vec::new();
    for (name, claude, agy, codex) in SCENARIOS {
        let codex = codex.map(fixture);
        let mut snapshot =
            Snapshot::build(Some(&fixture(claude)), Some(&fixture(agy))).with_codex(codex.as_ref());
        if *name == "fresh" {
            add_detail(&mut snapshot);
        }
        let text = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
        let file = ui_dir().join(format!("{name}.json"));
        if update {
            std::fs::create_dir_all(ui_dir()).unwrap();
            std::fs::write(&file, &text).unwrap();
        } else if std::fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
            stale.push(*name);
        }
    }
    assert!(
        stale.is_empty(),
        "UI fixtures out of date: {stale:?}; rerun with UPDATE_UI_FIXTURES=1"
    );
}
