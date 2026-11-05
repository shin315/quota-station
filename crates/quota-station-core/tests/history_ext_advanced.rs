// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switch history of one fixed set of files, as the app shows it with
//! the advanced part's hooks: the app's journal and its Antigravity and
//! Codex switch logs, and the switcher's log under the home, which only the
//! advanced part reads; the lines, the stats' timeline of the Claude
//! accounts drawn from them, and what the readings that follow keep in the
//! journal (`history_ext.rs` has the history without an extension). Pinned per build by golden files the build wrote before an
//! extension could add its own lines (`UPDATE_HISTORY_GOLDEN=1` rewrites
//! them after a change the windows are meant to see). The text is compared,
//! so the keys' order counts too.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::log as codex_log;
use quota_station_core::ext::Hooks;
use quota_station_core::history::{self, Entry, Seen};
use quota_station_core::local_time;
use quota_station_core::stats::report::Timeline;
use time::OffsetDateTime;

/// 2026-01-16 07:20 UTC.
const T0: i64 = 1_768_548_000;
const MIN: i64 = 60;

/// The home and the app's folder the history is read from.
struct Files {
    home: PathBuf,
    app: AppDir,
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The switcher's log under the home: when each line was written (from
/// `T0`, in seconds) and what it says.
const LOG: [(i64, &str); 12] = [
    (-60 * MIN, "switch to slot 2 (4.0%, ends 15:00)"),
    (-55 * MIN, "switch to slot 3 (0.0%, window not started)"),
    (-50 * MIN, "primed slot 4 (exit 0)"),
    (-45 * MIN, "switch to slot 5 (ends for good at 12:00, use it first)"),
    (-44 * MIN, "switch to slot 1 (ends 13:00 with 30% left, before slot 2)"),
    (-43 * MIN, "switch to slot 2 (park: no room anywhere, it resets first at 14:00)"),
    (-42 * MIN, "switch to slot 6 (12.0%, no Team slot with room)"),
    (-41 * MIN, "switch to slot 3 (something odd)"),
    (-40 * MIN, "pause ended (until 2026-01-16T14:00)"),
    (-39 * MIN, "switch to slot 4 failed: x: y"),
    (-30 * MIN, "switch to slot 2 (5.0%, ends 16:00)"),
    (-2 * MIN, "switch to slot 3 (1.0%, ends 17:00)"),
];

/// This Mac's time at `t`, as the switcher's log writes it.
fn stamp(t: i64) -> String {
    local_time::stamp(OffsetDateTime::from_unix_timestamp(t).unwrap()).replace('T', " ")
}

/// A line of the switcher's log at `t`, in this Mac's time.
fn log_line(t: i64, text: &str) -> String {
    format!("{} {text}\n", stamp(t))
}

/// `text` as it reads in every time zone.
fn stable(text: String) -> String {
    // Each of the switcher's times in this Mac's time put as `<T0-3600>`.
    let mut text = text;
    for (at, _) in LOG {
        text = text.replace(&stamp(T0 + at), &format!("<T0{at:+}>"));
    }
    text
}

/// The fixed files: the journal (the owner's switches, with and without a
/// hold, changes seen, an Antigravity overwrite, a line of no slot, a
/// broken line and a Codex line the journal does not show), the
/// Antigravity switch log (switched, failed, and to an account no longer
/// listed), the Codex switch log (switched and restarted, failed, seen),
/// and two Antigravity accounts listed where the build keeps them.
/// Also the old switcher's log ([`quota_station_core::switcher::LOG`]) under
/// the home (switches with each kind of reason, a prime, lines that are
/// neither, one without a time, and one at the same second as a journal
/// line), which only the advanced part reads.
fn files() -> Files {
    let root = temp_dir("history-ext");
    let home = root.join("home");
    let app = AppDir::new(root.join("app"));
    let journal = [
        format!(
            r#"{{"at":{},"app":"claude","from":"2","to":"1","what":{{"kind":"manual","holdUntil":"2026-01-16T08:20:00Z","result":"switched"}}}}"#,
            T0 - 90 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"claude","from":"1","to":"2","what":{{"kind":"manual","result":"raced"}}}}"#,
            T0 - 30 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"claude","from":"2","to":"3","what":{{"kind":"observed"}}}}"#,
            T0 - 25 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"claude","to":"x","what":{{"kind":"manual","result":"switched"}}}}"#,
            T0 - 20 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"antigravity","from":"a1","to":"a2","what":{{"kind":"observed"}}}}"#,
            T0 - 15 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"antigravity","from":"a2","to":"a1","what":{{"kind":"overwrite","overwrites":2}}}}"#,
            T0 - 10 * MIN
        ),
        "{ broken".to_string(),
        format!(
            r#"{{"at":{},"app":"claude","to":"4","what":{{"kind":"manual","result":"unconfirmed"}}}}"#,
            T0 - 8 * MIN
        ),
        format!(
            r#"{{"at":{},"app":"codex","to":"relay","what":{{"kind":"observed"}}}}"#,
            T0 - 5 * MIN
        ),
    ];
    write(&history::path(&app), &(journal.join("\n") + "\n"));

    let agy = [
        format!(
            r#"{{"at":{},"from":"a1","to":"a2","how":"quitFirst","result":"switched","reopened":true}}"#,
            T0 - 70 * MIN
        ),
        format!(
            r#"{{"at":{},"from":"a2","to":"a1","how":"plain","result":"failed","error":"x"}}"#,
            T0 - 65 * MIN
        ),
        format!(
            r#"{{"at":{},"from":null,"to":"a3","how":"keepRunning","result":"unconfirmed"}}"#,
            T0 - 62 * MIN
        ),
    ];
    write(&app.path().join("antigravity/switch-log.jsonl"), &(agy.join("\n") + "\n"));
    #[cfg(feature = "antigravity-accounts")]
    {
        let mut idx = AccountIndex::new();
        for id in ["a1", "a2"] {
            idx.accounts.push(AccountSummary {
                id: id.into(),
                email: format!("{id}@example.com"),
                name: None,
                created_at: T0,
                last_used: T0,
            });
        }
        index::save(&app, &idx).unwrap();
    }

    let codex = |at: i64, from: &str, to: &str, result: &str| codex_log::Line {
        at,
        from: from.into(),
        to: to.into(),
        result: result.into(),
        restarted: None,
        requires: None,
    };
    codex_log::append(&app, &codex(T0 - 75 * MIN, "openai", "relay", "switched")).unwrap();
    codex_log::mark_restarted(&app, true).unwrap();
    codex_log::append(&app, &codex(T0 - 74 * MIN, "relay", "openai", "failed")).unwrap();
    codex_log::append(&app, &codex(T0 - 12 * MIN, "relay", "openai", "observed")).unwrap();

    let mut log: Vec<String> = LOG.iter().map(|(at, text)| log_line(T0 + at, text)).collect();
    log.insert(10, "no time here switch to slot 2 (1.0%, ends 15:00)\n".to_string());
    write(&home.join("Library/Logs/cswap-prefer-team.log"), &log.concat());
    Files { home, app }
}

