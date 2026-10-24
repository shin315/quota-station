// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Copying the rotation file to the other Macs, through a fake `ssh` found on
//! a `PATH` that puts it first. The fake runs the command in a temporary
//! "home" per alias, so these tests never reach a real host and never touch
//! a real rotation file. Alias `down` cannot be reached, and `liar` writes the
//! file but reads back something else.

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use common::{script, temp_dir};
use quota_station_core::peers::{
    self, due, find_ssh, plan, push, PeerRecord, PeerState, Plan, SyncLog, RETRY_MINUTES,
};
use quota_station_core::rotation::{self, parse, render, Stamp};
use time::OffsetDateTime;

fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap() // 2026-01-16T07:20Z
}

struct Net {
    dir: PathBuf,
    ssh: PathBuf,
}

impl Net {
    fn new(tag: &str) -> Net {
        let dir = temp_dir(tag);
        let root = dir.join("macs");
        std::fs::create_dir_all(&root).unwrap();
        script(
            &dir.join("bin/ssh"),
            &format!(
                r#"root='{root}'
while [ $# -gt 0 ]; do
  case "$1" in
    -o) shift 2 ;;
    -T) shift ;;
    *) break ;;
  esac
done
alias=$1; shift
echo "$alias $*" >> "$root/calls"
if [ "$alias" = down ]; then
  echo "ssh: connect to host down port 22: Operation timed out" >&2
  exit 255
fi
mkdir -p "$root/$alias"
if [ "$alias" = liar ] && case "$*" in *"mv -f"*) true;; *) false;; esac; then
  HOME="$root/$alias" /bin/sh -c "$*" | sed 's/^full = /full = 1/'
  exit 0
fi
HOME="$root/$alias" exec /bin/sh -c "$*"
"#,
                root = root.display()
            ),
        );
        // The fake comes first on PATH, as it would for the app.
        let path = std::env::join_paths([
            dir.join("bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap();
        let ssh = find_ssh(Some(&path)).unwrap();
        assert_eq!(ssh, dir.join("bin/ssh"));
        Net { dir, ssh }
    }

    fn file(&self, alias: &str) -> PathBuf {
        rotation::path(&self.dir.join("macs").join(alias))
    }

    fn put(&self, alias: &str, text: &str) {
        let f = self.file(alias);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, text).unwrap();
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("macs/calls"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

impl Drop for Net {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn mine(at: OffsetDateTime) -> (String, Stamp) {
    let stamp = Stamp::new(at, "Mac A");
    (render(&common::sample_rules(), &stamp, None), stamp)
}

fn backups(f: &Path) -> usize {
    rotation::backups(f.parent().unwrap()).len()
}

#[test]
fn a_mac_without_the_file_gets_it_and_reads_it_back() {
    let net = Net::new("peer-new");
    let (text, stamp) = mine(now());
    assert_eq!(peers::read(&net.ssh, "mac-b").unwrap(), None);
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(r.against, rotation::version(&text));
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), text);
    assert_eq!(backups(&net.file("mac-b")), 0);
    // Each call names the alias and runs a fixed script under /bin/sh.
    assert!(net.calls().iter().all(|c| c.starts_with("mac-b /bin/sh -c ")));
}

#[test]
fn a_mac_where_the_app_keeps_the_accounts_gets_the_apps_own_file() {
    let net = Net::new("peer-own");
    let home = net.dir.join("macs/mac-b");
    let own = quota_station_core::claude::Paths::for_home(&home);
    std::fs::create_dir_all(&own.dir).unwrap();
    std::fs::write(own.accounts(), "{\"version\":1,\"accounts\":[]}").unwrap();
    let (text, stamp) = mine(now());
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(std::fs::read_to_string(own.rotation()).unwrap(), text);
    assert!(!net.file("mac-b").exists(), "the old switcher's file is left alone");
    assert_eq!(peers::read(&net.ssh, "mac-b").unwrap().as_deref(), Some(text.as_str()));
}

#[test]
fn an_older_copy_is_replaced_and_kept_as_a_backup_there() {
    let net = Net::new("peer-older");
    let (old, _) = mine(now() - time::Duration::hours(2));
    net.put("mac-b", &old);
    let (text, stamp) = mine(now());
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), text);
    let kept = rotation::backups(net.file("mac-b").parent().unwrap());
    assert_eq!(kept.len(), 1);
    assert_eq!(std::fs::read_to_string(&kept[0]).unwrap(), old);

    // A Mac that already has it is not written again.
    let before = net.calls().len();
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }));
    assert_eq!(net.calls().len(), before + 1, "only the read");
}

