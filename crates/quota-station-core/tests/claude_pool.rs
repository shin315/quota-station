// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude pool on one fixed set of accounts: which accounts count in
//! alert 1's check, the best other account, whether Claude is dry, and the
//! room the read of the accounts counts, without an extension's pool
//! policy. Everything is built in memory: no account, Keychain or file is
//! read.

use quota_station_core::alerts;
use quota_station_core::claude::store::List;
use quota_station_core::claude::usage;
use quota_station_core::quota::claude::{own, Own, PoolPolicy};
use quota_station_core::quota::Response;
use quota_station_core::settings::PoolAlert;
use serde_json::{json, Value};
use time::{Duration, OffsetDateTime, UtcOffset};

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
    let accounts: Vec<Value> = (1..=6).map(account).collect();
    serde_json::from_value(json!({ "version": 1, "accounts": accounts })).unwrap()
}

/// An account's reading: its number, the five-hour window's percent used
/// and its reset, the weekly window's and its reset, resets in hours from
/// now.
type Reading = (u32, f64, Option<i64>, f64, Option<i64>);

/// Each account's reading a minute ago.
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

/// The read of the accounts, Claude Code on account 2, with `policy`.
fn read(policy: Option<&dyn PoolPolicy>) -> Response {
    let (list, usage) = (list(), usage());
    own(&Own { list: &list, usage: &usage, active: Some(2), pool: policy }, now())
}

/// Alert 1's check at the read's time with the default thresholds.
fn check(report: &Response, policy: Option<&dyn PoolPolicy>) -> alerts::PoolCheck {
    let offset = UtcOffset::from_hms(7, 0, 0).unwrap();
    alerts::pool_check(report, policy, PoolAlert::default(), now(), offset).unwrap()
}

/// The accounts' row as JSON, for the keys a policy may add.
fn accounts_json(report: &Response) -> Value {
    let row = report.primary("claude").unwrap();
    serde_json::to_value(row.claude_accounts.as_ref().unwrap()).unwrap()
}

/// Without a pool policy every account counts, the best other account is
/// the one with the most left, Claude is not dry, and the read counts no
/// room.
#[test]
fn without_a_policy_every_account_counts_and_no_room_is_counted() {
    let report = read(None);
    let accounts = accounts_json(&report);
    let keys: Vec<&str> = accounts.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["activeSlot", "slots"]);
    let left: Vec<(u64, f64)> = accounts["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["slot"].as_u64().unwrap(), s["effectivePercentRemaining"].as_f64().unwrap()))
        .collect();
    assert_eq!(left, [(1, 12.0), (2, 8.0), (3, 14.0), (4, 90.0), (5, 95.0), (6, 80.0)]);

    let c = check(&report, None);
    assert_eq!(c.others, [(1, 12.0), (3, 14.0), (4, 90.0), (5, 95.0), (6, 80.0)]);
    assert_eq!((c.active, c.active_left), (Some(2), 8.0));
    assert!(!c.dry);
    assert_eq!(c.best_other(), Some((5, 95.0)));
    assert_eq!(c.better(), Some((5, 95.0)));
}
