//! The popover's Vitest fixtures are snapshots this crate builds from the
//! quota-axi fixtures, so the UI is tested against what Rust really sends.
//! `UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test ui_fixtures`
//! rewrites them after a change to the snapshot shape.

mod common;

use std::path::PathBuf;

use common::fixture;
use quota_station_core::Snapshot;

/// (UI fixture, claude report, agy report)
const SCENARIOS: &[(&str, &str, &str)] = &[
    ("fresh", "claude-pool-schema5.json", "agy-cockpit.json"),
    ("no-cockpit", "claude-pool-schema5.json", "agy-no-cockpit.json"),
    ("keychain", "claude-keychain-required.json", "agy-no-cockpit.json"),
    ("stale", "claude-stale.json", "agy-cockpit.json"),
    ("reset-pending", "claude-reset-pending.json", "agy-cockpit.json"),
    ("exhausted", "claude-exhausted.json", "agy-cockpit.json"),
    ("no-rotation", "claude-no-rotation.json", "agy-no-cockpit.json"),
    ("unavailable", "claude-unavailable.json", "agy-cockpit-unavailable.json"),
    ("no-antigravity", "claude-pool-schema5.json", "agy-not-set-up.json"),
    ("agy-pool", "claude-pool-schema5.json", "agy-pool.json"),
];

fn ui_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/fixtures/snapshots")
}

#[test]
fn ui_fixtures_match_the_snapshots_rust_builds() {
    let update = std::env::var_os("UPDATE_UI_FIXTURES").is_some();
    let mut stale = Vec::new();
    for (name, claude, agy) in SCENARIOS {
        let snapshot = Snapshot::build(Some(&fixture(claude)), Some(&fixture(agy)));
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
