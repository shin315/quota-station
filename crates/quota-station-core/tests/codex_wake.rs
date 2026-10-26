// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Tự đánh thức tài khoản ChatGPT": when each chosen account is woken,
//! and what a wake-up ended as. Only the decisions are here, on made-up
//! accounts and times; nothing runs Codex or reaches OpenAI.
//!
//! What these prove: off wakes nothing; an account not chosen is never
//! woken; a daily wake-up runs once a day per account, at its time or a
//! little after a Mac that slept; an open window, a week used up or a
//! refused sign-in is recorded instead of run; the reset mode wakes once a
//! window at most and never loops, even when every wake-up fails; the
//! model's answer is never kept.

mod common;

use common::temp_dir;
use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::wake::{self, Book, Login, Mode, Outcome, Run, Seen, Settings, Why};
use time::{Duration, OffsetDateTime, UtcOffset};

/// 2026-01-16 00:00 at UTC+7, the Mac's time in these tests.
const MIDNIGHT: i64 = 1_768_496_400;

fn hcm() -> UtcOffset {
    UtcOffset::from_hms(7, 0, 0).unwrap()
}

/// `h:m` on 2026-01-16, the Mac's time, `days` later.
fn local(days: i64, h: i64, m: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(MIDNIGHT + days * 86_400 + h * 3600 + m * 60).unwrap()
}

fn settings(mode: Mode, ids: &[&str]) -> Settings {
    Settings {
        on: true,
        mode,
        at: "06:00".into(),
        keep_alive: false,
        ids: ids.iter().map(|s| s.to_string()).collect(),
    }
}

/// An account signed in, its five-hour window not started, its week not used up.
fn idle(id: &str) -> Seen {
    Seen {
        id: id.into(),
        login: Login::Ok,
        waiting: true,
        five_used: Some(0.0),
        five_resets: None,
        week_left: Some(80.0),
    }
}

fn ids(plans: &[wake::Plan]) -> Vec<(&str, Why, Option<Outcome>)> {
    plans.iter().map(|p| (p.id.as_str(), p.why, p.skip)).collect()
}

fn ran(at: OffsetDateTime, id: &str, why: Why, result: Outcome) -> Run {
    Run { at: at.unix_timestamp(), id: id.into(), why, result, detail: None }
}

#[test]
fn off_wakes_nothing_and_an_account_not_chosen_is_never_woken() {
    let seen = [idle("chatgpt:1"), idle("chatgpt:2")];
    let mut s = settings(Mode::Daily, &["chatgpt:1"]);
    s.on = false;
    assert!(wake::due(&s, &[], &seen, local(0, 6, 0), hcm()).is_empty());
    s.on = true;
    assert_eq!(
        ids(&wake::due(&s, &[], &seen, local(0, 6, 0), hcm())),
        [("chatgpt:1", Why::Daily, None)]
    );
    let s = settings(Mode::Reset, &["chatgpt:2"]);
    assert_eq!(
        ids(&wake::due(&s, &[], &seen, local(0, 9, 0), hcm())),
        [("chatgpt:2", Why::Reset, None)]
    );
}

#[test]
fn a_daily_wake_up_runs_once_a_day_at_its_time_or_soon_after_a_mac_that_slept() {
    let s = settings(Mode::Daily, &["chatgpt:1", "chatgpt:2"]);
    let seen = [idle("chatgpt:1"), idle("chatgpt:2")];
    assert!(wake::due(&s, &[], &seen, local(0, 5, 59), hcm()).is_empty(), "not before its time");
    let plans = wake::due(&s, &[], &seen, local(0, 6, 0), hcm());
    assert_eq!(ids(&plans), [("chatgpt:1", Why::Daily, None), ("chatgpt:2", Why::Daily, None)]);
    let runs = [
        ran(local(0, 6, 1), "chatgpt:2", Why::Daily, Outcome::Started),
        ran(local(0, 6, 0), "chatgpt:1", Why::Daily, Outcome::Failed),
    ];
    // Done today, failed or not: the next is tomorrow.
    assert!(wake::due(&s, &runs, &seen, local(0, 6, 30), hcm()).is_empty());
    assert!(wake::due(&s, &runs, &seen, local(0, 23, 59), hcm()).is_empty());
    assert_eq!(wake::due(&s, &runs, &seen, local(1, 6, 0), hcm()).len(), 2);
    // A Mac asleep at 06:00 wakes them when it wakes, up to three hours late.
    assert_eq!(wake::due(&s, &[], &seen, local(0, 8, 59), hcm()).len(), 2);
    assert!(wake::due(&s, &[], &seen, local(0, 9, 1), hcm()).is_empty(), "too late: tomorrow");
    assert_eq!(wake::next(&s, &runs, local(0, 6, 30), hcm()), Some(local(1, 6, 0)));
    assert_eq!(wake::next(&s, &[], local(0, 5, 0), hcm()), Some(local(0, 6, 0)));
}

