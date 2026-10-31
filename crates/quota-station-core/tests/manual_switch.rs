// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A manual switch end to end, with the app's own switch against a fake
//! Keychain in a temporary home: these tests never touch the Keychain or
//! Claude Code's login.

#![cfg(not(feature = "author-tools"))]

mod common;

use quota_station_core::claude::switch;
use quota_station_core::switching::{switch_claude, Outcome, Tools};

use common::accounts::{credential, in_hours, oauth, Mac};

/// Accounts 2, 3 and 4, Claude Code on 2.
fn mac(tag: &str) -> (Mac, Tools) {
    let mac = Mac::new(tag);
    for (slot, email) in
        [(2, "lan@team.example"), (3, "lan.b@team.example"), (4, "lan.c@team.example")]
    {
        let login = credential(&format!("acc-{slot}"), &format!("ref-{slot}"), in_hours(5.0));
        mac.add(slot, email, "org-team", &login);
    }
    mac.log_in(
        &mac.store.credential(2).unwrap().unwrap(),
        &oauth("lan@team.example", "uuid-2", "org-team"),
    );
    let tools = Tools { store: mac.store.clone() };
    (mac, tools)
}

fn quiet() {}

#[test]
fn a_confirmed_switch_is_reported_with_the_new_numbers() {
    let (mac, tools) = mac("manual-switch");
    let done = switch_claude(&tools, 3, &quiet);
    assert_eq!(done.outcome, Outcome::Switched { slot: 3 });
    assert_eq!(mac.live_email().as_deref(), Some("lan.b@team.example"));
    // The report read after comes back to show at once, from the app's
    // own accounts.
    let report = done.report.expect("report");
    let accounts = report.primary("claude").unwrap().claude_accounts.as_ref().unwrap();
    assert_eq!(accounts.active_slot, Some(3));
}

#[test]
fn the_switch_is_in_the_data_file_and_nothing_else_is_written() {
    let (mac, tools) = mac("manual-switch-state");
    switch_claude(&tools, 3, &quiet);
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(mac.paths.state()).unwrap()).unwrap();
    assert_eq!(state["activeSlot"], 3);
    assert!(!mac.paths.state().with_file_name("swap").exists());
}

#[test]
fn a_switch_that_does_not_land_changes_nothing() {
    let (mac, tools) = mac("manual-switch-busy");
    std::fs::create_dir(mac.home.join(".claude.lock")).unwrap();
    let done = switch_claude(&tools, 3, &quiet);
    let Outcome::Failed { slot: 3, error, detail, active } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert_eq!(error, "busy");
    assert_eq!(detail, "", "no path from this Mac reaches the UI");
    assert_eq!(*active, None);
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
}

#[test]
fn another_switch_winning_is_raced() {
    let (mac, tools) = mac("manual-switch-other");
    let other = || {
        switch::switch(&mac.store, 4).unwrap();
    };
    let done = switch_claude(&tools, 3, &other);
    assert_eq!(done.outcome, Outcome::Raced { slot: 3, active: 4 });
}

#[test]
fn a_login_changed_to_one_the_app_does_not_keep_is_unconfirmed() {
    let (mac, tools) = mac("manual-switch-unconfirmed");
    let elsewhere = || {
        mac.log_in(
            &credential("acc-x", "ref-x", in_hours(5.0)),
            &oauth("x@else.example", "uuid-x", "org-x"),
        )
    };
    let done = switch_claude(&tools, 3, &elsewhere);
    assert_eq!(done.outcome, Outcome::Unconfirmed { slot: 3 });
}
