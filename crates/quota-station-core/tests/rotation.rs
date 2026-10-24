//! The rotation file in temporary folders, checked by a fake quota-axi that
//! prints a fixture: these tests never touch the real rotation file and never
//! run the real quota-axi.

mod common;

use std::path::{Path, PathBuf};

use common::{fixture, fixture_path, script, temp_dir};
use quota_station_core::rotation::{
    self, adopt, apply, backups, load, parse, render, verdict, ApplyError, Expire, FileState,
    Rules, Stamp, Verdict, BACKUPS_KEPT,
};
use time::OffsetDateTime;

const FILE: &str = "\
# Shared by the Macs.
team = 2 3 4
personal = 1
personal_hours = 8-23
full = 90
fable_full = 95
expire = 2=2026-01-16T17:45
";

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap() // 2026-01-16T07:20Z
}

fn stamp(machine: &str) -> Stamp {
    Stamp::new(now(), machine)
}

#[test]
fn the_six_keys_as_quota_axi_reads_them() {
    let p = parse(FILE).unwrap();
    assert_eq!(
        p.rules,
        Rules {
            team: vec![2, 3, 4],
            personal: Some(1),
            personal_from: 8,
            personal_to: 23,
            full: 90.0,
            fable_full: 95.0,
            expire: vec![Expire { slot: 2, at: "2026-01-16T17:45".into() }],
        }
    );
    assert_eq!(p.stamp, None);
    assert!(p.ignored_keys.is_empty());

    // The last value wins, other keys are kept aside, `personal =` is none.
    let p = parse(&format!("{FILE}full = 85\nnote = x\npersonal =\nexpire =\n")).unwrap();
    assert_eq!(p.rules.full, 85.0);
    assert_eq!(p.rules.personal, None);
    assert!(p.rules.expire.is_empty());
    assert_eq!(p.ignored_keys, vec!["note".to_string()]);
}