/// The entries, one JSON line each, as the windows get them.
fn lines(entries: &[Entry]) -> String {
    entries.iter().map(|e| serde_json::to_string(e).unwrap() + "\n").collect()
}

/// What the history of `f` shows with `hooks`: every line, the newest
/// three, the timeline of the Claude accounts, then two readings (Claude
/// from 2 to 3, then to 5 with Antigravity from a1 to a2) and what the
/// journal and the history are after them.
fn text(f: &Files, hooks: &dyn Hooks) -> String {
    let collect = |limit| history::collect(&f.app, &f.home, hooks, limit);
    let mut out = String::new();
    out += "# collect(50)\n";
    out += &lines(&collect(50));
    out += "# collect(3)\n";
    out += &lines(&collect(3));
    let t = Timeline::from_history(&collect(usize::MAX), Some(9));
    out += &format!("# timeline\nbefore {:?} now {:?}\n{:?}\n", t.before, t.now, t.switches);
    let mut seen = Seen { claude: Some(2), agy: Some("a1".into()) };
    history::track(&f.app, &f.home, hooks, &mut seen, Some(3), None, T0);
    history::track(&f.app, &f.home, hooks, &mut seen, Some(5), Some("a2"), T0 + MIN);
    out += &format!("# seen {:?} {:?}\n", seen.claude, seen.agy);
    out += "# journal after track\n";
    out += &std::fs::read_to_string(history::path(&f.app)).unwrap();
    out += "# collect(50) after track\n";
    out += &lines(&collect(50));
    stable(out)
}

/// Compares `text` with the golden file `path` (from the crate's folder),
/// or rewrites it.
fn assert_golden(text: &str, path: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    if std::env::var_os("UPDATE_HISTORY_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).expect("the golden file");
    assert_eq!(text, golden, "the history differs from {}", path.display());
}

/// The hooks the app has with the advanced part, for its history: that part's.
fn hooks() -> &'static dyn Hooks {
    &quota_station_core::advanced::history::HistoryHooks
}

/// This build's golden file.
fn golden() -> &'static str {
    if cfg!(feature = "antigravity-accounts") {
        "src/advanced/fixtures/history-antigravity-accounts.txt"
    } else {
        "src/advanced/fixtures/history.txt"
    }
}

/// The history is this build's: the same lines, sources, reasons, times
/// and order, the same timeline and the same lines kept after the
/// readings as the build had before an extension could add its own.
#[test]
fn the_history_is_the_golden_one_of_this_build() {
    assert_golden(&text(&files(), hooks()), golden());
}
