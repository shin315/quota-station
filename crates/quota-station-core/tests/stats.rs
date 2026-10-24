// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Thống kê (M16, J), on fake homes holding synthetic session logs
//! (`tests/fixtures/stats/`): Claude Code 1.0 and 2.0, Codex before and
//! after its token events, and versions in a format the app does not know.
//! Nothing here reads the real `~/.claude` or `~/.codex`.
//!
//! The trap: every prompt, reply, thinking block, tool call, tool output and
//! instruction in the fixtures carries the same made-up text. It must never
//! reach the index, the report, or any file of the app's.

mod common;

use std::path::{Path, PathBuf};

use common::temp_dir;
use quota_station_core::app_dir::AppDir;
use quota_station_core::history::{App, Cause, Entry, Source};
use quota_station_core::stats::index::{self, Index, Progress, Roots};
use quota_station_core::stats::report::{self, Period, Timeline};
use quota_station_core::stats::{claude_log, codex_log, prices, Agent, Line, Tokens};

use time::{OffsetDateTime, UtcOffset};

/// In every line of conversation in the fixtures.
const TRAP: &str = "TRAP-conversation-7c41e9";
/// 2026-01-16 07:20 UTC, 14:20 on a Mac at UTC+7.
fn now() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap()
}

fn plus7(_: OffsetDateTime) -> UtcOffset {
    UtcOffset::from_hms(7, 0, 0).unwrap()
}

fn fixture(name: &str) -> String {
    common::fixture_text(&format!("stats/{name}"))
}

/// A fake Mac: a home, a Codex folder, the app's folder.
struct Mac {
    home: PathBuf,
    codex: PathBuf,
    app: AppDir,
}

impl Mac {
    fn new(tag: &str) -> Self {
        let root = temp_dir(tag);
        let home = root.join("home");
        let codex = home.join(".codex");
        let app = AppDir::new(root.join("app"));
        std::fs::create_dir_all(&home).unwrap();
        Mac { home, codex, app }
    }

    fn roots(&self) -> Roots {
        Roots::new(&self.home, &self.codex)
    }

    fn claude_log(&self, project: &str, session: &str) -> PathBuf {
        self.home.join(".claude/projects").join(project).join(format!("{session}.jsonl"))
    }

    fn codex_log(&self, day: &str, session: &str) -> PathBuf {
        self.codex.join("sessions").join(day).join(format!("rollout-{session}.jsonl"))
    }

    fn put(&self, path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn append(&self, path: &Path, text: &str) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    /// Reads the logs into the index on disk, as the app does.
    fn read(&self) -> Index {
        let mut ix = index::load(&self.app);
        ix.update(&self.roots(), now().unix_timestamp(), &mut |_| {});
        index::save(&self.app, &ix).unwrap();
        ix
    }
}

fn totals(ix: &Index, agent: Agent) -> Tokens {
    index::totals(ix).get(&agent).copied().unwrap_or_default()
}

fn week(ix: &Index) -> report::Report {
    report::report(ix, Period::Week, now(), &plus7, &Timeline::default(), Path::new("/Users/demo"))
}

// Claude Code 2.0: a response over two lines (thinking, then a tool call)
// with the same usage counts once; a made-up `<synthetic>` reply and the
// summary, snapshot and system lines count nothing.
const CLAUDE_20: Tokens = Tokens { input: 14, cache_write: 2500, cache_read: 22000, output: 420 };
const CLAUDE_10: Tokens = Tokens { input: 1500, cache_write: 0, cache_read: 0, output: 250 };
// Codex 0.46: the first count, the same count again (nothing), and the
// growth to the next; `cached_input_tokens` is inside `input_tokens`.
const CODEX_046: Tokens = Tokens { input: 10000, cache_write: 0, cache_read: 20000, output: 1000 };

fn full_mac(tag: &str) -> Mac {
    let mac = Mac::new(tag);
    mac.put(&mac.claude_log("-Users-demo-Projects-web-shop", "s1"), &fixture("claude-2.0.jsonl"));
    mac.put(&mac.claude_log("-Users-demo-Projects-notes-app", "s2"), &fixture("claude-1.0.jsonl"));
    mac.put(
        &mac.claude_log("-Users-demo-Projects-web-shop", "s3"),
        &fixture("claude-unknown.jsonl"),
    );
    mac.put(&mac.codex_log("2026/01/16", "a"), &fixture("codex-0.46.jsonl"));
    mac.put(&mac.codex_log("2025/12/20", "old"), &fixture("codex-old.jsonl"));
    mac.put(&mac.codex_log("2026/01/16", "b"), &fixture("codex-0.170.jsonl"));
    mac
}

#[test]
fn claude_code_two_counts_a_response_once() {
    let mac = Mac::new("stats-claude2");
    mac.put(&mac.claude_log("-Users-demo-Projects-web-shop", "s1"), &fixture("claude-2.0.jsonl"));
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude), CLAUDE_20);
    assert_eq!(ix.skipped()[0].sessions, 0, "every line of 2.0 is known");
    let models: Vec<&str> = ix.buckets.iter().map(|b| b.model.as_str()).collect();
    assert!(
        models.contains(&"claude-sonnet-4-5-20250929")
            && models.contains(&"claude-opus-4-5-20251101")
    );
    assert!(!models.contains(&"<synthetic>"));
    assert!(ix.buckets.iter().all(|b| b.project == "/Users/demo/Projects/web-shop"));
}

