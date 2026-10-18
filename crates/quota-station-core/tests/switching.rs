//! A manual switch end to end with a fake `cswap` and a fake quota-axi in a
//! temporary home: these tests never run claude-swap, never touch the
//! Keychain and never write the real pause flag.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{fixture_path, script, temp_dir};
use quota_station_core::cswap::{self, SwitchError};
use quota_station_core::switching::{switch_claude, Outcome, Tools};
use quota_station_core::{local_time, pause};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const SWITCHED: &str = r#"{
  "schemaVersion": 1,
  "switched": true,
  "from": {"number": 2, "email": "someone@example.com"},
  "to": {"number": 3, "email": "other@example.com"},
  "strategy": "switch-to",
  "reason": "switched",
  "message": "Switched to Account-3 (other@example.com)",
  "warnings": []
}"#;

/// A home with a fake quota-axi and a fake cswap whose script `cswap(home)`
/// returns. quota-axi reports the slot in `<home>/active` as claude-swap's
/// active slot on a read that skips its cache (`--max-age 0`); a read that
/// may reuse the cache gets the reading from before the switch, slot 2, as
/// quota-axi does while a Keychain login changes under a cached reading.
fn home_with(cswap: impl Fn(&Path) -> String) -> (PathBuf, Tools) {
    let home = temp_dir("switching");
    std::fs::write(home.join("active"), "2\n").unwrap();
    let axi = script(
        &home.join(".local/bin/quota-axi"),
        &format!(
            "case \" $* \" in *\" --max-age 0 \"*) slot=$(cat '{}') ;; *) slot=2 ;; esac\n\
             sed \"s/\\\"activeSlot\\\": [0-9][0-9]*/\\\"activeSlot\\\": $slot/\" '{}'\n",
            home.join("active").display(),
            fixture_path("claude-pool-schema5.json").display()
        ),
    );
    let tools = Tools {
        cswap: Some(script(&home.join(".local/bin/cswap"), &cswap(&home))),
        quota_axi: Some(axi),
        pause_file: pause::path(&home),
        log_file: home.join("Library/Logs/cswap-prefer-team.log"),
    };
    (home, tools)
}

/// A fake `cswap switch N --json` that lands on N and records its arguments.
fn succeeding(home: &Path) -> String {
    format!(
        "printf '%s\\n' \"$@\" > '{args}'\necho \"$2\" > '{active}'\ncat <<'JSON'\n{SWITCHED}\nJSON\n",
        args = home.join("args").display(),
        active = home.join("active").display(),
    )
}

#[test]
fn a_confirmed_switch_with_a_hold_writes_the_pause_flag() {
    let (home, tools) = home_with(succeeding);
    let until = OffsetDateTime::now_utc() + time::Duration::hours(1);

    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(50));

    let args = std::fs::read_to_string(home.join("args")).unwrap();
    assert_eq!(args.lines().collect::<Vec<_>>(), ["switch", "3", "--json"]);
    let Outcome::Switched { slot: 3, hold: Some(hold) } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(!hold.failed);
    let flag = pause::read(&tools.pause_file).unwrap().expect("flag written");
    let written = flag.until.clone().unwrap();
    assert_eq!(written, local_time::minute(until));
    assert!(flag.reason.as_deref().unwrap().starts_with("chuyển tay sang tài khoản 3 lúc "));
    assert!(flag.holds(&local_time::stamp(OffsetDateTime::now_utc())));
    let shown = OffsetDateTime::parse(&hold.until, &Rfc3339).unwrap();
    assert_eq!(local_time::instant(&written), Some(shown));
    // The re-read report comes back to show at once.
    assert!(done.report.is_some());
}

#[test]
fn no_hold_leaves_the_switcher_alone() {
    let (_, tools) = home_with(succeeding);
    let done = switch_claude(&tools, 3, None, Duration::from_millis(50));
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert!(!tools.pause_file.exists());
}

#[test]
fn a_hold_already_over_is_no_hold() {
    let (_, tools) = home_with(succeeding);
    let past = OffsetDateTime::now_utc() - time::Duration::minutes(1);
    let done = switch_claude(&tools, 3, Some(past), Duration::from_millis(50));
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert!(!tools.pause_file.exists());
}

