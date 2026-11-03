// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The snapshot with the advanced part's keys, as the app shows it with that
//! part: Claude's pool as the rotation's room, from the report fixtures
//! (`snapshot.rs` has the snapshot without an extension).

mod common;

use common::fixture;
use quota_station_core::quota::Response;
use quota_station_core::snapshot::{Keychain, PoolKind, SectionStatus, Snapshot};
use serde_json::json;

/// The snapshot of these reports as this build's app shows it.
fn shown(claude: Option<&Response>, agy: Option<&Response>) -> Snapshot {
    let mut s = Snapshot::build(claude, agy);
    // With the advanced part's keys.
    quota_station_core::advanced::snapshot::decorate(&mut s, claude);
    s
}

fn claude(name: &str) -> Snapshot {
    shown(Some(&fixture(name)), None)
}

#[test]
fn the_room_pool_counts_the_whole_rotation() {
    let s = claude("claude-pool.json");
    assert_eq!(s.keychain, Keychain::Granted);
    let c = &s.claude;
    assert_eq!(c.status, SectionStatus::Ok);
    assert_eq!(c.accounts.iter().map(|a| a.number).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(c.ext.get("personalHours"), Some(&json!("8-23")));
    assert!(!c.stale);

    let pool = &c.pool;
    assert_eq!(pool.kind, PoolKind::Ext("rotation"));
    assert_eq!(pool.ext.get("room"), Some(&json!(2.27)));
    assert_eq!(pool.ext.get("members"), Some(&json!(4)));
    assert!((pool.percent.unwrap() - 56.75).abs() < 1e-9);
    assert_eq!(pool.next_refill_at.as_deref(), Some("2026-01-16T10:09:00.000Z"));
    assert_eq!(pool.ext.get("refills"), Some(&json!([4])));
    assert!(!pool.exhausted);
    let rooms: Vec<(u64, f64)> = pool
        .ext
        .get("rooms")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["number"].as_u64().unwrap(), r["percent"].as_f64().unwrap()))
        .collect();
    assert_eq!(rooms, [(2, 0.0), (3, 100.0), (4, 97.8), (1, 29.2)]);
}

#[test]
fn exhausted_rotation_names_who_refills_first() {
    let s = claude("claude-exhausted.json");
    let pool = &s.claude.pool;
    assert!(pool.exhausted);
    assert_eq!(pool.ext.get("room"), Some(&json!(0.0)));
    assert_eq!(pool.next_refill_at.as_deref(), Some("2026-01-16T09:40:00.000Z"));
    assert_eq!(pool.ext.get("refills"), Some(&json!([1, 3])));
}