#[test]
fn claude_code_one_without_cache_fields() {
    let mac = Mac::new("stats-claude1");
    mac.put(&mac.claude_log("-Users-demo-Projects-notes-app", "s2"), &fixture("claude-1.0.jsonl"));
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude), CLAUDE_10);
}

#[test]
fn codex_counts_the_growth_of_its_totals() {
    let mac = Mac::new("stats-codex");
    mac.put(&mac.codex_log("2026/01/16", "a"), &fixture("codex-0.46.jsonl"));
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Codex), CODEX_046);
    let by_model: Vec<(&str, u64)> =
        ix.buckets.iter().map(|b| (b.model.as_str(), b.tokens.total())).collect();
    assert!(by_model.contains(&("gpt-5-codex", 12600)), "{by_model:?}");
    assert!(by_model.contains(&("gpt-5.1-codex-max", 18400)), "{by_model:?}");
    assert!(ix.buckets.iter().all(|b| b.project == "/Users/demo/Projects/api-server"));
}

#[test]
fn old_codex_logs_have_no_tokens_and_are_not_unknown() {
    let mac = Mac::new("stats-codex-old");
    mac.put(&mac.codex_log("2025/12/20", "old"), &fixture("codex-old.jsonl"));
    let ix = mac.read();
    assert!(ix.buckets.is_empty());
    assert_eq!(ix.skipped()[1].sessions, 0);
    assert_eq!(ix.logs_of(Agent::Codex), 1);
}

#[test]
fn unknown_formats_are_skipped_and_counted_never_guessed() {
    let mac = full_mac("stats-unknown");
    let ix = mac.read();
    // The known logs still count, the unknown lines add nothing.
    assert_eq!(totals(&ix, Agent::Claude), {
        let mut t = CLAUDE_20;
        t.add(&CLAUDE_10);
        t
    });
    assert_eq!(totals(&ix, Agent::Codex), CODEX_046);
    let skipped = ix.skipped();
    let claude = &skipped[0];
    assert_eq!((claude.agent, claude.sessions, claude.lines), (Agent::Claude, 1, 4));
    assert_eq!(claude.versions, vec!["9.9.0".to_string()]);
    let codex = &skipped[1];
    assert_eq!((codex.agent, codex.sessions, codex.lines), (Agent::Codex, 1, 2));
    assert_eq!(codex.versions, vec!["0.170.0".to_string()]);
    assert!(codex.since.is_some());

    // A second read does not count them again.
    let ix = mac.read();
    assert_eq!(ix.skipped()[1].lines, 2);

    // "Đã biết" keeps the number it saw.
    let mut ix = ix;
    ix.acknowledge();
    assert_eq!(ix.skipped()[1].acknowledged, 1);
}

#[test]
fn single_lines_by_kind() {
    let mut c = claude_log::File::default();
    assert_eq!(c.line(b"{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}"), Line::Quiet);
    assert_eq!(c.line(b"{\"type\":\"assistant\",\"message\":{\"content\":[]}}"), Line::Unknown);
    assert_eq!(c.line(b"[1,2]"), Line::Unknown);
    let mut x = codex_log::File::default();
    assert_eq!(
        x.line(b"{\"type\":\"response_item\",\"payload\":{\"type\":\"message\"}}"),
        Line::Quiet
    );
    assert_eq!(
        x.line(b"{\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":null}}"),
        Line::Quiet
    );
    assert_eq!(x.line(b"{\"payload\":{}}"), Line::Unknown);
}

