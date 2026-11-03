// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The snapshot of one fixed set of accounts, as the app shows it with the
//! advanced part's hooks: six Claude accounts the app keeps, each read a
//! minute ago, with Claude Code on account 2 and on none, read without the
//! rotation's rules and with them (`snapshot_ext.rs` has it without an
//! extension). Pinned by a golden file the build wrote before
//! the extension added its own part (`UPDATE_SNAPSHOT_GOLDEN=1` rewrites it
//! after a change the windows are meant to see). The text is compared, so
//! the keys' order counts too.

use std::collections::BTreeMap;
use std::path::Path;

use quota_station_core::claude::store::List;
use quota_station_core::claude::usage;
use quota_station_core::ext::Hooks;
use quota_station_core::quota::claude::{own, Own, PoolPolicy};
use quota_station_core::quota::Response;
use quota_station_core::Snapshot;
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime};

/// 2026-01-16 07:20 UTC.
const NOW: i64 = 1_768_548_000;

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW).unwrap()
}

fn rfc(t: OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339).unwrap()
}

/// Six accounts the app keeps.
fn list() -> List {
    let account = |n: u32| {
        json!({
            "slot": n, "kind": "login",
            "accountUuid": format!("uuid-{n}"), "organizationUuid": "org",
            "identity": format!("id-{n}"), "added": "2026-01-01T00:00:00Z"
        })
    };
    let mut accounts: Vec<Value> = (1..=6).map(account).collect();
    // The owner holds account 5 out of the rotation.
    {
        accounts[4]["disabled"] = json!(true);
    }
    serde_json::from_value(json!({ "version": 1, "accounts": accounts })).unwrap()
}

/// An account's number, the five-hour window's percent used and its reset,
/// the weekly window's and its reset, resets in hours from now.
type Reading = (u32, f64, Option<i64>, f64, Option<i64>);

const READINGS: [Reading; 6] = [
    (1, 88.0, None, 20.0, None),
    (2, 92.0, Some(2), 40.0, Some(72)),
    (3, 86.0, Some(1), 50.0, Some(96)),
    (4, 10.0, None, 10.0, None),
    (5, 0.0, None, 5.0, None),
    (6, 20.0, None, 20.0, None),
];

fn usage() -> usage::Cache {
    let at = |hours: Option<i64>| hours.map(|h| rfc(now() + Duration::hours(h)));
    let rows: serde_json::Map<String, Value> = READINGS
        .iter()
        .map(|(n, five, five_at, week, week_at)| {
            let row = json!({
                "identity": format!("id-{n}"),
                "lastGood": {
                    "five_hour": { "pct": five, "resets_at": at(*five_at) },
                    "seven_day": { "pct": week, "resets_at": at(*week_at) }
                },
                "fetchedAt": (NOW - 60) as f64
            });
            (n.to_string(), row)
        })
        .collect();
    serde_json::from_value(json!({ "version": 1, "accounts": rows })).unwrap()
}

/// The read of the accounts with Claude Code on `active`, by `policy`.
fn read(active: Option<u32>, policy: Option<&dyn PoolPolicy>) -> Response {
    let (list, usage) = (list(), usage());
    own(&Own { list: &list, usage: &usage, active, pool: policy }, now())
}

/// The snapshot of `report` as the app shows it with `hooks`.
fn shown(report: &Response, hooks: &dyn Hooks) -> Snapshot {
    let mut snapshot = Snapshot::build(Some(report), None);
    hooks.decorate_snapshot(&mut snapshot, Some(report));
    snapshot
}

/// The snapshots by name, as the golden files have them.
fn text(snapshots: &BTreeMap<&str, Snapshot>) -> String {
    serde_json::to_string_pretty(snapshots).unwrap() + "\n"
}

/// Compares `snapshots` with the golden file `path` (from the crate's
/// folder), or rewrites it.
fn assert_golden(snapshots: &BTreeMap<&str, Snapshot>, path: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = text(snapshots);
    if std::env::var_os("UPDATE_SNAPSHOT_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).expect("the golden file");
    assert_eq!(text, golden, "the snapshot differs from {}", path.display());
}

/// With the advanced part, the snapshot has each account's
/// place in the rotation, held out or not, in it or out of it and when it
/// leaves it, the rotation's personal hours, and Claude's pool as the
/// rotation's room (or why there is none), on the same accounts read
/// without the rotation's rules and with them: Team 2 to 5, personal 1 at
/// any hour, `full = 90`, Team 4 gone an hour ago and Team 3 leaving in
/// three hours.
#[test]
fn with_the_advanced_part_the_snapshot_has_the_rotations_keys() {
    use quota_station_core::advanced::pool::Policy;
    use quota_station_core::advanced::snapshot::SnapshotHooks;
    use quota_station_core::rotation::{Expire, Rules};

    let rules = Rules {
        team: vec![2, 3, 4, 5],
        personal: Some(1),
        personal_from: 0,
        personal_to: 23,
        full: 90.0,
        fable_full: 95.0,
        expire: vec![
            Expire { slot: 4, at: rfc(now() - Duration::hours(1)) },
            Expire { slot: 3, at: rfc(now() + Duration::hours(3)) },
        ],
    };
    let policy = Policy { rules: Some(rules) };
    let snapshots = [
        ("plain-2", read(Some(2), None)),
        ("plain-none", read(None, None)),
        ("rules-2", read(Some(2), Some(&policy))),
        ("rules-none", read(None, Some(&policy))),
    ]
    .iter()
    .map(|(name, report)| (*name, shown(report, &SnapshotHooks)))
    .collect();
    assert_golden(&snapshots, "src/advanced/fixtures/snapshot-accounts.json");
}
