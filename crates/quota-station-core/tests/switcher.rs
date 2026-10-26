// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation's decision lines, its log and launchd, read from fakes:
//! these tests never run the real `launchctl`.

mod common;

use common::{script, temp_dir};
use quota_station_core::switcher::{
    decided, last_log_line, log_file, parse_decision, parse_log_line, service_loaded, Decision,
    LogEvent, Why,
};

#[test]
fn every_decision_line_the_switcher_prints() {
    use Decision::*;
    let at = |s: &str| s.to_string();
    let cases = [
        ("stay", Stay),
        ("stay (slot 4 usage unknown)", Unknown { slot: 4 }),
        ("stay (waiting for slot 3 at 16:40)", Waiting { slot: 3, at: at("16:40") }),
        (
            "switch 2 (ends for good at 17:45, use it first)",
            Switch { slot: 2, why: Why::Expiring { at: at("17:45") } },
        ),
        (
            "switch 3 (ends 15:00 with 40% left, before slot 4)",
            Switch { slot: 3, why: Why::Early { at: at("15:00"), left: 40, before: 4 } },
        ),
        (
            "switch 4 (4.0%, ends 15:00)",
            Switch { slot: 4, why: Why::Room { used: 4.0, ends: Some(at("15:00")) } },
        ),
        (
            "switch 3 (0.0%, window not started)",
            Switch { slot: 3, why: Why::Room { used: 0.0, ends: None } },
        ),
        (
            "switch 1 (21.0%, no Team slot with room)",
            Switch { slot: 1, why: Why::Personal { used: 21.0 } },
        ),
        (
            "switch 3 (park: no room anywhere, it resets first at 16:40)",
            Switch { slot: 3, why: Why::Park { at: at("16:40") } },
        ),
        ("wait (no account with room)", Wait),
        (
            "blocked (com.cswap.prefer-team is loaded)",
            Blocked { label: at("com.cswap.prefer-team") },
        ),
        ("switch 3 (something new)", Switch { slot: 3, why: Why::Other }),
        ("dance", Other),
    ];
    for (line, want) in cases {
        assert_eq!(parse_decision(line), want, "{line}");
    }
}

#[test]
fn the_pause_guard_lines() {
    assert_eq!(
        parse_decision("paused until 2026-10-16T18:30: chuyển tay sang tài khoản 1 lúc 17:30"),
        Decision::Paused {
            until: Some("2026-10-16T18:30".into()),
            reason: Some("chuyển tay sang tài khoản 1 lúc 17:30".into())
        }
    );
    assert_eq!(
        parse_decision("paused indefinitely: tạm dừng trong Quota Station lúc 14:20"),
        Decision::Paused {
            until: None,
            reason: Some("tạm dừng trong Quota Station lúc 14:20".into())
        }
    );
    assert_eq!(
        parse_decision("paused until 2026-10-16T18:30"),
        Decision::Paused { until: Some("2026-10-16T18:30".into()), reason: None }
    );
    assert_eq!(
        parse_decision("paused indefinitely"),
        Decision::Paused { until: None, reason: None }
    );
    // Other text after `paused` is not the guard, so it does not prove one.
    assert_eq!(parse_decision("paused by someone else"), Decision::Other);
    assert_eq!(parse_decision("paused until "), Decision::Other);
    assert_eq!(parse_decision("paused indefinitelyish"), Decision::Other);
}

#[test]
fn a_decision_keeps_the_line_as_printed() {
    let run = decided("switch 4 (4.0%, ends 15:00)\n");
    assert_eq!(run.raw, "switch 4 (4.0%, ends 15:00)");
    assert!(matches!(run.decision, Decision::Switch { slot: 4, .. }));
}

#[test]
fn the_log_is_the_old_switchers_until_the_app_keeps_the_accounts() {
    let home = temp_dir("switcher-log-file");
    assert_eq!(log_file(&home), home.join("Library/Logs/cswap-prefer-team.log"));
    let paths = quota_station_core::claude::Paths::for_home(&home);
    std::fs::create_dir_all(&paths.dir).unwrap();
    std::fs::write(paths.accounts(), "{\"version\":1,\"accounts\":[]}").unwrap();
    assert_eq!(log_file(&home), paths.log());
}

#[test]
fn log_lines() {
    let switch = parse_log_line("2026-10-16 12:51:44 switch to slot 4 (4.0%, ends 15:00)");
    assert_eq!(switch.event, LogEvent::Switch { slot: 4 });
    let at = switch.at.expect("a time");
    let back =
        time::OffsetDateTime::parse(&at, &time::format_description::well_known::Rfc3339).unwrap();
    assert_eq!(quota_station_core::local_time::stamp(back), "2026-10-16T12:51:44");
    assert_eq!(switch.raw, "2026-10-16 12:51:44 switch to slot 4 (4.0%, ends 15:00)");

    assert_eq!(
        parse_log_line("2026-10-16 12:54:44 switch to slot 4 failed").event,
        LogEvent::SwitchFailed { slot: 4 }
    );
    assert_eq!(
        parse_log_line("2026-10-16 12:54:44 switch to slot 4 failed: busy: .claude.lock is held by another process").event,
        LogEvent::SwitchFailed { slot: 4 }
    );
    assert_eq!(
        parse_log_line("2026-10-16 12:54:44 prime of slot 4 failed: session: no such account")
            .event,
        LogEvent::Other
    );
    assert_eq!(
        parse_log_line("2026-10-16 12:55:01 primed slot 3 (exit 0)").event,
        LogEvent::Primed { slot: 3 }
    );
    assert_eq!(
        parse_log_line("2026-10-16 18:30:12 pause ended (until 2026-10-16T18:30)").event,
        LogEvent::PauseEnded
    );
    let odd = parse_log_line("something else");
    assert_eq!((odd.at, odd.event), (None, LogEvent::Other));
}

#[test]
fn only_the_end_of_a_long_log_is_read() {
    let dir = temp_dir("switcher-log");
    let file = dir.join("cswap-prefer-team.log");
    let mut text = String::new();
    for i in 0..2000 {
        text.push_str(&format!("2026-10-15 10:{:02}:00 primed slot 3 (exit 0)\n", i % 60));
    }
    text.push_str("2026-10-16 12:51:44 switch to slot 4 (4.0%, ends 15:00)\n\n");
    std::fs::write(&file, text).unwrap();
    assert_eq!(last_log_line(&file).unwrap().event, LogEvent::Switch { slot: 4 });
    assert_eq!(last_log_line(&dir.join("none.log")), None);
}

#[test]
fn launchd_says_whether_a_service_is_loaded() {
    let dir = temp_dir("launchctl");
    let fake = script(
        &dir.join("launchctl"),
        "[ \"$1\" = list ] || exit 2\ncase \"$2\" in\n  com.cswap.prefer-team) echo '{}'; exit 0 ;;\n  com.cswap.auto) echo 'Could not find service' >&2; exit 113 ;;\n  *) exit 5 ;;\nesac\n",
    );
    assert_eq!(service_loaded(&fake, "com.cswap.prefer-team"), Some(true));
    assert_eq!(service_loaded(&fake, "com.cswap.auto"), Some(false));
    assert_eq!(service_loaded(&fake, "other"), None);
    assert_eq!(service_loaded(&dir.join("missing"), "com.cswap.auto"), None);
}