#[test]
fn later_reads_take_only_what_was_appended() {
    let mac = Mac::new("stats-append");
    let log = mac.claude_log("-Users-demo-Projects-web-shop", "s1");
    let text = fixture("claude-2.0.jsonl");
    let lines: Vec<&str> = text.lines().collect();
    // The first read stops in the middle of a line still being written.
    let (start, rest) = lines.split_at(4);
    let partial = &rest[0][..40];
    mac.put(&log, &format!("{}\n{partial}", start.join("\n")));
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude).total(), 12310, "the first response only");
    let offset = ix.logs[0].offset;
    assert_eq!(offset as usize, start.join("\n").len() + 1, "up to the last whole line");

    // The rest of that line and the others arrive.
    mac.append(&log, &format!("{}\n{}\n", &rest[0][40..], rest[1..].join("\n")));
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude), CLAUDE_20);

    // Nothing new: nothing changes.
    let again = mac.read();
    assert_eq!(again.buckets, ix.buckets);
    assert_eq!(again.logs[0].offset, ix.logs[0].offset);
}

#[test]
fn a_truncated_log_is_read_again_without_counting_twice() {
    let mac = Mac::new("stats-truncate");
    let log = mac.claude_log("-Users-demo-Projects-web-shop", "s1");
    mac.put(&log, &fixture("claude-2.0.jsonl"));
    mac.read();
    // Rewritten shorter, then a new response added: the old responses are
    // known by their keys, the new one counts.
    let text = fixture("claude-2.0.jsonl");
    let first: Vec<&str> = text.lines().take(3).collect();
    let new =
        text.lines().nth(5).unwrap().replace("msg_01B", "msg_01C").replace("req_01B", "req_01C");
    std::fs::write(&log, format!("{}\n{new}\n", first.join("\n"))).unwrap();
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude).total(), CLAUDE_20.total() + 12624);
}

#[test]
fn a_replaced_log_is_a_new_log() {
    let mac = Mac::new("stats-replace");
    let log = mac.codex_log("2026/01/16", "a");
    mac.put(&log, &fixture("codex-0.46.jsonl"));
    mac.read();
    // Removed, and a different session written under the same name.
    std::fs::remove_file(&log).unwrap();
    let other =
        fixture("codex-0.46.jsonl").replace("0199-codex-1", "0199-codex-9").replace("04:", "06:");
    mac.put(&log, &other);
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Codex).total(), 2 * CODEX_046.total());
}

#[test]
fn a_deleted_log_keeps_its_counts() {
    let mac = full_mac("stats-delete");
    let before = mac.read();
    std::fs::remove_dir_all(mac.home.join(".claude")).unwrap();
    std::fs::remove_dir_all(mac.codex.join("sessions")).unwrap();
    let after = mac.read();
    assert_eq!(after.buckets, before.buckets);
    // Only the logs with skipped lines stay listed, to keep the notice true.
    assert!(after.logs.iter().all(|l| l.gone && l.skipped > 0));
    assert_eq!(after.skipped()[1].sessions, 1);
}

#[test]
fn an_archived_codex_session_is_not_counted_twice() {
    let mac = Mac::new("stats-archive");
    let log = mac.codex_log("2026/01/16", "a");
    mac.put(&log, &fixture("codex-0.46.jsonl"));
    mac.read();
    // Codex moves the file; then the session goes on in it.
    let archived = mac.codex.join("archived_sessions/rollout-a.jsonl");
    std::fs::create_dir_all(archived.parent().unwrap()).unwrap();
    std::fs::rename(&log, &archived).unwrap();
    mac.append(
        &archived,
        "{\"timestamp\":\"2026-01-16T05:10:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"total_token_usage\":{\"input_tokens\":31000,\"cached_input_tokens\":20000,\"output_tokens\":1100}}}}\n",
    );
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Codex).total(), CODEX_046.total() + 1000 + 100);
    assert_eq!(ix.logs_of(Agent::Codex), 1);
}

