// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation inside the app: the decisions the old `cswap-prefer-team`
//! script made, with the same rules, and one run of it against a fake
//! Keychain, launchd and Anthropic. Every account is made up, and the
//! clock is an argument, never waited on.

mod common;

use std::collections::BTreeMap;
use std::process::Command;

use quota_station_core::claude::engine::{self, Input, Plan, Reading, Window};
use quota_station_core::claude::rotate::{self, Tools};
use quota_station_core::claude::switch;
use quota_station_core::local_time;
use quota_station_core::rotation::{self, Expire, Rules};
use serde_json::json;
use time::{Duration, OffsetDateTime};

use common::accounts::{credential, in_hours, FakeAnthropic, Mac};

/// 2030-01-16 07:20 UTC.
fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_894_778_400).unwrap()
}

fn at(minutes: i64) -> OffsetDateTime {
    now() + Duration::minutes(minutes)
}

fn hm(t: OffsetDateTime) -> String {
    local_time::clock(t)
}

/// A fresh reading: `five`% of the five-hour window used, ending in
/// `ends` minutes (none: not started), and the week at 10%.
fn acct(number: u32, five: f64, ends: Option<i64>) -> Reading {
    Reading {
        number,
        fresh: true,
        five_hour: Window { used: Some(five), resets_at: ends.map(at) },
        seven_day: Window { used: Some(10.0), resets_at: Some(at(3 * 24 * 60)) },
        ..Reading::default()
    }
}

/// The rules the tests' rotation file holds: Team 2 3 4, personal 1 from 8
/// to 23, full at 90%, Fable full at 95%.
fn rules() -> Rules {
    common::sample_rules()
}

fn decide(rules: &Rules, accounts: &[Reading], active: Option<u32>, hour: u8) -> Plan {
    engine::decide(&Input { rules, accounts, active, now: now(), hour, prime: None })
}

fn decide_primed(accounts: &[Reading], active: Option<u32>, last: &BTreeMap<u32, f64>) -> Plan {
    engine::decide(&Input {
        rules: &rules(),
        accounts,
        active,
        now: now(),
        hour: 12,
        prime: Some(last),
    })
}

#[test]
fn keeps_the_team_account_in_use_until_it_is_full() {
    let accounts = [acct(1, 0.0, None), acct(2, 89.0, Some(60)), acct(3, 0.0, Some(30))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).decision, "stay");
}

#[test]
fn a_full_account_moves_to_the_team_account_whose_window_ends_first() {
    let accounts = [acct(2, 90.0, Some(60)), acct(3, 40.0, Some(200)), acct(4, 70.0, Some(100))];
    let plan = decide(&rules(), &accounts, Some(2), 12);
    assert_eq!(plan.decision, format!("switch 4 (70.0%, ends {})", hm(at(100))));
}

#[test]
fn the_weekly_window_counts_toward_full() {
    let mut weekly_full = acct(2, 5.0, Some(60));
    weekly_full.seven_day.used = Some(93.0);
    let accounts = [weekly_full, acct(3, 20.0, Some(200))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(3));
}

#[test]
fn a_window_not_started_comes_after_a_running_one() {
    let accounts = [acct(2, 95.0, Some(60)), acct(3, 0.0, None), acct(4, 80.0, Some(250))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(4));
    let only_fresh = [acct(2, 95.0, Some(60)), acct(3, 0.0, None)];
    assert_eq!(
        decide(&rules(), &only_fresh, Some(2), 12).decision,
        "switch 3 (10.0%, window not started)"
    );
}

#[test]
fn an_account_out_of_fable_comes_last() {
    let mut fable_out = acct(3, 10.0, Some(30));
    fable_out.fable = Window { used: Some(96.0), resets_at: Some(at(2000)) };
    let accounts = [acct(2, 95.0, Some(60)), fable_out, acct(4, 50.0, Some(250))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(4));
}

