// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The switch history of one fixed set of files, as the app shows it with
//! the hooks it has ([`Hooks::history_lines`]): the app's journal and its
//! Antigravity and Codex switch logs; the lines, the stats' timeline of the
//! Claude accounts drawn from them, and what the readings that follow keep
//! in the journal.
//! Pinned per build by golden files the build wrote before an extension
//! could add its own lines (`UPDATE_HISTORY_GOLDEN=1` rewrites them after a
//! change the windows are meant to see). The text is compared, so the keys'
//! order counts too.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::log as codex_log;
use quota_station_core::ext::{self, Hooks, NoHooks};
use quota_station_core::history::{self, Entry, Seen, Source};
use quota_station_core::stats::report::Timeline;

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

/// `text` as it reads in every time zone.
fn stable(text: String) -> String {
    text
}

/// The fixed files: the journal (the owner's switches, with and without a
/// hold, changes seen, an Antigravity overwrite, a line of no slot, a
/// broken line and a Codex line the journal does not show), the
/// Antigravity switch log (switched, failed, and to an account no longer
/// listed), the Codex switch log (switched and restarted, failed, seen),
/// and two Antigravity accounts listed where the build keeps them.
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

/// The hooks the app has in this build, for its history.
fn hooks() -> &'static dyn Hooks {
    ext::none()
}

/// This build's golden file.
fn golden() -> &'static str {
    if cfg!(feature = "antigravity-accounts") {
        "tests/fixtures/history-antigravity-accounts.txt"
    } else {
        "tests/fixtures/history.txt"
    }
}

/// The history is this build's: the same lines, sources, reasons, times
/// and order, the same timeline and the same lines kept after the
/// readings as the build had before an extension could add its own.
#[test]
fn the_history_is_the_golden_one_of_this_build() {
    assert_golden(&text(&files(), hooks()), golden());
}

/// Without an extension's lines the history is the app's own: the hooks of
/// no extension add none and read nothing under the home.
#[test]
fn without_an_extension_the_history_is_the_apps_own() {
    let none = text(&files(), ext::none());
    assert_eq!(text(&files(), &NoHooks), none);
    let f = files();
    let all = history::collect(&f.app, &f.home, ext::none(), 50);
    assert!(all.iter().all(|e| matches!(e.source, Source::App | Source::Observed)));
    assert_golden(&none, golden());
}
