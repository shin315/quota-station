//! The switch history: the app's journal, the switcher's log and the app's
//! Antigravity switch log in one list, and the changes of the account in use
//! the journal keeps, on synthetic files in temporary folders.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
use quota_station_core::antigravity::switch::{self, How, LogLine};
use quota_station_core::antigravity::watch;
use quota_station_core::history::{self, App, Cause, Seen, Source};
use quota_station_core::local_time;
use quota_station_core::switcher::Why;
use time::OffsetDateTime;

const T0: i64 = 1_768_548_000;
const MIN: i64 = 60;

/// An app folder with `n` Antigravity accounts, and where the switcher's log goes.
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
    (app, root.join("cswap-prefer-team.log"), ids)
}

/// A line of the switcher's log at `t`, in this Mac's time as it writes them.
fn log_line(t: i64, text: &str) -> String {
    let stamp =
        local_time::stamp(OffsetDateTime::from_unix_timestamp(t).unwrap()).replace('T', " ");
    format!("{stamp} {text}\n")
}

fn write_log(file: &Path, lines: &[String]) {
    std::fs::write(file, lines.concat()).unwrap();
}

fn agy_log(app: &AppDir, lines: &[LogLine]) {
    let text: String = lines.iter().map(|l| serde_json::to_string(l).unwrap() + "\n").collect();
    app.write_private(&switch::log_path(app), &text).unwrap();
}

#[test]
fn the_three_sources_make_one_list_newest_first() {
    let (app, log, ids) = setup(2);
    history::manual_claude(&app, Some(2), 1, Some("2026-01-16T08:20:00Z".into()), "switched", T0)
        .unwrap();
    write_log(
        &log,
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

    let all = history::collect(&app, &log, 50);
    let kinds: Vec<(App, Source, Option<u32>)> =
        all.iter().map(|e| (e.app, e.source, e.to)).collect();
    assert_eq!(
        kinds,
        vec![
            (App::Antigravity, Source::App, Some(2)),
            (App::Claude, Source::App, Some(1)),
            (App::Claude, Source::Switcher, Some(3)),
            (App::Claude, Source::Switcher, Some(2)),
        ],
        "a failed switch, a failed switcher line and other log lines are left out"
    );
    assert_eq!(all[0].from, Some(1));
    assert!(matches!(
        all[0].cause,
        Cause::AgySwitch { how: How::QuitFirst, reopened: Some(true), .. }
    ));
    assert!(
        matches!(&all[1].cause, Cause::Manual { hold_until: Some(_), result } if result == "switched")
    );
    assert!(matches!(all[2].cause, Cause::Primed { .. }));
    assert!(
        matches!(&all[3].cause, Cause::Switcher { why: Why::Room { used, ends: Some(e) }, .. } if *used == 4.0 && e == "15:00")
    );
    assert_eq!(history::collect(&app, &log, 2).len(), 2);

    // An account logged out since keeps its line, without a number.
    let mut idx = index::load(&app).unwrap();
    idx.accounts.remove(1);
    index::save(&app, &idx).unwrap();
    assert_eq!(history::collect(&app, &log, 50)[0].to, None);
}

#[test]
fn a_change_nothing_explains_is_kept_as_seen() {
    let (app, log, _) = setup(0);
    let mut seen = Seen::default();
    // The first reading only sets what was seen.
    history::track(&app, &log, &mut seen, Some(2), None, T0);
    assert!(history::read(&app).is_empty());
    history::track(&app, &log, &mut seen, Some(2), None, T0 + MIN);
    assert!(history::read(&app).is_empty());
    // A switch to 3 that neither the app nor the switcher made.
    history::track(&app, &log, &mut seen, Some(3), None, T0 + 2 * MIN);
    let all = history::collect(&app, &log, 50);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].source, all[0].from, all[0].to), (Source::Observed, Some(2), Some(3)));
    // A reading that does not say which account is in use changes nothing.
    history::track(&app, &log, &mut seen, None, None, T0 + 3 * MIN);
    assert_eq!(seen.claude, Some(3));
}

#[test]
fn a_change_the_switcher_or_the_owner_made_is_not_kept_twice() {
    let (app, log, _) = setup(0);
    let mut seen = Seen { claude: Some(2), agy: None };
    write_log(&log, &[log_line(T0 - 2 * MIN, "switch to slot 3 (0.0%, window not started)")]);
    history::track(&app, &log, &mut seen, Some(3), None, T0);
    history::manual_claude(&app, Some(3), 1, None, "switched", T0 + MIN).unwrap();
    history::track(&app, &log, &mut seen, Some(1), None, T0 + MIN + 5);
    assert!(
        history::collect(&app, &log, 50).iter().all(|e| e.source != Source::Observed),
        "both changes have their switch"
    );
    // A switch long ago does not explain a change now.
    history::track(&app, &log, &mut seen, Some(3), None, T0 + 2 * 3600);
    let observed: Vec<_> = history::collect(&app, &log, 50)
        .into_iter()
        .filter(|e| e.source == Source::Observed)
        .collect();
    assert_eq!(observed.len(), 1);
}

#[test]
fn an_overwrite_is_kept_once_and_explains_the_change_it_made() {
    let (app, log, ids) = setup(2);
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
    history::track(&app, &log, &mut seen, None, Some(&ids[1]), T0 + 5);
    watch::observe(&app, Some(&ids[1]), T0 + 5);
    // Antigravity.app writes Ultra 1 back an hour later.
    watch::observe(&app, Some(&ids[0]), T0 + 3600);
    history::track(&app, &log, &mut seen, None, Some(&ids[0]), T0 + 3600);
    history::track(&app, &log, &mut seen, None, Some(&ids[0]), T0 + 3660);
    let all = history::collect(&app, &log, 50);
    let causes: Vec<&Cause> = all.iter().map(|e| &e.cause).collect();
    assert_eq!(all.len(), 2, "{causes:?}");
    assert!(matches!(all[0].cause, Cause::AgyOverwrite { overwrites: 1 }));
    assert_eq!((all[0].from, all[0].to), (Some(2), Some(1)));
    assert!(matches!(all[1].cause, Cause::AgySwitch { .. }));
}

#[test]
fn the_journal_keeps_its_newest_lines_and_skips_broken_ones() {
    let (app, log, _) = setup(0);
    for i in 0..510 {
        history::manual_claude(&app, None, 1 + (i % 3), None, "switched", T0 + i64::from(i))
            .unwrap();
    }
    let text = std::fs::read_to_string(history::path(&app)).unwrap();
    std::fs::write(history::path(&app), text + "{ broken\n").unwrap();
    let kept = history::read(&app);
    assert_eq!(kept.len(), 500);
    assert_eq!(kept[0].at, T0 + 10);
    assert_eq!(history::collect(&app, &log, history::SHOWN).len(), history::SHOWN);
}