#[test]
fn an_error_from_cswap_changes_nothing() {
    let (_, tools) = home_with(|_| {
        "cat <<'JSON'\n{\"schemaVersion\": 1, \"error\": {\"type\": \"ClaudeSwitchError\", \"message\": \"Account-3 (other@example.com) needs /login\"}}\nJSON\nexit 1\n".into()
    });
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);
    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(50));
    let Outcome::Failed { slot: 3, error, detail, active } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert_eq!(error, "refused");
    assert_eq!(detail, "ClaudeSwitchError: Account-3 … needs /login", "no email reaches the UI");
    assert_eq!(*active, None, "claude-swap said it did not switch: no re-read");
    assert!(!tools.pause_file.exists(), "no hold after a failed switch");
    assert!(done.report.is_none());
}

#[test]
fn the_check_after_a_switch_skips_quota_axi_cache() {
    // A cached reading would still name slot 2, the account before the switch.
    let (_, tools) = home_with(succeeding);
    let done = switch_claude(&tools, 3, None, Duration::from_millis(10));
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    let active = done
        .report
        .as_ref()
        .and_then(|r| r.primary("claude"))
        .and_then(|row| row.claude_swap.as_ref().and_then(|swap| swap.active_slot));
    assert_eq!(active, Some(3), "the stored report is the fresh one");
}

#[test]
fn cswap_stopping_without_an_answer_is_checked_before_reporting() {
    // claude-swap switched, then ended without its JSON answer.
    let (_, tools) = home_with(|h| {
        format!("echo \"$2\" > '{}'\necho 'interrupted' >&2\nexit 1\n", h.join("active").display())
    });
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);
    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(10));
    let Outcome::Switched { slot: 3, hold: Some(hold) } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(!hold.failed, "the switch landed, so the hold is written");
    assert!(tools.pause_file.exists());

    // It ended without switching: the failure says which account is in use.
    let (_, tools) = home_with(|_| "echo 'interrupted' >&2\nexit 1\n".into());
    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(10));
    let Outcome::Failed { slot: 3, error, active, .. } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert_eq!(error, "exit");
    assert_eq!(*active, Some(2));
    assert!(done.report.is_some());
    assert!(!tools.pause_file.exists(), "no hold when the switch did not land");
}

#[test]
fn a_hold_never_shortens_a_longer_pause() {
    let (_, tools) = home_with(succeeding);
    let hour = OffsetDateTime::now_utc() + time::Duration::hours(1);

    // Paused until the owner turns it back on: a one-hour hold leaves it be.
    let open = pause::Flag {
        until: None,
        reason: Some("tạm dừng trong Quota Station lúc 14:20".into()),
    };
    pause::write(&tools.pause_file, &open).unwrap();
    let done = switch_claude(&tools, 3, Some(hour), Duration::from_millis(10));
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert_eq!(pause::read(&tools.pause_file).unwrap(), Some(open));

    // Paused for longer than the hold: kept too.
    let later = pause::Flag {
        until: Some(local_time::minute(hour + time::Duration::hours(2))),
        reason: Some("tạm dừng trong Quota Station lúc 14:20".into()),
    };
    pause::write(&tools.pause_file, &later).unwrap();
    let done = switch_claude(&tools, 3, Some(hour), Duration::from_millis(10));
    assert_eq!(done.outcome, Outcome::Switched { slot: 3, hold: None });
    assert_eq!(pause::read(&tools.pause_file).unwrap(), Some(later));

    // A shorter pause gives way to the hold.
    let sooner = pause::Flag {
        until: Some(local_time::minute(OffsetDateTime::now_utc() + time::Duration::minutes(10))),
        reason: None,
    };
    pause::write(&tools.pause_file, &sooner).unwrap();
    let done = switch_claude(&tools, 3, Some(hour), Duration::from_millis(10));
    assert!(matches!(&done.outcome, Outcome::Switched { hold: Some(h), .. } if !h.failed));
    let flag = pause::read(&tools.pause_file).unwrap().unwrap();
    assert_eq!(flag.until, Some(local_time::minute(hour)));
}

#[test]
fn a_hold_that_cannot_be_written_is_reported() {
    let (home, tools) = home_with(succeeding);
    // The flag's folder is a file, so the flag cannot be written.
    std::fs::write(home.join("blocked"), "").unwrap();
    let tools = Tools { pause_file: home.join("blocked/pause"), ..tools };
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);
    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(10));
    let Outcome::Switched { slot: 3, hold: Some(hold) } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(hold.failed, "the switch stands, and the UI says the switcher is not held");
}

