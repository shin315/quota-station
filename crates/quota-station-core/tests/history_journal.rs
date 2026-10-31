// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switch history: the app's journal and the app's Antigravity and
//! Codex switch logs in one list, and the changes of the account in use the
//! journal keeps, on synthetic files in temporary folders.

#![cfg(not(feature = "author-tools"))]

mod common;

use common::temp_dir;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::switch::{self, How, LogLine};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::watch;
use quota_station_core::app_dir::AppDir;
use quota_station_core::history::{self, App, Cause, Seen, Source};

const T0: i64 = 1_768_548_000;
const MIN: i64 = 60;

/// An app folder with `n` Antigravity accounts.
#[cfg(feature = "antigravity-accounts")]
fn setup(n: usize) -> (AppDir, Vec<String>) {
    let app = AppDir::new(temp_dir("history-journal").join("app"));
    let mut idx = AccountIndex::new();
    for i in 1..=n {
        let email = format!("agy{i}@example.com");
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
    (app, ids)
}

/// An app folder; without the Antigravity accounts there is no list to put
/// accounts in.
#[cfg(not(feature = "antigravity-accounts"))]
fn setup(n: usize) -> (AppDir, Vec<String>) {
    assert_eq!(n, 0);
    (AppDir::new(temp_dir("history-journal").join("app")), Vec::new())
}

#[cfg(feature = "antigravity-accounts")]
fn agy_log(app: &AppDir, lines: &[LogLine]) {
    let text: String = lines.iter().map(|l| serde_json::to_string(l).unwrap() + "\n").collect();
    app.write_private(&switch::log_path(app), &text).unwrap();
}

#[cfg(feature = "antigravity-accounts")]
#[test]
fn the_journal_and_the_antigravity_log_make_one_list_newest_first() {
    let (app, ids) = setup(2);
    history::manual_claude(&app, Some(2), 1, "switched", T0).unwrap();
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

    let all = history::collect(&app, 50);
    let kinds: Vec<(App, Source, Option<u32>)> =
        all.iter().map(|e| (e.app, e.source, e.to)).collect();
    assert_eq!(
        kinds,
        vec![(App::Antigravity, Source::App, Some(2)), (App::Claude, Source::App, Some(1))],
        "a failed switch is left out"
    );
    assert_eq!(all[0].from, Some(1));
    assert!(matches!(
        all[0].cause,
        Cause::AgySwitch { how: How::QuitFirst, reopened: Some(true), .. }
    ));
    assert!(matches!(&all[1].cause, Cause::Manual { result } if result == "switched"));
    assert_eq!(history::collect(&app, 1).len(), 1);

    // An account logged out since keeps its line, without a number.
    let mut idx = index::load(&app).unwrap();
    idx.accounts.remove(1);
    index::save(&app, &idx).unwrap();
    assert_eq!(history::collect(&app, 50)[0].to, None);
}

#[test]
fn a_change_nothing_explains_is_kept_as_seen() {
    let (app, _) = setup(0);
    let mut seen = Seen::default();
    // The first reading only sets what was seen.
    history::track(&app, &mut seen, Some(2), None, T0);
    assert!(history::read(&app).is_empty());
    history::track(&app, &mut seen, Some(2), None, T0 + MIN);
    assert!(history::read(&app).is_empty());
    // A switch to 3 that the app did not make.
    history::track(&app, &mut seen, Some(3), None, T0 + 2 * MIN);
    let all = history::collect(&app, 50);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].source, all[0].from, all[0].to), (Source::Observed, Some(2), Some(3)));
    // A reading that does not say which account is in use changes nothing.
    history::track(&app, &mut seen, None, None, T0 + 3 * MIN);
    assert_eq!(seen.claude, Some(3));
}

#[test]
fn a_change_the_owner_made_is_not_kept_twice() {
    let (app, _) = setup(0);
    let mut seen = Seen { claude: Some(3), agy: None };
    history::manual_claude(&app, Some(3), 1, "switched", T0 + MIN).unwrap();
    history::track(&app, &mut seen, Some(1), None, T0 + MIN + 5);
    assert!(
        history::collect(&app, 50).iter().all(|e| e.source != Source::Observed),
        "the change has its switch"
    );
    // A switch long ago does not explain a change now.
    history::track(&app, &mut seen, Some(3), None, T0 + 2 * 3600);
    let observed: Vec<_> =
        history::collect(&app, 50).into_iter().filter(|e| e.source == Source::Observed).collect();
    assert_eq!(observed.len(), 1);
}

#[cfg(feature = "antigravity-accounts")]
#[test]
fn an_overwrite_is_kept_once_and_explains_the_change_it_made() {
    let (app, ids) = setup(2);
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
    history::track(&app, &mut seen, None, Some(&ids[1]), T0 + 5);
    watch::observe(&app, Some(&ids[1]), T0 + 5);
    // Antigravity.app writes account 1 back an hour later.
    watch::observe(&app, Some(&ids[0]), T0 + 3600);
    history::track(&app, &mut seen, None, Some(&ids[0]), T0 + 3600);
    history::track(&app, &mut seen, None, Some(&ids[0]), T0 + 3660);
    let all = history::collect(&app, 50);
    let causes: Vec<&Cause> = all.iter().map(|e| &e.cause).collect();
    assert_eq!(all.len(), 2, "{causes:?}");
    assert!(matches!(all[0].cause, Cause::AgyOverwrite { overwrites: 1 }));
    assert_eq!((all[0].from, all[0].to), (Some(2), Some(1)));
    assert!(matches!(all[1].cause, Cause::AgySwitch { .. }));
}

#[test]
fn the_journal_keeps_its_newest_lines_and_skips_broken_ones() {
    let (app, _) = setup(0);
    for i in 0..510 {
        history::manual_claude(&app, None, 1 + (i % 3), "switched", T0 + i64::from(i)).unwrap();
    }
    let text = std::fs::read_to_string(history::path(&app)).unwrap();
    std::fs::write(history::path(&app), text + "{ broken\n").unwrap();
    let kept = history::read(&app);
    assert_eq!(kept.len(), 500);
    assert_eq!(kept[0].at, T0 + 10);
    assert_eq!(history::collect(&app, history::SHOWN).len(), history::SHOWN);
}

#[test]
fn codex_switches_and_changes_seen_outside_come_from_the_codex_log() {
    use quota_station_core::codex::log as codex_log;
    let (app, _) = setup(0);
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

    let all = history::collect(&app, 50);
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