#[test]
fn the_other_mac_keeps_its_ten_newest_backups() {
    let net = Net::new("peer-prune");
    let f = net.file("mac-b");
    net.put("mac-b", &mine(now() - time::Duration::hours(1)).0);
    for i in 0..12 {
        std::fs::write(f.with_file_name(format!("rotation.bak-20250101-0000{i:02}")), "old")
            .unwrap();
    }
    let (text, stamp) = mine(now());
    assert!(matches!(
        push(&net.ssh, "mac-b", &text, &stamp, now(), false).state,
        PeerState::Synced { .. }
    ));
    let kept = rotation::backups(f.parent().unwrap());
    assert_eq!(kept.len(), 10);
    // The backup just taken is the newest and stays.
    assert!(!kept[0].to_string_lossy().contains("2025"));
}

#[test]
fn an_unreachable_mac_is_tried_again_after_five_minutes() {
    let net = Net::new("peer-down");
    let (text, stamp) = mine(now());
    let r = push(&net.ssh, "down", &text, &stamp, now(), false);
    let v = rotation::version(&text);
    let PeerState::Unreachable { error, retry_at } = &r.state else { panic!("{r:?}") };
    assert_eq!(error, "ssh: connect to host down port 22: Operation timed out");
    let retry =
        OffsetDateTime::parse(retry_at, &time::format_description::well_known::Rfc3339).unwrap();
    assert_eq!(retry, now() + time::Duration::minutes(RETRY_MINUTES));
    assert!(!due(Some(&r), &v, now() + time::Duration::minutes(4)));
    assert!(due(Some(&r), &v, now() + time::Duration::minutes(5)));
}

#[test]
fn a_copy_that_reads_back_different_is_a_mismatch() {
    let net = Net::new("peer-liar");
    let (text, stamp) = mine(now());
    let r = push(&net.ssh, "liar", &text, &stamp, now(), false);
    assert!(matches!(r.state, PeerState::Mismatch { .. }), "{r:?}");
    assert!(due(
        Some(&r),
        &rotation::version(&text),
        now() + time::Duration::minutes(RETRY_MINUTES)
    ));
}

#[test]
fn the_newer_updated_wins_and_nothing_is_overwritten() {
    let net = Net::new("peer-newer");
    let (text, stamp) = mine(now());
    // Mac B saved ten minutes after this Mac's copy.
    let mut rules = common::sample_rules();
    rules.full = 88.0;
    let theirs_stamp = Stamp::new(now() + time::Duration::minutes(10), "Mac B");
    let theirs = render(&rules, &theirs_stamp, None);
    net.put("mac-b", &theirs);

    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert_eq!(r.state, PeerState::Newer { their: theirs_stamp.clone() });
    assert_eq!(r.their_text.as_deref(), Some(theirs.as_str()));
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), theirs, "left as it was");
    assert!(
        !due(Some(&r), &rotation::version(&text), now() + time::Duration::hours(1)),
        "the owner decides"
    );

    // Kept here instead: this Mac's copy is stamped again, now newer, and goes out.
    let (again, again_stamp) = mine(now() + time::Duration::minutes(11));
    assert!(due(Some(&r), &rotation::version(&again), now()), "a new copy is tried again");
    let r = push(&net.ssh, "mac-b", &again, &again_stamp, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), again);
}

#[test]
fn which_copy_wins() {
    let (text, stamp) = mine(now());
    let later = Stamp::new(now() + time::Duration::minutes(1), "Mac B");
    let earlier = Stamp::new(now() - time::Duration::minutes(1), "Mac B");
    let with = |s: &Stamp| render(&common::sample_rules(), s, None);
    assert_eq!(plan(&text, &stamp, None), Plan::Write);
    assert_eq!(plan(&text, &stamp, Some(&text)), Plan::Same);
    assert_eq!(plan(&text, &stamp, Some(&with(&later))), Plan::Newer(later));
    assert_eq!(plan(&text, &stamp, Some(&with(&earlier))), Plan::Write);
    // Same stamp, other text: edited by hand there.
    let edited = text.replace("full = 90", "full = 80");
    assert_eq!(plan(&text, &stamp, Some(&edited)), Plan::Edited(Some(stamp.clone())));
    // No stamp, or a file quota-axi cannot read: edited by hand there too.
    let unstamped = text.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(parse(&unstamped).unwrap().stamp, None);
    assert_eq!(plan(&text, &stamp, Some(&unstamped)), Plan::Edited(None));
    assert_eq!(plan(&text, &stamp, Some("garbage")), Plan::Edited(None));
}

