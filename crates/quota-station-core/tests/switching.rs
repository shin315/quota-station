// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! A manual switch end to end, with the app's own switch against a fake
//! Keychain and launchd in a temporary home: these tests never touch the
//! Keychain, Claude Code's login or the real pause flag.

mod common;

use quota_station_core::claude::{rotate, switch};
use quota_station_core::switching::{switch_claude, Outcome, Tools};
use quota_station_core::{local_time, pause};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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
    let tools = Tools {
        store: mac.store.clone(),
        launchctl: mac.launchctl.clone(),
        pause_file: pause::path(&mac.home),
    };
    (mac, tools)
}

fn quiet() {}

#[test]
fn a_confirmed_switch_with_a_hold_writes_the_pause_flag() {
    let (mac, tools) = mac("switching");
    let until = OffsetDateTime::now_utc() + time::Duration::hours(1);

    let done = switch_claude(&tools, 3, Some(until), &quiet);

    let Outcome::Switched { slot: 3, hold: Some(hold) } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(!hold.failed);
    assert_eq!(mac.live_email().as_deref(), Some("lan.b@team.example"));
    let flag = pause::read(&tools.pause_file).unwrap().expect("flag written");
    let written = flag.until.clone().unwrap();
    assert_eq!(written, local_time::minute(until));
    assert!(flag.reason.as_deref().unwrap().starts_with("chuyển tay sang tài khoản 3 lúc "));
    assert!(flag.holds(&local_time::stamp(OffsetDateTime::now_utc())));
    let shown = OffsetDateTime::parse(&hold.until, &Rfc3339).unwrap();
    assert_eq!(local_time::instant(&written), Some(shown));
    // The report read after comes back to show at once, from the app's
    // own accounts.
    let report = done.report.expect("report");
    let accounts = report.primary("claude").unwrap().claude_accounts.as_ref().unwrap();
    assert_eq!(accounts.active_slot, Some(3));
}

#[cfg(feature = "author-tools")]
#[test]
fn the_switch_is_in_the_files_firstmate_reads() {
    let (mac, tools) = mac("switching-state");
    mac.show_firstmate(true);
    switch_claude(&tools, 3, None, &quiet);
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(mac.paths.state()).unwrap()).unwrap();
    assert_eq!(state["activeSlot"], 3);
    let sequence: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(mac.paths.swap().join("sequence.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(sequence["activeAccountNumber"], 3);
}

#[test]
fn without_firstmate_no_copy_is_written() {
    let (mac, tools) = mac("switching-no-firstmate");
    mac.show_firstmate(false);
    switch_claude(&tools, 3, None, &quiet);
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(mac.paths.state()).unwrap()).unwrap();
    assert_eq!(state["activeSlot"], 3);
    assert!(!mac.paths.state().with_file_name("swap").exists());
    #[cfg(feature = "author-tools")]
    assert!(mac.paths.firstmate_env().is_empty());
}

/// A build without the author's tools writes the data file only: no copy
/// in claude-swap's layout, even with Nâng cao and Firstmate on in a
/// settings file a build with them wrote.
#[cfg(not(feature = "author-tools"))]
#[test]
fn without_the_author_tools_no_copy_is_written_whatever_the_settings_say() {
    let (mac, tools) = mac("switching-no-author-tools");
    mac.show_firstmate(true);
    switch_claude(&tools, 3, None, &quiet);
    let state: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(mac.paths.state()).unwrap()).unwrap();
    assert_eq!(state["activeSlot"], 3);
    assert!(!mac.paths.state().with_file_name("swap").exists());
}

#[test]
fn no_hold_leaves_the_rotation_alone() {
    let (_, tools) = mac("switching-nohold");
    let done = switch_claude(&tools, 3, None, &quiet);
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert!(!tools.pause_file.exists());
}

#[test]
fn a_hold_already_over_is_no_hold() {
    let (_, tools) = mac("switching-past");
    let past = OffsetDateTime::now_utc() - time::Duration::minutes(1);
    let done = switch_claude(&tools, 3, Some(past), &quiet);
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert!(!tools.pause_file.exists());
}

#[test]
fn a_switch_that_does_not_land_changes_nothing() {
    let (mac, tools) = mac("switching-busy");
    std::fs::create_dir(mac.home.join(".claude.lock")).unwrap();
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);
    let done = switch_claude(&tools, 3, Some(until), &quiet);
    let Outcome::Failed { slot: 3, error, detail, active } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert_eq!(error, "busy");
    assert_eq!(detail, "", "no path from this Mac reaches the UI");
    assert_eq!(*active, None);
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
    assert!(!tools.pause_file.exists(), "no hold after a failed switch");
}