#[test]
fn the_personal_account_only_in_its_hours_when_no_team_account_has_room() {
    let accounts = [acct(1, 20.0, None), acct(2, 95.0, Some(60)), acct(3, 92.0, Some(90))];
    assert_eq!(
        decide(&rules(), &accounts, Some(2), 12).decision,
        "switch 1 (20.0%, no Team slot with room)"
    );
    assert_eq!(decide(&rules(), &accounts, Some(1), 12).decision, "stay");
    // At 07:00, before its hours, the Team account in use is used up instead.
    assert_eq!(
        decide(&rules(), &accounts, Some(2), 7).decision,
        "stay (no room anywhere, using the last 5% of slot 2)"
    );
    // A Team account with room always wins over the personal one.
    let team_room = [acct(1, 0.0, None), acct(2, 95.0, Some(60)), acct(3, 50.0, Some(90))];
    assert_eq!(decide(&rules(), &team_room, Some(2), 12).target(), Some(3));
}

#[test]
fn the_last_percent_of_the_account_in_use_comes_before_parking() {
    let accounts = [acct(1, 95.0, None), acct(2, 93.0, Some(60)), acct(3, 99.0, Some(30))];
    assert_eq!(
        decide(&rules(), &accounts, Some(2), 12).decision,
        "stay (no room anywhere, using the last 7% of slot 2)"
    );
}

#[test]
fn with_nothing_left_it_parks_on_the_window_that_resets_first() {
    let accounts = [acct(1, 100.0, None), acct(2, 100.0, Some(60)), acct(3, 100.0, Some(30))];
    assert_eq!(
        decide(&rules(), &accounts, Some(2), 12).decision,
        format!("switch 3 (park: no room anywhere, it resets first at {})", hm(at(30)))
    );
    assert_eq!(
        decide(&rules(), &accounts, Some(3), 12).decision,
        format!("stay (waiting for slot 3 at {})", hm(at(30)))
    );
    let none_started = [acct(2, 100.0, None), acct(3, 100.0, None)];
    assert_eq!(
        decide(&rules(), &none_started, Some(2), 12).decision,
        "wait (no account with room)"
    );
}

#[test]
fn an_unknown_reading_of_the_account_in_use_holds_the_rotation() {
    let mut unknown = acct(2, 95.0, Some(60));
    unknown.fresh = false;
    let accounts = [unknown, acct(3, 0.0, Some(30))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).decision, "stay (slot 2 usage unknown)");
}

#[test]
fn a_stale_reading_never_counts_as_room() {
    let mut stale = acct(3, 0.0, Some(30));
    stale.fresh = false;
    let accounts = [acct(2, 95.0, Some(60)), stale, acct(4, 60.0, Some(200))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(4));
}

#[test]
fn an_account_that_ends_for_good_is_used_first_then_left() {
    let mut ending = rules();
    ending.expire = vec![Expire { slot: 4, at: "2030-01-16T09:00Z".into() }];
    let accounts = [acct(2, 10.0, Some(60)), acct(3, 0.0, None), acct(4, 50.0, Some(200))];
    let ends = OffsetDateTime::from_unix_timestamp(1_894_784_400).unwrap();
    assert_eq!(
        decide(&ending, &accounts, Some(2), 12).decision,
        format!("switch 4 (ends for good at {}, use it first)", hm(ends))
    );
    ending.expire = vec![Expire { slot: 4, at: "2030-01-16T07:00Z".into() }];
    let full = [acct(2, 95.0, Some(60)), acct(3, 99.0, Some(30)), acct(4, 0.0, Some(10))];
    // Past its time, even an account with room is out of every choice.
    assert_ne!(decide(&ending, &full, Some(2), 12).target(), Some(4));
}

#[test]
fn an_account_held_out_by_the_owner_is_never_chosen() {
    let mut held = acct(3, 0.0, Some(30));
    held.disabled = true;
    let accounts = [acct(2, 95.0, Some(60)), held, acct(4, 60.0, Some(200))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(4));
    let mut held_personal = acct(1, 0.0, None);
    held_personal.disabled = true;
    let no_team_room = [held_personal, acct(2, 95.0, Some(60))];
    assert_ne!(decide(&rules(), &no_team_room, Some(2), 12).target(), Some(1));
}