#[test]
fn every_reason_quota_axi_gives() {
    let with = |key: &str, value: &str| {
        FILE.lines()
            .map(|l| {
                if l.starts_with(&format!("{key} =")) {
                    format!("{key} = {value}")
                } else {
                    l.into()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let cases = [
        (FILE.replace("full = 90\n", ""), "rotation_missing_key:full"),
        (format!("{FILE}not a key line\n"), "rotation_malformed_line"),
        (format!("{FILE}Team = 2\n"), "rotation_malformed_line"),
        (with("team", ""), "rotation_bad_value:team"),
        (with("team", "2 2"), "rotation_bad_value:team"),
        (with("team", "0 3"), "rotation_bad_value:team"),
        (with("personal", "one"), "rotation_bad_value:personal"),
        (with("personal_hours", "8-25"), "rotation_bad_value:personal_hours"),
        (with("personal_hours", "20-8"), "rotation_bad_value:personal_hours"),
        (with("personal_hours", "8 - 23"), "rotation_bad_value:personal_hours"),
        (with("full", "0"), "rotation_bad_value:full"),
        (with("full", "101"), "rotation_bad_value:full"),
        (with("fable_full", "high"), "rotation_bad_value:fable_full"),
        (with("expire", "2=tomorrow"), "rotation_bad_value:expire"),
        (with("expire", "2"), "rotation_bad_value:expire"),
    ];
    for (text, reason) in cases {
        assert_eq!(parse(&text).unwrap_err(), reason, "{text}");
    }
}

#[test]
fn rules_are_checked_before_writing() {
    assert_eq!(Rules::defaults().check(), Ok(()));
    let mut r = Rules::defaults();
    r.personal = Some(3);
    assert_eq!(r.check().unwrap_err(), "rotation_personal_in_team");
    let mut r = Rules::defaults();
    r.personal_to = 24;
    assert_eq!(r.check().unwrap_err(), "rotation_bad_value:personal_hours");
    let mut r = Rules::defaults();
    r.expire = vec![Expire { slot: 2, at: "2026-01-16 17:45".into() }];
    assert_eq!(r.check().unwrap_err(), "rotation_bad_value:expire");
    let mut r = Rules::defaults();
    r.team.clear();
    assert_eq!(r.check().unwrap_err(), "rotation_bad_value:team");
}

#[test]
fn a_rewrite_keeps_every_other_line_and_one_stamp_at_the_top() {
    let before =
        format!("# updated = 2026-01-15T10:00:00+07:00 by Mac B\n{FILE}note = keep me\n\n# end\n");
    let mut rules = parse(&before).unwrap().rules;
    rules.full = 85.0;
    rules.expire.push(Expire { slot: 3, at: "2026-01-17T09:00".into() });
    let text = render(&rules, &stamp("Mac A"), Some(&before));
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("# updated = 2026-01-16T"), "{text}");
    assert!(lines[0].ends_with(" by Mac A"));
    assert_eq!(text.matches("# updated").count(), 1);
    assert_eq!(lines[1], "# Shared by the Macs.");
    assert!(text.contains("full = 85\n"));
    assert!(text.contains("expire = 2=2026-01-16T17:45 3=2026-01-17T09:00\n"));
    assert!(text.contains("note = keep me\n\n# end\n"));

    let back = parse(&text).unwrap();
    assert_eq!(back.rules, rules);
    assert_eq!(back.stamp, Some(stamp("Mac A")));
    assert_eq!(back.ignored_keys, vec!["note".to_string()]);

    // A new file holds the six keys.
    let fresh = render(&Rules::defaults(), &stamp("Mac A"), None);
    assert_eq!(parse(&fresh).unwrap().rules, Rules::defaults());
    assert!(fresh.contains("personal_hours = 8-23\nfull = 90\nfable_full = 95\nexpire = \n"));
}

#[test]
fn a_stamp_written_later_is_newer() {
    let earlier = Stamp::new(now() - time::Duration::minutes(9), "Mac B");
    let later = stamp("Mac A");
    assert!(later.newer_than(Some(&earlier)));
    assert!(!earlier.newer_than(Some(&later)));
    assert!(!later.newer_than(Some(&later)));
    assert!(later.newer_than(None));
    let broken = Stamp { at: "yesterday".into(), machine: "Mac C".into() };
    assert!(!broken.newer_than(Some(&earlier)));
    assert!(earlier.newer_than(Some(&broken)));
    // The offset is this Mac's, and the time reads back as the same instant.
    assert_eq!(later.instant(), Some(now()));
}

/// A rotation file in a temporary home, and a fake quota-axi that refuses
/// any file holding a `refuse_me` line and accepts the rest.
struct Setup {
    dir: PathBuf,
    file: PathBuf,
    quota_axi: PathBuf,
}

fn setup(tag: &str) -> Setup {
    let dir = temp_dir(tag);
    let file = rotation::path(&dir.join("home"));
    let accepted = fixture_path("claude-pool-schema5.json");
    let refused = fixture_path("claude-rotation-refused.json");
    let quota_axi = script(
        &dir.join("bin/quota-axi"),
        &format!(
            "if grep -q '^refuse_me' '{}'; then cat '{}'; else cat '{}'; fi\n",
            file.display(),
            refused.display(),
            accepted.display()
        ),
    );
    Setup { dir, file, quota_axi }
}

fn base(file: &Path) -> Option<String> {
    load(file).version()
}

#[test]
fn a_save_is_written_whole_kept_as_a_backup_and_checked() {
    let s = setup("rot-save");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, FILE).unwrap();
    let mut rules = parse(FILE).unwrap().rules;
    rules.full = 85.0;
    let text = render(&rules, &stamp("Mac A"), Some(FILE));

    let applied =
        apply(&s.file, Some(&s.quota_axi), &[], &text, base(&s.file).as_deref(), now()).unwrap();
    assert_eq!(std::fs::read_to_string(&s.file).unwrap(), text);
    assert_eq!(applied.parsed.rules, rules);
    let backup = applied.backup.unwrap();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), FILE);
    assert!(backup.file_name().unwrap().to_string_lossy().starts_with("rotation.bak-2026"));
    let dir = s.file.parent().unwrap();
    let leftovers: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no temporary file is left");
    assert_eq!(verdict(&applied.report), Verdict::Accepted);
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_file_quota_axi_refuses_is_rolled_back_to_the_backup() {
    let s = setup("rot-refused");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, FILE).unwrap();
    // A key this app does not know and keeps; this quota-axi refuses it.
    let text = format!(
        "{}refuse_me = 1\n",
        render(&parse(FILE).unwrap().rules, &stamp("Mac A"), Some(FILE))
    );

    let err = apply(&s.file, Some(&s.quota_axi), &[], &text, base(&s.file).as_deref(), now())
        .unwrap_err();
    assert_eq!(
        err,
        ApplyError::Refused { reason: "rotation_bad_value:personal_hours".into(), restored: true }
    );
    assert_eq!(std::fs::read_to_string(&s.file).unwrap(), FILE, "the file before is back");
    assert_eq!(backups(s.file.parent().unwrap()).len(), 1, "and its backup is kept");

    // With no file before, a refused file is removed again.
    std::fs::remove_file(&s.file).unwrap();
    let err = apply(&s.file, Some(&s.quota_axi), &[], &text, None, now()).unwrap_err();
    assert_eq!(err.kind(), "refused");
    assert_eq!(load(&s.file), FileState::Missing);
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_file_quota_axi_cannot_check_is_rolled_back_too() {
    let s = setup("rot-unchecked");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, FILE).unwrap();
    let text = render(&parse(FILE).unwrap().rules, &stamp("Mac A"), Some(FILE));

    // No quota-axi at all.
    let err = apply(&s.file, None, &[], &text, base(&s.file).as_deref(), now()).unwrap_err();
    assert!(matches!(err, ApplyError::Unchecked { restored: true, .. }), "{err:?}");
    assert_eq!(std::fs::read_to_string(&s.file).unwrap(), FILE);

    // A quota-axi without the rotation block (not the +cswap fork).
    let plain = script(
        &s.dir.join("bin/plain-quota-axi"),
        &format!("cat '{}'\n", fixture_path("claude-no-rotation.json").display()),
    );
    let err =
        apply(&s.file, Some(&plain), &[], &text, base(&s.file).as_deref(), now()).unwrap_err();
    assert!(matches!(err, ApplyError::Unchecked { restored: true, .. }), "{err:?}");
    assert_eq!(std::fs::read_to_string(&s.file).unwrap(), FILE);
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_file_changed_since_the_editor_read_it_is_not_overwritten() {
    let s = setup("rot-changed");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, FILE).unwrap();
    let started = base(&s.file);
    // Another Mac's copy lands meanwhile.
    let theirs = render(&parse(FILE).unwrap().rules, &stamp("Mac B"), Some(FILE));
    std::fs::write(&s.file, &theirs).unwrap();

    let mine = render(&Rules::defaults(), &stamp("Mac A"), Some(FILE));
    let err =
        apply(&s.file, Some(&s.quota_axi), &[], &mine, started.as_deref(), now()).unwrap_err();
    assert_eq!(err, ApplyError::Changed);
    assert_eq!(std::fs::read_to_string(&s.file).unwrap(), theirs);

    // Text quota-axi would refuse is never written.
    let err =
        apply(&s.file, Some(&s.quota_axi), &[], "team = 2\n", base(&s.file).as_deref(), now())
            .unwrap_err();
    assert_eq!(err, ApplyError::Invalid("rotation_missing_key:personal".into()));
    assert!(backups(s.file.parent().unwrap()).is_empty());
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn only_the_newest_backups_stay() {
    let s = setup("rot-prune");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, FILE).unwrap();
    for i in 0..BACKUPS_KEPT + 3 {
        let at = now() + time::Duration::seconds(i as i64);
        rotation::write(&s.file, &format!("{FILE}# {i}\n"), at).unwrap();
    }
    let kept = backups(s.file.parent().unwrap());
    assert_eq!(kept.len(), BACKUPS_KEPT);
    // The newest backup holds the file from before the last write.
    assert!(std::fs::read_to_string(&kept[0])
        .unwrap()
        .ends_with(&format!("# {}\n", BACKUPS_KEPT + 1)));
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn quota_axis_verdict_on_a_report() {
    assert_eq!(verdict(&fixture("claude-pool-schema5.json")), Verdict::Accepted);
    assert_eq!(
        verdict(&fixture("claude-rotation-refused.json")),
        Verdict::Refused("rotation_bad_value:personal_hours".into())
    );
    assert_eq!(verdict(&fixture("claude-no-rotation.json")), Verdict::NotRead);
}

#[test]
fn a_rewrite_drops_lines_quota_axi_cannot_read_and_the_backup_keeps_them() {
    let s = setup("rot-drop");
    let before = format!("{FILE}not a key line\nTeam = 2\nnote = kept\n");
    assert_eq!(parse(&before).unwrap_err(), "rotation_malformed_line");
    let text = render(&Rules::defaults(), &stamp("Mac A"), Some(&before));
    assert!(!text.contains("not a key line") && !text.contains("Team = 2"), "{text}");
    assert!(text.contains("note = kept\n"));
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, &before).unwrap();
    let applied =
        apply(&s.file, Some(&s.quota_axi), &[], &text, base(&s.file).as_deref(), now()).unwrap();
    assert_eq!(std::fs::read_to_string(applied.backup.unwrap()).unwrap(), before);
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn a_personal_slot_that_is_also_team_is_never_written_and_reads_as_invalid() {
    let s = setup("rot-overlap");
    let overlap = FILE.replace("personal = 1", "personal = 3");
    std::fs::create_dir_all(s.file.parent().unwrap()).unwrap();
    std::fs::write(&s.file, &overlap).unwrap();
    // On disk it is invalid, so the editor starts from the defaults.
    assert!(
        matches!(load(&s.file), FileState::Invalid { ref reason, .. } if reason == "rotation_personal_in_team")
    );
    // A copy taken from another Mac with the same overlap is refused before writing.
    let err = apply(&s.file, Some(&s.quota_axi), &[], &overlap, base(&s.file).as_deref(), now())
        .unwrap_err();
    assert_eq!(err, ApplyError::Invalid("rotation_personal_in_team".into()));
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn an_adopted_copy_is_stamped_again_later_than_both_copies() {
    let mine = stamp("Mac A");
    let theirs = format!("{FILE}note = from Mac B\n");

    // No `updated` line there: the taken copy carries one, so it is sent on.
    let taken = adopt(&theirs, Some(&mine), now(), "Mac A").unwrap();
    let p = parse(&taken).unwrap();
    assert_eq!(p.rules, parse(FILE).unwrap().rules);
    assert!(taken.contains("note = from Mac B"));
    let new = p.stamp.unwrap();
    assert_eq!(new.machine, "Mac A");
    assert!(new.newer_than(Some(&mine)));

    // This Mac's own stamp there (edited by hand after a copy): newer than it too.
    let same =
        render(&parse(FILE).unwrap().rules, &mine, Some(FILE)).replace("full = 90", "full = 88");
    let taken = parse(&adopt(&same, Some(&mine), now(), "Mac A").unwrap()).unwrap();
    assert_eq!(taken.rules.full, 88.0);
    assert!(taken.stamp.unwrap().newer_than(Some(&mine)));

    // A newer stamp from a clock ahead: still later than that one.
    let ahead = Stamp::new(now() + time::Duration::minutes(5), "Mac B");
    let later = render(&parse(FILE).unwrap().rules, &ahead, Some(FILE));
    let taken = parse(&adopt(&later, Some(&mine), now(), "Mac A").unwrap()).unwrap();
    assert!(taken.stamp.unwrap().newer_than(Some(&ahead)));

    // A copy that breaks the rules is not taken.
    let bad = FILE.replace("personal = 1", "personal = 2");
    assert_eq!(adopt(&bad, Some(&mine), now(), "Mac A").unwrap_err(), "rotation_personal_in_team");
}
