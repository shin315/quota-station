// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude pool on one fixed set of accounts with the advanced part's
//! pool policy (the core's `claude_pool` has the same accounts without a
//! policy): which accounts count in alert 1's check, the best other
//! account, whether Claude is dry, and the room the read of the accounts
//! counts. Everything is built in memory: no account, Keychain or file is
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
    let mut accounts: Vec<Value> = (1..=6).map(account).collect();
    // The owner holds account 5 out of the rotation.
    accounts[4]["disabled"] = json!(true);
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

/// The rotation on the same accounts: Team 2 to 5, personal 1 at any hour,
/// `full = 90`, and Team 4 gone an hour ago.
fn rules() -> quota_station_core::rotation::Rules {
    use quota_station_core::rotation::{Expire, Rules};
    Rules {
        team: vec![2, 3, 4, 5],
        personal: Some(1),
        personal_from: 0,
        personal_to: 23,
        full: 90.0,
        fable_full: 95.0,
        expire: vec![Expire { slot: 4, at: rfc(now() - Duration::hours(1)) }],
    }
}

/// With the advanced part's policy only the accounts the rotation may move
/// to count: not 4, past its end, nor 5, held out, nor 6, outside the
/// rotation. Claude is then dry, and the best other account is 3. The read
/// writes the rotation's keys and its room: every account it may move to
/// with a fresh reading, held out or not, each by its tighter window before
/// `full`, and the first reset that gives one more room. From the report's
/// rotation or from the rotation file's rules, the check is the same.
#[test]
fn with_the_private_part_the_rotation_decides_who_counts_and_counts_its_room() {
    use quota_station_core::advanced::pool::Policy;

    let policy = Policy { rules: Some(rules()) };
    let report = read(Some(&policy));
    let accounts = accounts_json(&report);
    assert_eq!(
        accounts["rotation"],
        json!({
            "team": [2, 3, 4, 5], "personal": 1, "personalHours": "0-23", "full": 90.0,
            "expire": [{ "slot": 4, "at": "2026-01-16T06:20:00.000Z" }]
        })
    );
    assert_eq!(
        accounts["pool"],
        json!({
            "scope": "all_models", "members": 4, "slotsOfRoom": 1.01,
            "effectivePercentRemaining": 100.0, "nextRefillAt": "2026-01-16T08:20:00.000Z",
            "slots": [
                { "slot": 2, "roomPercent": 0.0, "refillAt": "2026-01-16T09:20:00.000Z" },
                { "slot": 3, "roomPercent": 4.4, "refillAt": "2026-01-16T08:20:00.000Z" },
                { "slot": 5, "roomPercent": 94.4 },
                { "slot": 1, "roomPercent": 2.2 }
            ]
        })
    );
    assert_eq!(accounts.get("poolReason"), None);

    let c = check(&report, Some(&policy));
    assert_eq!(c.others, [(1, 12.0), (3, 14.0)]);
    assert_eq!((c.active, c.active_left), (Some(2), 8.0));
    assert!(c.dry);
    assert_eq!(c.best_other(), Some((3, 14.0)));
    assert_eq!(c.better(), Some((3, 14.0)));

    // A report without the rotation's keys: the rotation file's rules
    // decide, the same way; with neither, a held-out account still never
    // counts.
    let plain = read(None);
    assert_eq!(check(&plain, Some(&policy)), c);
    let none = check(&plain, Some(&Policy { rules: None }));
    assert_eq!(none.others, [(1, 12.0), (3, 14.0), (4, 90.0), (6, 80.0)]);
    assert_eq!(none.best_other(), Some((4, 90.0)));

    // Without the rules a read writes nothing of the rotation.
    let unread = accounts_json(&read(Some(&Policy { rules: None })));
    assert_eq!(unread, accounts_json(&plain));
}