#[test]
fn an_account_held_out_by_the_owner_is_kept_while_in_use_by_the_usual_rules() {
    let mut held = acct(2, 60.0, Some(60));
    held.disabled = true;
    let accounts = [held, acct(3, 0.0, Some(30))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).decision, "stay");
    let mut full = acct(2, 95.0, Some(60));
    full.disabled = true;
    let accounts = [full, acct(3, 0.0, Some(30))];
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).target(), Some(3));
    let mut personal = acct(1, 20.0, None);
    personal.disabled = true;
    let no_team_room = [personal, acct(2, 95.0, Some(60)), acct(3, 99.0, Some(30))];
    assert_eq!(decide(&rules(), &no_team_room, Some(1), 12).decision, "stay");
}

#[test]
fn the_threshold_comes_from_the_rules() {
    let mut low = rules();
    low.full = 50.0;
    let accounts = [acct(2, 60.0, Some(60)), acct(3, 10.0, Some(200))];
    assert_eq!(decide(&low, &accounts, Some(2), 12).target(), Some(3));
    assert_eq!(decide(&rules(), &accounts, Some(2), 12).decision, "stay");
}

#[test]
fn priming_starts_idle_windows_at_most_every_half_hour() {
    let accounts = [acct(2, 20.0, Some(60)), acct(3, 0.0, None), acct(4, 0.0, None)];
    let secs = now().unix_timestamp() as f64;
    let last = BTreeMap::from([(3, secs - 10.0 * 60.0), (4, secs - 31.0 * 60.0)]);
    let plan = decide_primed(&accounts, Some(2), &last);
    assert_eq!(plan.decision, "stay");
    assert_eq!(plan.primes, vec![4]);
    // Never the account switched to, nor one whose window already runs.
    let full = [acct(2, 95.0, Some(60)), acct(3, 0.0, None), acct(4, 30.0, Some(100))];
    let plan = decide_primed(&full, Some(2), &BTreeMap::new());
    assert_eq!(plan.target(), Some(4));
    assert_eq!(plan.primes, vec![3]);
}

/// The old script on the same cases, when `QUOTA_STATION_PREFER_TEAM` names
/// it: `cargo test --test claude_rotation -- --ignored`.
#[test]
#[ignore]
fn decisions_match_the_old_script() {
    let Some(script) = std::env::var_os("QUOTA_STATION_PREFER_TEAM") else { return };
    let cases: Vec<(Vec<Reading>, Option<u32>, u8)> = vec![
        (vec![acct(2, 89.0, Some(60)), acct(3, 0.0, Some(30))], Some(2), 12),
        (
            vec![acct(2, 90.0, Some(60)), acct(3, 40.0, Some(200)), acct(4, 70.0, Some(100))],
            Some(2),
            12,
        ),
        (vec![acct(2, 95.0, Some(60)), acct(3, 0.0, None)], Some(2), 12),
        (vec![acct(1, 20.0, None), acct(2, 95.0, Some(60)), acct(3, 92.0, Some(90))], Some(2), 12),
        (vec![acct(1, 20.0, None), acct(2, 95.0, Some(60)), acct(3, 92.0, Some(90))], Some(2), 7),
        (vec![acct(1, 95.0, None), acct(2, 93.0, Some(60)), acct(3, 99.0, Some(30))], Some(2), 12),
        (
            vec![acct(1, 100.0, None), acct(2, 100.0, Some(60)), acct(3, 100.0, Some(30))],
            Some(2),
            12,
        ),
        (vec![acct(2, 100.0, None), acct(3, 100.0, None)], Some(2), 12),
    ];
    for (accounts, active, hour) in cases {
        let home = common::temp_dir("prefer-team");
        let input = home.join("list.json");
        let list: Vec<_> = accounts
            .iter()
            .map(|a| {
                json!({
                    "number": a.number,
                    "usageStatus": "ok",
                    "usageAgeSeconds": 10,
                    "usage": {
                        "fiveHour": { "pct": a.five_hour.used, "resetsAt": a.five_hour.resets_at.map(|t| t.format(&time::format_description::well_known::Rfc3339).unwrap()) },
                        "sevenDay": { "pct": a.seven_day.used }
                    }
                })
            })
            .collect();
        std::fs::write(
            &input,
            json!({ "activeAccountNumber": active, "accounts": list }).to_string(),
        )
        .unwrap();
        let out = Command::new(&script)
            .arg("--dry-run")
            .env("HOME", &home)
            .env("CSWAP_TEST_JSON", &input)
            .env("CSWAP_TEST_NOW", now().unix_timestamp().to_string())
            .env("CSWAP_TEST_HOUR", hour.to_string())
            .env("CSWAP_TEAM_EXPIRE", "")
            .output()
            .unwrap();
        let theirs = String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").to_string();
        assert_eq!(decide(&rules(), &accounts, active, hour).decision, theirs);
    }
}