#[test]
fn a_resumed_claude_session_copying_old_responses_counts_them_once() {
    let mac = Mac::new("stats-resume");
    mac.put(&mac.claude_log("-Users-demo-Projects-web-shop", "s1"), &fixture("claude-2.0.jsonl"));
    mac.read();
    mac.put(
        &mac.claude_log("-Users-demo-Projects-web-shop", "s1-resumed"),
        &fixture("claude-2.0.jsonl"),
    );
    let ix = mac.read();
    assert_eq!(totals(&ix, Agent::Claude), CLAUDE_20);
}

#[test]
fn the_index_survives_a_restart_and_drops_old_counts() {
    let mac = full_mac("stats-reload");
    let first = mac.read();
    let loaded = index::load(&mac.app);
    assert_eq!(loaded, first);
    assert!(!loaded.seen.is_empty());
    // A month later the counts are past what the index keeps.
    let mut later = loaded.clone();
    later.update(&mac.roots(), now().unix_timestamp() + 70 * 86_400, &mut |_| {});
    assert!(later.buckets.is_empty());
    assert!(later.seen.is_empty());
    // An index the app cannot read starts again from the logs.
    std::fs::write(index::path(&mac.app), "{not json").unwrap();
    assert_eq!(mac.read().buckets, first.buckets);
}

#[test]
fn progress_counts_each_agent_s_logs() {
    let mac = full_mac("stats-progress");
    let mut seen: Vec<Progress> = Vec::new();
    let mut ix = Index::default();
    assert!(ix.first_read(Agent::Claude));
    ix.update(&mac.roots(), now().unix_timestamp(), &mut |p| seen.push(p));
    assert_eq!(seen.first(), Some(&Progress { agent: Agent::Claude, done: 0, total: 3 }));
    assert_eq!(seen.last(), Some(&Progress { agent: Agent::Codex, done: 3, total: 3 }));
    assert!(!ix.first_read(Agent::Codex));
}

#[test]
fn no_conversation_text_is_kept_anywhere() {
    let mac = full_mac("stats-trap");
    let ix = mac.read();
    let rep = week(&ix);
    let today = report::report(
        &ix,
        Period::Today,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    let mut seen = vec![
        serde_json::to_string(&ix.skipped()).unwrap(),
        serde_json::to_string(&rep).unwrap(),
        serde_json::to_string(&today).unwrap(),
        format!("{ix:?}"),
    ];
    // Every file the app wrote.
    fn files(dir: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                files(&p, out);
            } else {
                out.push(String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into_owned());
            }
        }
    }
    files(mac.app.path(), &mut seen);
    // The window's own tests draw this very report (`src/fixtures/stats-trap.json`):
    // `UPDATE_UI_FIXTURES=1` writes it again after a change to its shape. The
    // time a skipped log was found is its change time here, so it is left out.
    let mut window = serde_json::to_value(&rep).unwrap();
    for s in window["skipped"].as_array_mut().unwrap() {
        s["since"] = serde_json::Value::Null;
    }
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src/fixtures/stats-trap.json");
    let text = serde_json::to_string_pretty(&window).unwrap() + "\n";
    if std::env::var_os("UPDATE_UI_FIXTURES").is_some() {
        std::fs::write(&fixture, &text).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&fixture).unwrap(), text, "run with UPDATE_UI_FIXTURES=1");
    seen.push(text);
    assert!(seen.len() > 4, "the index was written");
    for text in &seen {
        assert!(!text.contains(TRAP), "conversation text in {}", &text[..text.len().min(200)]);
        assert!(!text.contains("giỏ hàng"), "a prompt's words were kept");
    }
}

#[test]
fn the_week_by_day_on_this_mac_s_clock() {
    let mac = full_mac("stats-week");
    let rep = week(&mac.read());
    let labels: Vec<&str> = rep.points.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["10/01", "11/01", "12/01", "13/01", "14/01", "15/01", "16/01"]);
    assert_eq!(rep.points[5].claude, CLAUDE_10.total());
    assert_eq!(rep.points[6].claude, CLAUDE_20.total());
    assert_eq!(rep.points[6].codex, CODEX_046.total());
    assert_eq!(rep.total, CLAUDE_10.total() + CLAUDE_20.total() + CODEX_046.total());
    assert_eq!(rep.claude + rep.codex, rep.total);
    // By project: the folder's name and its path from the home.
    let web = rep.projects.iter().find(|p| p.name == "web-shop").unwrap();
    assert_eq!((web.path.as_str(), web.tokens), ("~/Projects/web-shop", CLAUDE_20.total()));
    assert_eq!(rep.projects[0].name, "api-server");
    // By model, each priced from the one table.
    let opus = rep.models.iter().find(|m| m.model == "claude-opus-4-5-20251101").unwrap();
    assert_eq!((opus.input, opus.cache_read, opus.output), (504, 12000, 120));
    let want = (4.0 * 5.0 + 500.0 * 6.25 + 12000.0 * 0.5 + 120.0 * 25.0) / 1e6;
    assert!((opus.usd.unwrap() - want).abs() < 1e-12);
    assert!(rep.unpriced.is_empty(), "{:?}", rep.unpriced);
    let sum: f64 = rep.models.iter().filter_map(|m| m.usd).sum();
    assert!((rep.usd - sum).abs() < 1e-12);
    assert_eq!(rep.prices_as_of, prices::AS_OF);
}

