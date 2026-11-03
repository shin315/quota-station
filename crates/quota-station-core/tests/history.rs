// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switch history: the app's journal, the switcher's log and the app's
//! Antigravity switch log in one list, and the changes of the account in use
//! the journal keeps, on synthetic files in temporary folders. The app
//! without an extension reads no switcher's log and has its own in
//! `history_journal.rs`.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::advanced::history::SOURCE;
use quota_station_core::advanced::history::{switcher_lines, HistoryHooks};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::advanced::switch::HOLD_UNTIL;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::switch::{self, How, LogLine};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::watch;
use quota_station_core::app_dir::AppDir;
use quota_station_core::ext::Fields;
use quota_station_core::history::{self, App, Cause, Entry, Seen, Source};
use quota_station_core::local_time;
use quota_station_core::switcher;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::switcher::Why;
#[cfg(feature = "antigravity-accounts")]
use serde_json::Value;
use time::OffsetDateTime;

const T0: i64 = 1_768_548_000;
const MIN: i64 = 60;

/// An app folder with `n` Antigravity accounts, and the home the
/// switcher's log goes under.
#[cfg(feature = "antigravity-accounts")]
fn setup(n: usize) -> (AppDir, PathBuf, Vec<String>) {
    let root = temp_dir("history");
    let app = AppDir::new(root.join("app"));
    let mut idx = AccountIndex::new();
    for i in 1..=n {
        let email = format!("ultra{i}@example.com");
        idx.accounts.push(AccountSummary {
            id: index::account_id(&email),
            email,
            name: None,
            created_at: T0,
            last_used: T0,
        });
    }
    let ids = idx.accounts.iter().map(|a| a.id.clone()).collect();
    index::save(&app, &idx).unwrap();
    (app, root.join("home"), ids)
}

/// An app folder and the home the switcher's log goes under; without the
/// Antigravity accounts there is no list to put accounts in.
#[cfg(not(feature = "antigravity-accounts"))]
fn setup(n: usize) -> (AppDir, PathBuf, Vec<String>) {
    assert_eq!(n, 0);
    let root = temp_dir("history");
    (AppDir::new(root.join("app")), root.join("home"), Vec::new())
}

/// A line of the switcher's log at `t`, in this Mac's time as it writes them.
fn log_line(t: i64, text: &str) -> String {
    let stamp =
        local_time::stamp(OffsetDateTime::from_unix_timestamp(t).unwrap()).replace('T', " ");
    format!("{stamp} {text}\n")
}

/// Writes the switcher's log under `home`.
fn write_log(home: &Path, lines: &[String]) {
    let file = switcher::log_file(home);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, lines.concat()).unwrap();
}

/// The history with the advanced part's lines, read under `home`.
fn collect(app: &AppDir, home: &Path, limit: usize) -> Vec<Entry> {
    history::collect(app, home, &HistoryHooks, limit)
}

/// What the readings show changed, with the advanced part's lines read
/// under `home`.
fn track(
    app: &AppDir,
    home: &Path,
    seen: &mut Seen,
    claude: Option<u32>,
    agy: Option<&str>,
    now: i64,
) {
    history::track(app, home, &HistoryHooks, seen, claude, agy, now);
}

/// The reason the advanced part gave for one of its lines.
#[cfg(feature = "antigravity-accounts")]
fn reason(cause: &Cause) -> Value {
    match cause {
        Cause::Ext(c) => serde_json::from_str(c.json()).unwrap(),
        other => panic!("not the advanced part's: {other:?}"),
    }
}

#[cfg(feature = "antigravity-accounts")]
fn agy_log(app: &AppDir, lines: &[LogLine]) {
    let text: String = lines.iter().map(|l| serde_json::to_string(l).unwrap() + "\n").collect();
    app.write_private(&switch::log_path(app), &text).unwrap();
}

