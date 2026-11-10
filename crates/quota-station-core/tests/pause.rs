// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The pause flag in a temporary folder: never the real one.

mod common;

use std::path::Path;

use common::temp_dir;
use quota_station_core::claude::rotate;
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
fn the_flag_lives_in_the_app_folder() {
    let home = std::path::Path::new("/home/someone");
    let app = quota_station_core::claude::app_dir(home);
    assert_eq!(pause::path(home), app.join("claude").join("pause"));
    assert_eq!(pause::old_path(home), home.join(".config/cswap-prefer-team/pause"));
}

/// A flag as one written before the move might hold: a comment, a key the
/// app does not read and no final line break, carried over byte for byte.
const BEFORE: &str =
    "# by hand\nuntil = 2099-01-01T08:00\nreason = chuyển tay sang tài khoản 2 lúc 07:00\nnote = x";

fn write_old(home: &Path, text: &str) {
    let old = pause::old_path(home);
    std::fs::create_dir_all(old.parent().unwrap()).unwrap();
    std::fs::write(&old, text).unwrap();
}

/// What the app's folder for the Claude accounts holds, by name.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn an_old_flag_is_carried_over_once() {
    let home = temp_dir("pause-old");
    write_old(&home, BEFORE);
    let new = pause::path(&home);
    assert_eq!(pause::file(&home), new);
    assert_eq!(std::fs::read_to_string(&new).unwrap(), BEFORE);
    assert!(!pause::old_path(&home).exists(), "the old file is removed");
    assert_eq!(names(new.parent().unwrap()), ["pause"], "no temporary file left behind");
    // The rotation file beside it is not touched.
    assert!(pause::old_path(&home).parent().unwrap().is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(new.parent().unwrap()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "the folder made for it is the owner's only");
    }

    // Again: nothing to carry, the flag stays as it is.
    assert_eq!(pause::file(&home), new);
    assert_eq!(std::fs::read_to_string(&new).unwrap(), BEFORE);
    // Resumed: the old pause never comes back.
    pause::remove(&new).unwrap();
    assert_eq!(pause::read(&pause::file(&home)).unwrap(), None);
    assert!(!new.exists());
}

#[test]
fn a_flag_in_the_app_folder_is_used_as_it_is() {
    let home = temp_dir("pause-new");
    let flag = Flag { until: None, reason: Some(pause::pause_reason("09:15")) };
    pause::write(&pause::path(&home), &flag).unwrap();
    assert_eq!(pause::file(&home), pause::path(&home));
    assert_eq!(pause::read(&pause::file(&home)).unwrap(), Some(flag));
    let old = pause::old_path(&home);
    assert!(!old.parent().unwrap().exists(), "the old folder is not made");
}

#[test]
fn with_both_the_flag_in_the_app_folder_wins_and_the_old_one_goes() {
    let home = temp_dir("pause-both");
    let newer = Flag { until: Some("2099-01-01T09:00".into()), reason: None };
    pause::write(&pause::path(&home), &newer).unwrap();
    write_old(&home, BEFORE);
    assert_eq!(pause::file(&home), pause::path(&home));
    assert_eq!(pause::read(&pause::path(&home)).unwrap(), Some(newer));
    assert!(!pause::old_path(&home).exists());
}

#[test]
fn with_neither_there_is_no_pause_and_nothing_is_made() {
    let home = temp_dir("pause-none");
    let file = pause::file(&home);
    assert_eq!(file, pause::path(&home));
    assert_eq!(pause::read(&file).unwrap(), None);
    assert_eq!(names(&home), Vec::<String>::new(), "no folder made by looking");
}

#[test]
fn a_carry_over_that_cannot_be_made_keeps_the_old_flag_and_tries_again() {
    let home = temp_dir("pause-blocked");
    write_old(&home, BEFORE);
    // The app's folder for the Claude accounts cannot be made: a file is there.
    let dir = pause::path(&home).parent().unwrap().to_path_buf();
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    std::fs::write(&dir, "").unwrap();

    let file = pause::file(&home);
    assert_eq!(file, pause::old_path(&home), "the old flag is used meanwhile");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), BEFORE);
    assert!(pause::read(&file).unwrap().unwrap().holds("2026-11-04T10:00:00"));

    // Once the folder can be made, the next read carries it over.
    std::fs::remove_file(&dir).unwrap();
    assert_eq!(pause::file(&home), pause::path(&home));
    assert_eq!(std::fs::read_to_string(pause::path(&home)).unwrap(), BEFORE);
    assert!(!pause::old_path(&home).exists());
}

/// The old file cannot be removed (its folder is read-only): nothing is
/// carried, so the pause is never in two places to be applied twice.
#[cfg(unix)]
#[test]
fn an_old_flag_that_cannot_be_removed_stays_the_only_one() {
    use std::os::unix::fs::PermissionsExt;
    let home = temp_dir("pause-locked");
    write_old(&home, BEFORE);
    let old_dir = pause::old_path(&home).parent().unwrap().to_path_buf();
    std::fs::set_permissions(&old_dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    // A user the folder's mode does not stop (root) has nothing to test here.
    let probe = old_dir.join("probe");
    if std::fs::write(&probe, "").is_ok() {
        std::fs::set_permissions(&old_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    let file = pause::file(&home);
    std::fs::set_permissions(&old_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(file, pause::old_path(&home));
    assert!(!pause::path(&home).exists(), "no second flag");
    assert_eq!(
        names(pause::path(&home).parent().unwrap()),
        Vec::<String>::new(),
        "no temporary file"
    );

    assert_eq!(pause::file(&home), pause::path(&home), "tried again once it can be");
    assert_eq!(std::fs::read_to_string(pause::path(&home)).unwrap(), BEFORE);
    assert!(!pause::old_path(&home).exists());
}

/// A pause set before the upgrade, at the old place, still stops the
/// rotation after it, and its end is applied in the app's folder.
#[test]
fn a_pause_set_before_the_upgrade_still_holds_after_it() {
    let home = temp_dir("pause-upgrade");
    write_old(&home, BEFORE);
    let now = time::OffsetDateTime::now_utc();
    assert_eq!(
        rotate::guard(&pause::file(&home), now),
        rotate::Guard::Paused(
            "paused until 2099-01-01T08:00: chuyển tay sang tài khoản 2 lúc 07:00".into()
        )
    );
    assert!(pause::path(&home).exists() && !pause::old_path(&home).exists());

    // An ended pause at the old place ends at the new one.
    let home = temp_dir("pause-upgrade-ended");
    write_old(&home, "until = 2000-01-01T00:00\n");
    let rotate::Guard::Ended { until, .. } = rotate::guard(&pause::file(&home), now) else {
        panic!("the pause has ended");
    };
    assert_eq!(until, "2000-01-01T00:00");
}
