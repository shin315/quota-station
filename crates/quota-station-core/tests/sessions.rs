// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Live Claude Code sessions counted from a temporary folder of records.

mod common;

use common::temp_dir;
use quota_station_core::os::{self, ProcessList};
use quota_station_core::sessions::{self, count, count_with, Sessions};

fn record(dir: &std::path::Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).unwrap();
}

#[test]
fn counts_records_whose_pid_is_alive() {
    let dir = temp_dir("sessions");
    let me = std::process::id();
    // A finished child's pid is dead: this test program, listing nothing.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--exact", "no-such-test"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let dead = child.id();
    child.wait().unwrap();

    // Other fields are present in real records and are never read.
    record(
        &dir,
        "a.json",
        &format!(r#"{{"pid":{me},"kind":"interactive","cwd":"/x","sessionId":"s"}}"#),
    );
    record(&dir, "b.json", &format!(r#"{{"pid":{me},"kind":"bg"}}"#));
    record(&dir, "c.json", &format!(r#"{{"pid":{dead},"kind":"interactive"}}"#));
    record(&dir, "d.json", &format!(r#"{{"pid":{me},"kind":"daemon"}}"#));
    record(&dir, "e.json", "{ not json");
    record(&dir, "f.json", r#"{"kind":"interactive"}"#);
    record(&dir, "notes.txt", "not a record");

    assert_eq!(count(&dir).unwrap(), Sessions { live: 2, unreadable: 2 });
}

#[test]
fn the_liveness_check_decides_and_a_missing_folder_is_no_session() {
    let dir = temp_dir("sessions-fake");
    record(&dir, "1.json", r#"{"pid":4242}"#);
    record(&dir, "2.json", r#"{"pid":4343,"kind":"daemon-worker"}"#);
    assert_eq!(count_with(&dir, |pid| pid == 4242).unwrap().live, 1);
    assert_eq!(count_with(&dir, |_| true).unwrap().live, 2);

    assert_eq!(count(&dir.join("nope")).unwrap(), Sessions::default());
}

#[test]
fn pid_checks() {
    let processes = os::processes();
    assert!(processes.alive(std::process::id() as i32));
    assert!(!processes.alive(0));
    assert!(!processes.alive(1), "pid 1 is launchd, never a session");
    assert!(!processes.alive(-5));
}

#[test]
fn the_records_live_in_the_default_claude_folder() {
    let home = std::path::Path::new("/home/someone");
    assert_eq!(sessions::dir(home), home.join(".claude/sessions"));
}
