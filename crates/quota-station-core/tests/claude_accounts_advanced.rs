// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Claude accounts' data file with the advanced part (the core's
//! `claude_accounts` tests the app's own), against a fake Keychain and a
//! fake Anthropic in a temporary home. Every login is made up.

mod common;

use quota_station_core::claude::state;
use quota_station_core::claude::switch;
use quota_station_core::claude::usage;
use serde_json::Value;
use time::OffsetDateTime;

use common::accounts::{credential, in_hours, oauth, FakeAnthropic};
use common::advanced::Mac;

/// Accounts 2 and 3, Claude Code on 2.
fn mac(tag: &str) -> (Mac, FakeAnthropic) {
    let mac = Mac::new(tag);
    mac.add(2, "lan@team.example", "org-team", &credential("acc-2", "ref-2", in_hours(5.0)));
    mac.add(3, "lan.b@team.example", "org-team", &credential("acc-3", "ref-3", in_hours(5.0)));
    mac.log_in(
        &credential("acc-2", "ref-2", in_hours(5.0)),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    (mac, FakeAnthropic::start())
}

fn read(mac: &Mac, anthropic: &FakeAnthropic, force: bool) -> usage::Pass {
    usage::read_due(&mac.store, &anthropic.endpoints(), force, true)
}

// The data file.

#[test]
fn the_data_file_has_every_account_and_no_secret_or_email() {
    let (mac, anthropic) = mac("state");
    let mut list = mac.store.load().unwrap();
    quota_station_core::advanced::held::set(list.get_mut(3).unwrap(), true);
    mac.store.save(&list).unwrap();
    anthropic.usage("acc-2", 30.0, Some("2030-01-16T09:00:00Z"), Some(12.0), None);
    anthropic.usage("acc-3", 5.0, None, None, None);
    read(&mac, &anthropic, false);
    state::write(&mac.store, switch::active_slot(&mac.store), OffsetDateTime::now_utc()).unwrap();
    let text = std::fs::read_to_string(mac.paths.state()).unwrap();
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data["activeSlot"], 2);
    let accounts = data["accounts"].as_array().unwrap();
    assert_eq!(
        accounts.iter().map(|a| a["slot"].as_u64().unwrap()).collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(
        (accounts[0]["enabled"].as_bool(), accounts[1]["enabled"].as_bool()),
        (Some(true), Some(false))
    );
    assert_eq!(accounts[0]["status"], "fresh");
    assert_eq!(accounts[0]["windows"]["fiveHour"]["usedPercent"], 30.0);
    assert_eq!(accounts[0]["windows"]["fiveHour"]["resetsAt"], "2030-01-16T09:00:00Z");
    for secret in ["acc-2", "acc-3", "ref-2", "ref-3", "@team.example"] {
        assert!(!text.contains(secret), "the data file holds {secret}");
    }
}