// One run against the fakes.

fn iso(hours: f64) -> String {
    let t = OffsetDateTime::now_utc() + Duration::seconds((hours * 3600.0) as i64);
    t.format(&time::format_description::well_known::Rfc3339).unwrap()
}

/// Team 2 and 3 with logins in the app, Claude Code on 2, and a rotation
/// file with [`rules`].
fn three_accounts(tag: &str) -> (Mac, FakeAnthropic, Tools) {
    let mac = Mac::new(tag);
    let anthropic = FakeAnthropic::start();
    mac.add(1, "lan@example.com", "org-personal", &credential("acc-1", "ref-1", in_hours(5.0)));
    mac.add(2, "lan@team.example", "org-team", &credential("acc-2", "ref-2", in_hours(5.0)));
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(
        &credential("acc-2", "ref-2", in_hours(5.0)),
        &common::accounts::oauth("lan@team.example", "uuid-2", "org-team"),
    );
    let file = mac.paths.rotation_file();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let stamp = rotation::Stamp::new(OffsetDateTime::now_utc(), "Mac A");
    std::fs::write(&file, rotation::render(&rules(), &stamp, None)).unwrap();
    let tools = Tools {
        launchctl: mac.launchctl.clone(),
        claude: None,
        pause_file: mac.home.join(".config/cswap-prefer-team/pause"),
        endpoints: anthropic.endpoints(),
    };
    (mac, anthropic, tools)
}

fn log(mac: &Mac) -> String {
    std::fs::read_to_string(mac.paths.log()).unwrap_or_default()
}

#[test]
fn a_run_moves_claude_code_off_a_full_account_and_logs_it() {
    let (mac, anthropic, tools) = three_accounts("rotate-run");
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    let decided = rotate::run(&mac.store, &tools);
    assert_eq!(decided.line.split(' ').take(2).collect::<Vec<_>>(), ["switch", "3"]);
    assert_eq!(switch::active_slot(&mac.store), Some(3));
    assert_eq!(mac.live_email().as_deref(), Some("lan.b@team.example"));
    assert!(
        log(&mac).lines().any(|l| l.contains(" switch to slot 3 (20.0%, ends ")),
        "{}",
        log(&mac)
    );
}

#[test]
fn a_run_without_a_rotation_file_switches_nothing() {
    // The app has no built-in rotation: a Mac that never saved one has none.
    let (mac, anthropic, tools) = three_accounts("rotate-no-rules");
    std::fs::remove_file(mac.paths.rotation_file()).unwrap();
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    let decided = rotate::run(&mac.store, &tools);
    assert_eq!(decided.line, rotate::NO_RULES);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(!log(&mac).contains("switch to slot"));
    assert_eq!(rotate::rules(&mac.store), None);
    let now = OffsetDateTime::now_utc();
    assert_eq!(rotate::dry_run(&mac.store, &tools, None, now).line, rotate::NO_RULES);
}

#[test]
fn a_run_while_the_pause_holds_changes_nothing() {
    let (mac, anthropic, tools) = three_accounts("rotate-paused");
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    std::fs::create_dir_all(tools.pause_file.parent().unwrap()).unwrap();
    std::fs::write(&tools.pause_file, "until = 2999-01-01T00:00\nreason = thử máy\n").unwrap();
    let decided = rotate::run(&mac.store, &tools);
    assert_eq!(decided.line, "paused until 2999-01-01T00:00: thử máy");
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(anthropic.requests().is_empty());
    assert!(tools.pause_file.exists());
    assert_eq!(log(&mac), "");
}

