// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts' file on the core's fixed `accounts.json`
//! (its `fixtures/claude-accounts-file.json`) with the advanced part's keys
//! on the accounts (the core's `account_keys` has it without an
//! extension's): the file read and written again, after a change and after
//! an account is added, `claude status` and its JSON, the state file, the
//! read of the accounts, who counts in alert 1's pool, and an account held
//! out or put back by hand, pinned as
//! `src/advanced/fixtures/claude-accounts.txt` (written before the keys
//! left the app's own types). A temporary home and a fake Keychain: no real
//! account is touched.

mod common;

use quota_station_core::alerts;
use quota_station_core::claude::cli::{self, Cli, Tools};
use quota_station_core::claude::store::Store;
use quota_station_core::claude::{manage, state, usage};
use quota_station_core::ext::Hooks;
use quota_station_core::settings::PoolAlert;
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;

/// 2026-01-16 07:20 UTC.
const NOW: i64 = 1_768_548_000;

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(NOW).unwrap()
}

/// `text` with the times of the clock the test runs at put as `<now>` and
/// `<age>` (an account added now, the state's time and a reading's age).
fn steady(text: &str) -> String {
    let lines: Vec<String> = text
        .trim_end()
        .lines()
        .map(|l| {
            let key = l.trim_start();
            let indent = &l[..l.len() - key.len()];
            let comma = if key.ends_with(',') { "," } else { "" };
            if key.starts_with("\"generatedAt\"") {
                format!("{indent}\"generatedAt\": \"<now>\"{comma}")
            } else if key.starts_with("\"ageSeconds\"") {
                format!("{indent}\"ageSeconds\": \"<age>\"{comma}")
            } else if key.starts_with("\"added\"") && !key.starts_with("\"added\": \"2026-01-0") {
                format!("{indent}\"added\": \"<now>\"{comma}")
            } else {
                l.to_string()
            }
        })
        .collect();
    lines.join("\n") + "\n"
}

fn call(cli: &Cli, hooks: &dyn Hooks, args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(cli, hooks, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

/// A Mac with the fixed file and a reading of each account a minute ago.
fn mac(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    std::fs::create_dir_all(mac.paths.accounts().parent().unwrap()).unwrap();
    std::fs::write(mac.paths.accounts(), common::fixture_text("claude-accounts-file.json"))
        .unwrap();
    let rows: serde_json::Map<String, Value> =
        [(1, 88.0), (2, 10.0), (3, 20.0), (4, 30.0), (5, 5.0)]
            .iter()
            .map(|(n, five): &(u32, f64)| {
                let row = json!({
                    "identity": format!("id-{n}"),
                    "lastGood": {
                        "five_hour": { "pct": five, "resets_at": null },
                        "seven_day": { "pct": 10.0, "resets_at": null }
                    },
                    "fetchedAt": (NOW - 60) as f64
                });
                (n.to_string(), row)
            })
            .collect();
    let cache: usage::Cache =
        serde_json::from_value(json!({ "version": 1, "accounts": rows })).unwrap();
    usage::save(&mac.store, &cache).unwrap();
    mac
}

/// What the app does with the file through `store`, with `hooks` for the
/// command line and the read, as text.
fn walk(mac: &Mac, store: &Store, hooks: &dyn Hooks) -> String {
    let file = || steady(&std::fs::read_to_string(mac.paths.accounts()).unwrap());
    let mut o = String::new();
    let list = store.load().unwrap();
    store.save(&list).unwrap();
    o += &format!("== the file read and written again\n{}", file());
    manage::set_alias(store, 4, "Bốn").unwrap();
    o += &format!("== after an alias on 4\n{}", file());

    let cli = Cli {
        store: store.clone(),
        tools: Tools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
    };
    let (code, out, err) = call(&cli, hooks, &["status"]);
    o += &format!("== claude status ({code})\n{out}{err}");
    let (code, out, err) = call(&cli, hooks, &["status", "--json"]);
    o += &format!("== claude status --json ({code})\n{}{err}", steady(&out));
    state::write(store, Some(4), now()).unwrap();
    let written = std::fs::read_to_string(store.paths.state()).unwrap();
    o += &format!("== state.json\n{}", steady(&written));

    let report = quota_station_core::quota::claude::own_now(store, hooks, now()).unwrap();
    let row = report.primary("claude").unwrap();
    let accounts = serde_json::to_string_pretty(row.claude_accounts.as_ref().unwrap()).unwrap();
    o += &format!("== the read's accounts\n{accounts}\n");
    let policy = hooks.claude_pool(&store.paths);
    let offset = UtcOffset::from_hms(7, 0, 0).unwrap();
    let c = alerts::pool_check(&report, policy.as_deref(), PoolAlert::default(), now(), offset);
    o += &format!("== alert 1's pool\n{:?}\n", c.map(|c| (c.active, c.others)));

    let login = credential("acc-9", "ref-9", in_hours(5.0));
    mac.log_in(&login, &oauth("new@team.example", "uuid-9", "org-team"));
    let added = manage::adopt_live(store).unwrap();
    o += &format!("== after adding the login in use ({})\n{}", added.slot, file());
    o
}

/// With the advanced part an account held out stays held out wherever the
/// app keeps, writes or shows it, a new account is written in the rotation,
/// and an account is held out or put back by hand.
#[test]
fn with_the_private_part_the_accounts_are_the_pinned_ones() {
    use quota_station_core::advanced::held::{self, HeldHooks};
    use quota_station_core::advanced::pool::PoolHooks;

    let mac = mac("account-keys-private");
    let store = Store::new(mac.paths.clone(), mac.keychain.clone()).with_hooks(&HeldHooks);
    let mut text = walk(&mac, &store, &PoolHooks);
    let file = || steady(&std::fs::read_to_string(mac.paths.accounts()).unwrap());
    held::set_enabled(&store, 1, false).unwrap();
    held::set_enabled(&store, 2, true).unwrap();
    text += &format!("== after holding 1 out and putting 2 back\n{}", file());
    let cli = Cli {
        store: store.clone(),
        tools: Tools { claude: None, endpoints: FakeAnthropic::start().endpoints() },
    };
    let (code, out, err) = call(&cli, &PoolHooks, &["status"]);
    text += &format!("== claude status ({code})\n{out}{err}");
    let unknown = held::set_enabled(&store, 9, true);
    text += &format!("== an unknown account\n{:?}\n", unknown.map_err(|e| e.kind()));
    assert_eq!(text, include_str!("../src/advanced/fixtures/claude-accounts.txt"));
}
