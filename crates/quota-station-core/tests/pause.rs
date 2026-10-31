// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The pause flag in a temporary folder: never the real one.

#![cfg(feature = "author-tools")]

mod common;

use common::temp_dir;
use quota_station_core::pause::{self, Flag};

#[test]
fn a_flag_with_a_future_until_holds_and_an_expired_one_does_not() {
    let flag =
        Flag::parse("until = 2026-10-16T18:30\nreason = chuyển tay sang tài khoản 3 lúc 17:30\n");
    assert_eq!(flag.until.as_deref(), Some("2026-10-16T18:30"));
    assert_eq!(flag.reason.as_deref(), Some("chuyển tay sang tài khoản 3 lúc 17:30"));
    assert!(flag.holds("2026-10-16T18:29:59"));
    assert!(!flag.holds("2026-10-16T18:30:00"));
    assert!(!flag.holds("2026-10-17T09:00:00"));
}

#[test]
fn no_until_pauses_until_the_flag_is_removed() {
    let flag = Flag::parse("reason = tạm dừng trong Quota Station lúc 14:20\n");
    assert_eq!(flag.until, None);
    assert!(flag.holds("2099-01-01T00:00:00"));
    // An until that is not a time keeps the pause, as the switcher's guard does.
    assert!(Flag::parse("until = tomorrow\n").holds("2099-01-01T00:00:00"));
}

#[test]
fn the_flag_reads_like_the_rotation_file() {
    let flag = Flag::parse(
        "# written by hand\n  until=2026-10-16T18:00  \nuntil = 2026-10-16T19:00\nnote = ignored\nreason =\n",
    );
    assert_eq!(flag.until.as_deref(), Some("2026-10-16T19:00"), "the last value wins");
    assert_eq!(flag.reason, None, "an empty value is no value");
}

#[test]
fn write_is_atomic_read_back_and_remove() {
    let home = temp_dir("pause");
    let file = pause::path(&home);
    assert_eq!(pause::read(&file).unwrap(), None);
    pause::remove(&file).unwrap();

    let flag = Flag {
        until: Some("2026-10-16T18:30".into()),
        reason: Some(pause::hold_reason(3, "17:30")),
    };
    pause::write(&file, &flag).unwrap();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "until = 2026-10-16T18:30\nreason = chuyển tay sang tài khoản 3 lúc 17:30\n"
    );
    assert_eq!(pause::read(&file).unwrap(), Some(flag));
    let dir: Vec<_> = std::fs::read_dir(file.parent().unwrap()).unwrap().flatten().collect();
    assert_eq!(dir.len(), 1, "no temporary file left behind");

    let indefinite = Flag { until: None, reason: Some(pause::pause_reason("14:20")) };
    pause::write(&file, &indefinite).unwrap();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "reason = tạm dừng trong Quota Station lúc 14:20\n"
    );

    pause::remove(&file).unwrap();
    assert!(!file.exists());
}

#[test]
fn writes_at_the_same_time_never_share_a_temporary_file() {
    let home = temp_dir("pause-race");
    let file = pause::path(&home);
    let writers: Vec<_> = (0..8)
        .map(|i| {
            let file = file.clone();
            std::thread::spawn(move || {
                (0..25).all(|_| {
                    let flag = Flag { until: Some(format!("2026-10-16T18:{i:02}")), reason: None };
                    pause::write(&file, &flag).is_ok()
                })
            })
        })
        .collect();
    assert!(writers.into_iter().all(|w| w.join().unwrap()), "every write succeeds");
    let flag = pause::read(&file).unwrap().expect("one whole flag");
    assert!(flag.until.is_some_and(|u| u.starts_with("2026-10-16T18:0")));
    let dir: Vec<_> = std::fs::read_dir(file.parent().unwrap()).unwrap().flatten().collect();
    assert_eq!(dir.len(), 1, "no temporary file left behind");
}

#[test]
fn the_flag_lives_next_to_the_switcher_rotation_file() {
    let home = std::path::Path::new("/home/someone");
    assert_eq!(pause::path(home), home.join(".config/cswap-prefer-team/pause"));
}