#[test]
fn the_sync_log_survives_a_restart() {
    let dir = temp_dir("peer-log");
    let file = dir.join("rotation-sync.json");
    assert_eq!(SyncLog::load(&file), SyncLog::default());
    let (text, _) = mine(now());
    let v = rotation::version(&text);
    let mut log = SyncLog::default();
    log.peers.insert(
        "mac-b".into(),
        PeerRecord {
            state: PeerState::Synced { at: "2026-01-16T07:20:00Z".into() },
            against: v.clone(),
            their_text: None,
        },
    );
    log.peers.insert(
        "mac-c".into(),
        PeerRecord {
            state: PeerState::Unreachable {
                error: "timed out".into(),
                retry_at: "2026-01-16T07:25:00Z".into(),
            },
            against: v.clone(),
            their_text: None,
        },
    );
    log.save(&file).unwrap();
    let back = SyncLog::load(&file);
    assert_eq!(back, log);
    assert!(!due(back.peers.get("mac-b"), &v, now()));
    assert!(due(None, &v, now()), "a Mac never tried is due");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_ssh_on_path_falls_back_to_the_system_one() {
    let empty = temp_dir("peer-nossh");
    let path = OsString::from(empty.as_os_str());
    let system = PathBuf::from("/usr/bin/ssh");
    let want = system.is_file().then_some(system);
    assert_eq!(find_ssh(Some(&path)), want);
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn a_copy_with_no_stamp_there_is_kept_until_the_owner_keeps_this_one() {
    let net = Net::new("peer-unstamped");
    let hand =
        "team = 3\npersonal =\npersonal_hours = 8-23\nfull = 90\nfable_full = 95\nexpire =\n";
    net.put("mac-b", hand);
    let (text, stamp) = mine(now());
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), false);
    assert_eq!(r.state, PeerState::Edited { their: None });
    assert_eq!(r.their_text.as_deref(), Some(hand));
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), hand, "not overwritten");
    // "Giữ bản máy này": forced, and that Mac keeps a backup of its copy.
    let r = push(&net.ssh, "mac-b", &text, &stamp, now(), true);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), text);
    let kept = rotation::backups(net.file("mac-b").parent().unwrap());
    assert_eq!(std::fs::read_to_string(&kept[0]).unwrap(), hand);
}

#[test]
fn a_copy_saved_there_after_the_read_is_not_overwritten() {
    let net = Net::new("peer-race");
    let (old, _) = mine(now() - time::Duration::hours(1));
    let (text, _) = mine(now());
    // Read `old`, but the other Mac saved a newer copy before the write.
    let theirs = render(
        &common::sample_rules(),
        &Stamp::new(now() + time::Duration::minutes(1), "Mac B"),
        None,
    );
    net.put("mac-b", &theirs);
    assert_eq!(
        peers::write(&net.ssh, "mac-b", &text, Some(&old)),
        Err(peers::RemoteError::Changed)
    );
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), theirs);
    assert_eq!(
        peers::write(&net.ssh, "mac-b", &text, None),
        Err(peers::RemoteError::Changed),
        "a file appeared"
    );
    assert_eq!(peers::write(&net.ssh, "mac-b", &text, Some(&theirs)).unwrap(), text);
}

#[test]
fn two_saves_in_one_second_are_two_copies_to_send() {
    let net = Net::new("peer-twice");
    let first = Stamp::new(now(), "Mac A");
    let one = render(&common::sample_rules(), &first, None);
    let r = push(&net.ssh, "mac-b", &one, &first, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }));
    // The next stamp is always later, even within the same second.
    let second = Stamp::next(now(), "Mac A", Some(&first));
    assert!(second.newer_than(Some(&first)));
    let mut rules = common::sample_rules();
    rules.full = 85.0;
    let two = render(&rules, &second, None);
    assert!(due(Some(&r), &rotation::version(&two), now()), "the second copy is due");
    let r = push(&net.ssh, "mac-b", &two, &second, now(), false);
    assert!(matches!(r.state, PeerState::Synced { .. }), "{r:?}");
    assert_eq!(std::fs::read_to_string(net.file("mac-b")).unwrap(), two);
}