#[cfg(feature = "antigravity-accounts")]
#[test]
fn the_three_sources_make_one_list_newest_first() {
    let (app, home, ids) = setup(2);
    let mut held = Fields::new();
    held.insert(HOLD_UNTIL, "2026-01-16T08:20:00Z".into());
    history::manual_claude(&app, Some(2), 1, held, "switched", T0).unwrap();
    write_log(
        &home,
        &[
            log_line(T0 - 30 * MIN, "switch to slot 2 (4.0%, ends 15:00)"),
            log_line(T0 - 20 * MIN, "primed slot 3 (exit 0)"),
            log_line(T0 - 10 * MIN, "pause ended (until 2026-01-16T14:00)"),
            log_line(T0 - 5 * MIN, "switch to slot 4 failed"),
        ],
    );
    let line = |at, to: &str, result: &str| LogLine {
        at,
        from: Some(ids[0].clone()),
        to: to.to_string(),
        how: How::QuitFirst,
        result: result.to_string(),
        error: None,
        reopened: Some(true),
    };
    agy_log(&app, &[line(T0 + MIN, &ids[1], "switched"), line(T0 + 2 * MIN, &ids[1], "failed")]);

    let all = collect(&app, &home, 50);
    let kinds: Vec<(App, Source, Option<u32>)> =
        all.iter().map(|e| (e.app, e.source, e.to)).collect();
    assert_eq!(
        kinds,
        vec![
            (App::Antigravity, Source::App, Some(2)),
            (App::Claude, Source::App, Some(1)),
            (App::Claude, Source::Ext(SOURCE), Some(3)),
            (App::Claude, Source::Ext(SOURCE), Some(2)),
        ],
        "a failed switch, a failed switcher line and other log lines are left out"
    );
    assert_eq!(all[0].from, Some(1));
    assert!(matches!(
        all[0].cause,
        Cause::AgySwitch { how: How::QuitFirst, reopened: Some(true), .. }
    ));
    assert!(
        matches!(&all[1].cause, Cause::Manual { ext, result } if ext.get(HOLD_UNTIL).is_some_and(|h| h.is_string()) && result == "switched")
    );
    assert_eq!(reason(&all[2].cause)["kind"], "primed");
    let why = Why::Room { used: 4.0, ends: Some("15:00".into()) };
    assert_eq!(reason(&all[3].cause)["kind"], "switcher");
    assert_eq!(reason(&all[3].cause)["why"], serde_json::to_value(why).unwrap());
    assert_eq!(collect(&app, &home, 2).len(), 2);

    // An account logged out since keeps its line, without a number.
    let mut idx = index::load(&app).unwrap();
    idx.accounts.remove(1);
    index::save(&app, &idx).unwrap();
    assert_eq!(collect(&app, &home, 50)[0].to, None);
}

#[test]
fn a_change_nothing_explains_is_kept_as_seen() {
    let (app, home, _) = setup(0);
    let mut seen = Seen::default();
    // The first reading only sets what was seen.
    track(&app, &home, &mut seen, Some(2), None, T0);
    assert!(history::read(&app).is_empty());
    track(&app, &home, &mut seen, Some(2), None, T0 + MIN);
    assert!(history::read(&app).is_empty());
    // A switch to 3 that neither the app nor the switcher made.
    track(&app, &home, &mut seen, Some(3), None, T0 + 2 * MIN);
    let all = collect(&app, &home, 50);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].source, all[0].from, all[0].to), (Source::Observed, Some(2), Some(3)));
    // A reading that does not say which account is in use changes nothing.
    track(&app, &home, &mut seen, None, None, T0 + 3 * MIN);
    assert_eq!(seen.claude, Some(3));
}

#[test]
fn a_change_the_switcher_or_the_owner_made_is_not_kept_twice() {
    let (app, home, _) = setup(0);
    let mut seen = Seen { claude: Some(2), agy: None };
    write_log(&home, &[log_line(T0 - 2 * MIN, "switch to slot 3 (0.0%, window not started)")]);
    track(&app, &home, &mut seen, Some(3), None, T0);
    history::manual_claude(&app, Some(3), 1, Fields::new(), "switched", T0 + MIN).unwrap();
    track(&app, &home, &mut seen, Some(1), None, T0 + MIN + 5);
    assert!(
        collect(&app, &home, 50).iter().all(|e| e.source != Source::Observed),
        "both changes have their switch"
    );
    // A switch long ago does not explain a change now.
    track(&app, &home, &mut seen, Some(3), None, T0 + 2 * 3600);
    let observed: Vec<_> =
        collect(&app, &home, 50).into_iter().filter(|e| e.source == Source::Observed).collect();
    assert_eq!(observed.len(), 1);
}

