// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The popover's Vitest fixtures (`src/fixtures/snapshots/`) are the
//! snapshots the app sends with the advanced part: the report fixtures with
//! its keys (`ui_fixtures.rs` checks them without it, in
//! `tests/fixtures/ui-plain/`). `UPDATE_UI_FIXTURES=1 cargo test -p
//! quota-station-core --test ui_fixtures_advanced` rewrites them after a
//! change to the snapshot shape.

mod common;

use std::path::PathBuf;

use common::fixture;
use common::ui::{add_detail, SCENARIOS};
use quota_station_core::Snapshot;

fn ui_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.join("src/fixtures/snapshots")
}

#[test]
fn ui_fixtures_match_the_snapshots_rust_builds() {
    let update = std::env::var_os("UPDATE_UI_FIXTURES").is_some();
    let mut stale = Vec::new();
    for (name, claude, agy, codex) in SCENARIOS {
        let codex = codex.map(fixture);
        let claude = fixture(claude);
        let mut snapshot =
            Snapshot::build(Some(&claude), Some(&fixture(agy))).with_codex(codex.as_ref());
        // With the advanced part's keys, as the app shows them with it.
        quota_station_core::advanced::snapshot::decorate(&mut snapshot, Some(&claude));
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