#[test]
fn an_open_window_a_week_used_up_or_a_refused_sign_in_is_recorded_instead_of_run() {
    let s = settings(Mode::Daily, &["chatgpt:1", "chatgpt:2", "chatgpt:3", "chatgpt:4"]);
    let open = Seen { five_used: Some(12.0), five_resets: Some(local(0, 8, 0)), ..idle("chatgpt:1") };
    let week = Seen { week_left: Some(0.0), ..idle("chatgpt:2") };
    let refused = Seen { login: Login::Relogin, ..idle("chatgpt:3") };
    let seen = [open, week, refused, idle("chatgpt:4")];
    assert_eq!(
        ids(&wake::due(&s, &[], &seen, local(0, 6, 0), hcm())),
        [
            ("chatgpt:1", Why::Daily, Some(Outcome::Open)),
            ("chatgpt:2", Why::Daily, Some(Outcome::Limited)),
            ("chatgpt:3", Why::Daily, Some(Outcome::Relogin)),
            ("chatgpt:4", Why::Daily, None),
        ]
    );
    // A window the app opened itself counts as open for five hours, even
    // before a quota read shows it.
    let woke = [ran(local(0, 4, 0), "chatgpt:4", Why::Now, Outcome::Started)];
    let plans = wake::due(&s, &woke, &[idle("chatgpt:4")], local(0, 6, 0), hcm());
    assert_eq!(ids(&plans), [("chatgpt:4", Why::Daily, Some(Outcome::Open))]);
    // An account the app does not know (removed since) is left alone.
    assert!(wake::due(&s, &[], &[], local(0, 6, 0), hcm()).is_empty());
}

#[test]
fn the_reset_mode_wakes_once_a_window_at_most_and_never_loops_even_when_every_wake_up_fails() {
    let s = settings(Mode::Reset, &["chatgpt:1"]);
    let seen = [idle("chatgpt:1")];
    let mut runs: Vec<Run> = Vec::new();
    // A day of the scheduler's 30-second ticks, every wake-up failing.
    let start = local(0, 0, 0);
    let mut t = start;
    while t < start + Duration::days(1) {
        for p in wake::due(&s, &runs, &seen, t, hcm()) {
            assert_eq!(p.skip, None, "the reset mode records nothing it did not run");
            runs.insert(0, ran(t, &p.id, p.why, Outcome::Failed));
        }
        t += Duration::seconds(30);
    }
    assert!((4..=wake::DAILY_CAP).contains(&runs.len()), "{} wake-ups in a day", runs.len());
    for pair in runs.windows(2) {
        assert!(pair[0].at - pair[1].at >= 5 * 3600, "five hours apart at least");
    }
    // A window open (read from the quota) or a week used up: not woken.
    let open = [Seen { five_used: Some(3.0), five_resets: Some(local(0, 9, 0)), ..idle("chatgpt:1") }];
    assert!(wake::due(&s, &[], &open, local(0, 7, 0), hcm()).is_empty());
    let spent = [Seen { week_left: Some(0.0), ..idle("chatgpt:1") }];
    assert!(wake::due(&s, &[], &spent, local(0, 7, 0), hcm()).is_empty());
    // Once the window has reset, it is woken again.
    assert_eq!(wake::due(&s, &[], &open, local(0, 9, 1), hcm()).len(), 1);
}

#[test]
fn keep_alive_wakes_a_waiting_account_whose_sign_in_ran_out_twice_a_day_at_most() {
    let mut s = settings(Mode::Daily, &["chatgpt:1", "chatgpt:2"]);
    s.keep_alive = true;
    let ran_out = Seen { login: Login::Idle, ..idle("chatgpt:1") };
    // The account Codex uses renews its own sign-in: not woken for that.
    let in_use = Seen { login: Login::Idle, waiting: false, ..idle("chatgpt:2") };
    let seen = [ran_out, in_use];
    let plans = wake::due(&s, &[], &seen, local(0, 14, 0), hcm());
    assert_eq!(ids(&plans), [("chatgpt:1", Why::KeepAlive, None)]);
    let runs = [ran(local(0, 14, 0), "chatgpt:1", Why::KeepAlive, Outcome::Failed)];
    assert!(wake::due(&s, &runs, &seen, local(0, 20, 0), hcm()).is_empty());
    assert_eq!(wake::due(&s, &runs, &seen, local(1, 2, 1), hcm()).len(), 1);
    s.keep_alive = false;
    assert!(wake::due(&s, &[], &seen, local(0, 14, 0), hcm()).is_empty());
}