#[test]
fn a_run_after_the_pause_ended_removes_the_flag_and_goes_on() {
    let (mac, anthropic, tools) = three_accounts("rotate-ended");
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    std::fs::create_dir_all(tools.pause_file.parent().unwrap()).unwrap();
    std::fs::write(&tools.pause_file, "until = 2020-01-01T00:00\n").unwrap();
    rotate::run(&mac.store, &tools);
    assert!(!tools.pause_file.exists());
    assert!(log(&mac).lines().next().unwrap().ends_with(" pause ended (until 2020-01-01T00:00)"));
    assert_eq!(switch::active_slot(&mac.store), Some(3));
}

#[test]
fn a_run_while_the_old_switcher_is_loaded_neither_switches_nor_refreshes() {
    let (mac, anthropic, tools) = three_accounts("rotate-legacy");
    mac.load_label("com.cswap.prefer-team");
    // Slot 3's token has expired: only a refresh could read it.
    let expired = credential("acc-3", "ref-3", in_hours(-1.0));
    mac.store.set_credential(3, &expired).unwrap();
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3b", 10.0, Some(&iso(2.0)), Some(20.0), None);
    let decided = rotate::run(&mac.store, &tools);
    assert_eq!(decided.line, "blocked (com.cswap.prefer-team is loaded)");
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(mac.store.credential(3).unwrap(), Some(expired));
    assert!(!log(&mac).contains("switch to slot"));
}

#[test]
fn a_run_when_launchctl_cannot_tell_neither_switches_nor_refreshes() {
    let (mac, anthropic, mut tools) = three_accounts("rotate-launchctl-unknown");
    tools.launchctl = common::script(&mac.dir.join("bin/launchctl-unknown"), "exit 1\n");
    let expired = credential("acc-3", "ref-3", in_hours(-1.0));
    mac.store.set_credential(3, &expired).unwrap();
    anthropic.refreshes("ref-3", "acc-3b", "ref-3b");
    anthropic.usage("acc-2", 96.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3b", 10.0, Some(&iso(2.0)), Some(20.0), None);
    let decided = rotate::run(&mac.store, &tools);
    assert!(decided.line.starts_with("blocked ("), "{}", decided.line);
    assert_eq!(switch::active_slot(&mac.store), Some(2));
    assert!(anthropic.to("/v1/oauth/token").is_empty());
    assert_eq!(mac.store.credential(3).unwrap(), Some(expired));
}

#[test]
fn a_dry_run_reads_nothing_over_the_network() {
    let (mac, anthropic, tools) = three_accounts("rotate-dry");
    let decided = rotate::dry_run(&mac.store, &tools, Some(&rules()), OffsetDateTime::now_utc());
    // No readings yet: the account in use is unknown, so the rotation holds.
    assert_eq!(decided.line, "stay (slot 2 usage unknown)");
    assert!(anthropic.requests().is_empty());
}

#[test]
fn the_preview_decides_with_the_draft_rules() {
    let (mac, anthropic, tools) = three_accounts("rotate-preview");
    anthropic.usage("acc-1", 0.0, None, Some(5.0), None);
    anthropic.usage("acc-2", 60.0, Some(&iso(1.0)), Some(40.0), None);
    anthropic.usage("acc-3", 10.0, Some(&iso(2.0)), Some(20.0), None);
    quota_station_core::claude::usage::read_due(&mac.store, &tools.endpoints, false, true);
    let now = OffsetDateTime::now_utc();
    assert_eq!(rotate::dry_run(&mac.store, &tools, Some(&rules()), now).line, "stay");
    let mut draft = rules();
    draft.full = 50.0;
    assert_eq!(rotate::dry_run(&mac.store, &tools, Some(&draft), now).primes, Vec::<u32>::new());
    assert!(rotate::dry_run(&mac.store, &tools, Some(&draft), now).line.starts_with("switch 3 "));
}