#[test]
fn the_switcher_switching_back_is_reported_as_raced() {
    // cswap lands on 3; a moment later the switcher puts slot 2 back and logs it.
    let (_, tools) = home_with(|h| {
        let log = h.join("Library/Logs/cswap-prefer-team.log");
        format!(
            "{switch}( sleep 1; echo 2 > '{active}'; mkdir -p '{logs}'; echo \"$(date '+%F %T') switch to slot 2 (ends for good at 17:45, use it first)\" >> '{log}' ) >/dev/null 2>&1 &\n",
            switch = succeeding(h),
            active = h.join("active").display(),
            logs = log.parent().unwrap().display(),
            log = log.display(),
        )
    });
    let until = OffsetDateTime::now_utc() + time::Duration::minutes(30);

    let done = switch_claude(&tools, 3, Some(until), Duration::from_millis(2500));

    let Outcome::Raced { slot: 3, active: 2, by_switcher_at, hold } = &done.outcome else {
        panic!("{:?}", done.outcome)
    };
    assert!(by_switcher_at.is_some(), "the switcher's log names the switch");
    assert!(hold.as_ref().is_some_and(|h| !h.failed), "the hold flag stays written");
}

#[test]
fn another_switch_winning_is_raced_without_the_switcher() {
    let (_, tools) = home_with(|h| {
        format!(
            "{switch}( sleep 1; echo 4 > '{active}' ) >/dev/null 2>&1 &\n",
            switch = succeeding(h),
            active = h.join("active").display(),
        )
    });
    let done = switch_claude(&tools, 3, None, Duration::from_millis(2500));
    assert!(matches!(
        done.outcome,
        Outcome::Raced { slot: 3, active: 4, by_switcher_at: None, .. }
    ));
}

#[test]
fn without_quota_axi_the_switch_is_unconfirmed() {
    let (_, tools) = home_with(succeeding);
    let tools = Tools { quota_axi: None, ..tools };
    let done = switch_claude(&tools, 3, None, Duration::from_millis(10));
    assert!(matches!(done.outcome, Outcome::Unconfirmed { slot: 3, hold: None }));
}

#[test]
fn a_missing_cswap_fails_before_anything() {
    let (_, tools) = home_with(succeeding);
    let tools = Tools { cswap: None, ..tools };
    let done = switch_claude(&tools, 3, None, Duration::from_millis(10));
    assert!(matches!(&done.outcome, Outcome::Failed { error, .. } if error == "notFound"));
}

#[test]
fn cswap_answers() {
    let dir = temp_dir("cswap");
    // Already on the target counts as done.
    let already = script(
        &dir.join("already"),
        "echo '{\"switched\": false, \"from\": {\"number\": 3}, \"to\": {\"number\": 3}, \"reason\": \"already-active\"}'\n",
    );
    assert_eq!(cswap::switch(&already, 3).unwrap().reason, "already-active");
    // A no-op elsewhere is a refusal with claude-swap's reason.
    let disabled = script(
        &dir.join("disabled"),
        "echo '{\"switched\": false, \"from\": {\"number\": 2}, \"to\": {\"number\": 2}, \"reason\": \"no-valid-target\", \"message\": \"Account-3 is disabled\"}'\n",
    );
    assert_eq!(
        cswap::switch(&disabled, 3).unwrap_err(),
        SwitchError::Refused {
            kind: "no-valid-target".into(),
            message: "Account-3 is disabled".into()
        }
    );
    // No JSON: the exit code and the first stderr line, without an email.
    let crash = script(&dir.join("crash"), "echo 'boom for me@example.com' >&2\nexit 3\n");
    assert_eq!(
        cswap::switch(&crash, 3).unwrap_err(),
        SwitchError::Exit { code: Some(3), message: "boom for …".into() }
    );
    // A reason or error type with an email in it loses the email too.
    let odd = script(
        &dir.join("odd"),
        "echo '{\"switched\": false, \"to\": {\"number\": 2}, \"reason\": \"locked-by me@example.com\"}'\n",
    );
    assert!(
        matches!(cswap::switch(&odd, 3).unwrap_err(), SwitchError::Refused { kind, .. } if kind == "locked-by …")
    );
    assert_eq!(cswap::switch(&dir.join("nope"), 3).unwrap_err(), SwitchError::NotFound);
}

#[test]
fn cswap_is_found_in_local_bin_first() {
    let home = temp_dir("cswap-discover");
    assert_eq!(cswap::candidates(&home)[0], home.join(".local/bin/cswap"));
    let fake = script(&home.join(".local/bin/cswap"), "exit 0\n");
    assert_eq!(cswap::discover(&home), Some(fake));
    assert_eq!(cswap::switch_args(4), ["switch", "4", "--json"]);
}
