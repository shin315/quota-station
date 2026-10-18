//! The pause flag in a temporary folder: never the real one.

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
fn the_flag_lives_next_to_the_switcher_rotation_file() {
    let home = std::path::Path::new("/home/someone");
    assert_eq!(pause::path(home), home.join(".config/cswap-prefer-team/pause"));
}
