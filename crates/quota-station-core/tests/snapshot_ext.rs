// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The snapshot of one fixed set of accounts, as the app shows it with the
//! hooks it has ([`Hooks::decorate_snapshot`]): six Claude accounts the app
//! keeps, each read a minute ago, with Claude Code on account 2 and on none.
//! Pinned by golden files the build wrote before the extension added its
//! own part (`UPDATE_SNAPSHOT_GOLDEN=1` rewrites them after a change the
//! windows are meant to see). The text is compared, so the keys' order
//! counts too.

use std::collections::BTreeMap;
use std::path::Path;

use quota_station_core::claude::store::List;
use quota_station_core::claude::usage;
use quota_station_core::ext::{self, Hooks, NoHooks};
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
    #[allow(unused_mut)]
    let mut accounts: Vec<Value> = (1..=6).map(account).collect();
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

/// Without an extension's part the snapshot is the app's own: the hooks of
/// no extension add nothing to it, and it is the golden one.
#[test]
fn without_an_extension_the_snapshot_is_the_apps_own() {
    let reports = [("plain-2", read(Some(2), None)), ("plain-none", read(None, None))];
    for (_, report) in &reports {
        let built = Snapshot::build(Some(report), None);
        for hooks in [ext::none(), &NoHooks as &dyn Hooks] {
            let snapshot = shown(report, hooks);
            assert_eq!(snapshot, built);
            assert_eq!(text(&[("s", snapshot)].into()), text(&[("s", built.clone())].into()));
        }
    }
    let snapshots = reports.iter().map(|(name, r)| (*name, shown(r, ext::none()))).collect();
    assert_golden(&snapshots, "tests/fixtures/snapshot-accounts.json");
}