#[test]
fn nothing_switches_while_the_old_switcher_is_loaded() {
    let (mac, tools) = mac("switching-legacy");
    mac.load_label("com.cswap.prefer-team");
    let done = switch_claude(&tools, 3, None, &quiet);
    assert_eq!(
        done.outcome,
        Outcome::Failed {
            slot: 3,
            error: "legacy".into(),
            detail: "com.cswap.prefer-team".into(),
            active: None
        }
    );
    assert_eq!(mac.live_email().as_deref(), Some("lan@team.example"));
}

#[test]
fn a_hold_never_shortens_a_longer_pause() {
    let (_, tools) = mac("switching-longer");
    let hour = OffsetDateTime::now_utc() + time::Duration::hours(1);

    // Paused until the owner turns it back on: a one-hour hold leaves it be.
    let open = pause::Flag {
        until: None,
        reason: Some("tạm dừng trong Quota Station lúc 14:20".into()),
    };
    pause::write(&tools.pause_file, &open).unwrap();
    let done = switch_claude(&tools, 3, Some(hour), &quiet);
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert_eq!(pause::read(&tools.pause_file).unwrap(), Some(open));

    // Paused for longer than the hold: kept too.
    let later = pause::Flag {
        until: Some(local_time::minute(hour + time::Duration::hours(2))),
        reason: Some("tạm dừng trong Quota Station lúc 14:20".into()),
    };
    pause::write(&tools.pause_file, &later).unwrap();
    let done = switch_claude(&tools, 4, Some(hour), &quiet);
    assert_eq!(done.outcome, Outcome::Switched { slot: 4, hold: None });
    assert_eq!(pause::read(&tools.pause_file).unwrap(), Some(later));

    // A shorter pause gives way to the hold.
    let sooner = pause::Flag {
        until: Some(local_time::minute(OffsetDateTime::now_utc() + time::Duration::minutes(10))),
        reason: None,
    };
    pause::write(&tools.pause_file, &sooner).unwrap();
    let done = switch_claude(&tools, 3, Some(hour), &quiet);
    assert!(matches!(&done.outcome, Outcome::Switched { hold: Some(h), .. } if !h.failed));
    let flag = pause::read(&tools.pause_file).unwrap().unwrap();
    assert_eq!(flag.until, Some(local_time::minute(hour)));
}

#[test]
fn a_hold_that_cannot_be_written_is_reported() {
    let (mac, tools) = mac("switching-blocked");
    // The flag's folder is a file, so the flag cannot be written.
    std::fs::write(mac.home.join("blocked"), "").unwrap();
    let tools = Tools { pause_file: mac.home.join("blocked/pause"), ..tools };
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);
    let done = switch_claude(&tools, 3, Some(until), &quiet);
    let Outcome::Switched { slot: 3, hold: Some(hold) } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(hold.failed, "the switch stands, and the UI says the rotation is not held");
}

#[test]
fn the_rotation_switching_back_is_reported_as_raced() {
    let (mac, tools) = mac("switching-raced");
    let rotation = || {
        switch::switch(&mac.store, 2).unwrap();
        rotate::log(
            &mac.store,
            "switch to slot 2 (ends for good at 17:45, use it first)",
            OffsetDateTime::now_utc(),
        );
    };
    let done = switch_claude(&tools, 3, None, &rotation);
    let Outcome::Raced { slot: 3, active: 2, by_switcher_at, .. } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(by_switcher_at.is_some(), "the rotation's log names the switch");
}

#[test]
fn another_switch_winning_is_raced_without_the_rotation() {
    let (mac, tools) = mac("switching-other");
    let other = || {
        switch::switch(&mac.store, 4).unwrap();
    };
    let done = switch_claude(&tools, 3, None, &other);
    assert!(matches!(
        done.outcome,
        Outcome::Raced { slot: 3, active: 4, by_switcher_at: None, .. }
    ));
}

#[test]
fn a_login_changed_to_one_the_app_does_not_keep_is_unconfirmed() {
    let (mac, tools) = mac("switching-unconfirmed");
    let elsewhere = || {
        mac.log_in(
            &credential("acc-x", "ref-x", in_hours(5.0)),
            &oauth("x@else.example", "uuid-x", "org-x"),
        )
    };
    let done = switch_claude(&tools, 3, None, &elsewhere);
    assert!(matches!(done.outcome, Outcome::Unconfirmed { slot: 3, hold: None }));
}