#[test]
fn today_by_hour() {
    let mac = full_mac("stats-today");
    let ix = mac.read();
    let rep = report::report(
        &ix,
        Period::Today,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    assert_eq!(rep.points.len(), 24);
    assert_eq!(rep.points[9].label, "09:00");
    assert_eq!(rep.points[9].claude, CLAUDE_20.total(), "02:00 and 02:20 UTC");
    assert_eq!(rep.points[11].codex, CODEX_046.total(), "04:00 and 04:30 UTC");
    assert_eq!(rep.total, CLAUDE_20.total() + CODEX_046.total(), "yesterday is not today");
    let month = report::report(
        &ix,
        Period::Month,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    assert_eq!(month.points.len(), 30);
    assert_eq!(month.points[0].label, "18/12");
}

fn switch(at: &str, from: Option<u32>, to: u32) -> Entry {
    Entry {
        at: at.into(),
        app: App::Claude,
        source: Source::App,
        from,
        to: Some(to),
        cause: Cause::Manual { hold_until: None, result: "switched".into() },
    }
}

#[test]
fn claude_accounts_follow_the_switch_history() {
    let mac = full_mac("stats-accounts");
    let ix = mac.read();
    // Slot 2 until 02:10 UTC, then slot 3: the first response is 2's, the second 3's.
    let entries = vec![
        switch("2026-01-16T02:10:00Z", Some(2), 3),
        Entry {
            at: "2026-01-16T01:00:00Z".into(),
            app: App::Antigravity,
            source: Source::App,
            from: Some(1),
            to: Some(2),
            cause: Cause::Observed,
        },
    ];
    let line = Timeline::from_history(&entries, Some(3));
    assert_eq!(line.before, Some(2));
    let rep = report::report(&ix, Period::Week, now(), &plus7, &line, Path::new("/Users/demo"));
    let of = |slot: Option<u32>| rep.accounts.iter().find(|a| a.slot == slot).map(|a| a.tokens);
    assert_eq!(of(Some(2)), Some(12310 + CLAUDE_10.total()));
    assert_eq!(of(Some(3)), Some(12624));
    // With no switch at all, everything goes to the account in use.
    let none = Timeline::from_history(&[], Some(1));
    let rep = report::report(&ix, Period::Week, now(), &plus7, &none, Path::new("/Users/demo"));
    assert_eq!(rep.accounts.len(), 1);
    assert_eq!(rep.accounts[0].slot, Some(1));
    // With neither, the account is not known.
    let rep = report::report(
        &ix,
        Period::Week,
        now(),
        &plus7,
        &Timeline::default(),
        Path::new("/Users/demo"),
    );
    assert_eq!(rep.accounts[0].slot, None);
}

#[test]
fn prices_by_model_id() {
    assert_eq!(prices::price("claude-sonnet-4-5-20250929").unwrap().input, 3.0);
    assert_eq!(prices::price("claude-opus-5-5").unwrap().output, 20.0);
    assert_eq!(prices::price("gpt-5-codex").unwrap().cache_read, 0.125);
    assert_eq!(prices::price("gpt-5.1-codex-max").unwrap().input, 1.25);
    assert_eq!(prices::price("gpt-5-2025-08-07").unwrap().input, 1.25);
    // A model the table does not have is not priced like a neighbour.
    assert!(prices::price("gpt-5.7").is_none());
    assert!(prices::price("claude-opus-6").is_none());
    assert!(prices::price("claude-opus-4-9-20270101").is_none());
    assert!(prices::price("").is_none());
}