#[test]
fn run_now_wakes_every_chosen_account_but_one_it_cannot_help() {
    let s = settings(Mode::Daily, &["chatgpt:1", "chatgpt:2", "chatgpt:3"]);
    let open = Seen { five_used: Some(12.0), five_resets: Some(local(0, 8, 0)), ..idle("chatgpt:2") };
    let seen = [idle("chatgpt:1"), open, idle("chatgpt:3")];
    let plans = wake::now(&s, &[], &seen, local(0, 7, 0));
    assert_eq!(
        ids(&plans),
        [
            ("chatgpt:1", Why::Now, None),
            ("chatgpt:2", Why::Now, Some(Outcome::Open)),
            ("chatgpt:3", Why::Now, None),
        ]
    );
}

#[test]
fn what_a_wake_up_ended_as_is_read_from_codex_and_its_answer_is_not_kept() {
    let ok = concat!(
        "{\"type\":\"thread.started\",\"thread_id\":\"t-1\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"id\":\"item_0\",\"type\":\"agent_message\",\"text\":\"Hello from the model\"}}\n",
        "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":10,\"output_tokens\":2}}\n",
    );
    assert_eq!(wake::outcome(true, ok, ""), (Outcome::Started, None));

    let refused = "{\"type\":\"turn.failed\",\"error\":{\"message\":\"unexpected status 401 Unauthorized: refresh_token_reused, url: https://chatgpt.com/backend-api/codex/responses, cf-ray: abc, request id: req_1\"}}\n";
    assert_eq!(wake::outcome(false, refused, "").0, Outcome::Relogin);

    let limited = "{\"type\":\"turn.failed\",\"error\":{\"message\":\"You've hit your usage limit. Try again at 11:00.\"}}\n";
    assert_eq!(wake::outcome(false, limited, "").0, Outcome::Limited);

    let other = "{\"type\":\"error\",\"message\":\"stream disconnected before completion, url: https://chatgpt.com/x, request id: req_2\"}\n";
    let (result, detail) = wake::outcome(false, other, "");
    assert_eq!(result, Outcome::Failed);
    assert_eq!(detail.as_deref(), Some("stream disconnected before completion"));

    let (result, detail) = wake::outcome(false, "", "Error: something broke at https://example.com/x\n");
    assert_eq!(result, Outcome::Failed);
    assert_eq!(detail.as_deref(), Some("Error: something broke at"));
    for (_, d) in [wake::outcome(true, ok, ""), wake::outcome(false, ok, "")] {
        assert!(d.unwrap_or_default().find("Hello").is_none(), "the model's answer is never kept");
    }
}

#[test]
fn the_settings_and_the_newest_runs_are_kept_in_the_app_folder() {
    let app = AppDir::new(temp_dir("codex-wake"));
    assert_eq!(Book::load(&app), Book::default());
    assert!(!Book::default().settings.on, "off until the owner turns it on");
    let mut book = Book { settings: settings(Mode::Reset, &["chatgpt:1"]), runs: Vec::new() };
    for i in 0..(wake::KEPT + 5) {
        book.record(ran(local(0, 0, i as i64), "chatgpt:1", Why::Reset, Outcome::Started));
    }
    assert_eq!(book.runs.len(), wake::KEPT);
    assert_eq!(book.runs[0].at, local(0, 0, (wake::KEPT + 4) as i64).unix_timestamp(), "newest first");
    book.save(&app).unwrap();
    assert_eq!(Book::load(&app), book);
    // Times that are not a clock, ids that are not a ChatGPT account: refused.
    assert!(wake::check(&settings(Mode::Daily, &["chatgpt:1"])).is_ok());
    assert!(wake::check(&Settings { at: "25:00".into(), ..settings(Mode::Daily, &[]) }).is_err());
    assert!(wake::check(&Settings { at: "6h".into(), ..settings(Mode::Daily, &[]) }).is_err());
    assert!(wake::check(&settings(Mode::Daily, &["custom"])).is_err());
}