#[cfg(feature = "antigravity-accounts")]
#[test]
fn an_overwrite_is_kept_once_and_explains_the_change_it_made() {
    let (app, home, ids) = setup(2);
    let mut seen = Seen { claude: None, agy: Some(ids[0].clone()) };
    agy_log(
        &app,
        &[LogLine {
            at: T0,
            from: Some(ids[0].clone()),
            to: ids[1].clone(),
            how: How::KeepRunning,
            result: "switched".into(),
            error: None,
            reopened: None,
        }],
    );
    watch::begin(&app, &ids[0], &ids[1], T0).unwrap();
    track(&app, &home, &mut seen, None, Some(&ids[1]), T0 + 5);
    watch::observe(&app, Some(&ids[1]), T0 + 5);
    // Antigravity.app writes Ultra 1 back an hour later.
    watch::observe(&app, Some(&ids[0]), T0 + 3600);
    track(&app, &home, &mut seen, None, Some(&ids[0]), T0 + 3600);
    track(&app, &home, &mut seen, None, Some(&ids[0]), T0 + 3660);
    let all = collect(&app, &home, 50);
    let causes: Vec<&Cause> = all.iter().map(|e| &e.cause).collect();
    assert_eq!(all.len(), 2, "{causes:?}");
    assert!(matches!(all[0].cause, Cause::AgyOverwrite { overwrites: 1 }));
    assert_eq!((all[0].from, all[0].to), (Some(2), Some(1)));
    assert!(matches!(all[1].cause, Cause::AgySwitch { .. }));
}

#[test]
fn the_journal_keeps_its_newest_lines_and_skips_broken_ones() {
    let (app, home, _) = setup(0);
    for i in 0..510 {
        history::manual_claude(
            &app,
            None,
            1 + (i % 3),
            Fields::new(),
            "switched",
            T0 + i64::from(i),
        )
        .unwrap();
    }
    let text = std::fs::read_to_string(history::path(&app)).unwrap();
    std::fs::write(history::path(&app), text + "{ broken\n").unwrap();
    let kept = history::read(&app);
    assert_eq!(kept.len(), 500);
    assert_eq!(kept[0].at, T0 + 10);
    assert_eq!(collect(&app, &home, history::SHOWN).len(), history::SHOWN);
}

#[test]
fn a_line_that_starts_the_log_s_last_64_kb_is_kept_and_a_cut_one_is_not() {
    let (_, home, _) = setup(0);
    let tail: usize = 64 * 1024;
    let first = log_line(T0 - 50 * MIN, "switch to slot 2 (4.0%, ends 15:00)");
    let target = log_line(T0 - 40 * MIN, "switch to slot 3 (5.0%, ends 16:00)");
    // A line of padding so that the target and what follows are exactly the tail.
    let stamp = log_line(T0 - 30 * MIN, "");
    let pad = log_line(T0 - 30 * MIN, &"x".repeat(tail - target.len() - stamp.len()));
    assert_eq!(target.len() + pad.len(), tail);
    write_log(&home, &[first.clone(), target.clone(), pad.clone()]);
    let slots: Vec<_> =
        switcher_lines(&switcher::log_file(&home)).iter().map(|l| l.raw.clone()).collect();
    assert_eq!(slots, vec![target.trim_end().to_string()]);

    // One byte more of padding: the target is cut, so it is left out.
    let pad = log_line(T0 - 30 * MIN, &"x".repeat(tail - target.len() - stamp.len() + 1));
    write_log(&home, &[first, target, pad]);
    assert!(switcher_lines(&switcher::log_file(&home)).is_empty());
}

#[test]
fn codex_switches_and_changes_seen_outside_come_from_the_codex_log() {
    use quota_station_core::codex::log as codex_log;
    let (app, home, _) = setup(0);
    let line = |at: i64, from: &str, to: &str, result: &str| codex_log::Line {
        at,
        from: from.into(),
        to: to.into(),
        result: result.into(),
        restarted: None,
        requires: None,
    };
    codex_log::append(&app, &line(T0, "openai", "relay", "switched")).unwrap();
    codex_log::mark_restarted(&app, true).unwrap();
    codex_log::append(&app, &line(T0 + MIN, "relay", "openai", "failed")).unwrap();
    codex_log::append(&app, &line(T0 + 2 * MIN, "relay", "openai", "observed")).unwrap();

    let all = collect(&app, &home, 50);
    assert_eq!(all.len(), 2, "a failed switch is not history");
    assert!(all.iter().all(|e| e.app == App::Codex && e.from.is_none() && e.to.is_none()));
    assert_eq!(all[0].source, Source::Observed);
    assert_eq!(all[0].cause, Cause::CodexObserved { from: "relay".into(), to: "openai".into() });
    assert_eq!(all[1].source, Source::App);
    assert_eq!(
        all[1].cause,
        Cause::CodexSwitch { from: "openai".into(), to: "relay".into(), restarted: Some(true) }
    );
}
