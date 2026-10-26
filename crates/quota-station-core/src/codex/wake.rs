// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Tự đánh thức tài khoản ChatGPT": a short message sent with Codex itself
//! (`codex exec`) in each chosen account's own folder, so its five-hour
//! window starts early, and so a sign-in that ran out while it waited is
//! renewed by Codex. The app holds no token for it.
//!
//! This module only decides: which account is due and why, what a run
//! ended as, and the settings and the runs kept in `codex/wake.json`. It
//! never loops: a window is woken once at most, every run (failed ones too)
//! holds the account off for five hours, and a day has a cap.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Duration, OffsetDateTime, Time, UtcOffset};

use super::accounts::{text, CHATGPT_ID};
use crate::app_dir::AppDir;

/// The file, in the app's folder.
pub const FILE: &str = "codex/wake.json";
/// The runs kept, newest first.
pub const KEPT: usize = 100;
/// The most wake-ups of one account in 24 hours.
pub const DAILY_CAP: usize = 5;
/// A five-hour window; also the least time between two wake-ups of one account.
const WINDOW: i64 = 5 * 3600;
const DAY: i64 = 86_400;
/// A Mac asleep at the daily time wakes the accounts up to this late.
const CATCH_UP: i64 = 3 * 3600;
/// The least time between two keep-alive wake-ups of one account.
const KEEP_ALIVE_GAP: i64 = 12 * 3600;
/// The most accounts kept in the settings.
const IDS_MAX: usize = 50;
/// The longest error kept with a run.
const DETAIL_MAX: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// Each day at `at`.
    Daily,
    /// As soon as an account's five-hour window is not open.
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub on: bool,
    pub mode: Mode,
    /// `06:00`, the Mac's time.
    pub at: String,
    /// Also wake a waiting account whose sign-in ran out.
    pub keep_alive: bool,
    /// `chatgpt:<n>`.
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Why {
    Daily,
    Reset,
    KeepAlive,
    Now,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// A five-hour window opened.
    Started,
    /// One was open already: nothing was sent.
    Open,
    /// ChatGPT refused the sign-in.
    Relogin,
    /// No quota left this week.
    Limited,
    /// Codex ended with another error.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Unix seconds.
    pub at: i64,
    pub id: String,
    pub why: Why,
    pub result: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// How an account's sign-in did at the last quota read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Login {
    Ok,
    /// Its token ran out while it waited in its folder.
    Idle,
    /// ChatGPT refused it, or it is gone.
    Relogin,
}

/// What the app knows of one account now.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub id: String,
    pub login: Login,
    /// Its sign-in waits in its own folder (Codex does not use it).
    pub waiting: bool,
    /// The five-hour window, percent used, and when it resets.
    pub five_used: Option<f64>,
    pub five_resets: Option<OffsetDateTime>,
    /// The week's percent left.
    pub week_left: Option<f64>,
}

/// One account due now: woken, or (`skip`) recorded as is without a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub id: String,
    pub why: Why,
    pub skip: Option<Outcome>,
}

/// The settings and the runs: `codex/wake.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub settings: Settings,
    #[serde(default)]
    pub runs: Vec<Run>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            on: false,
            mode: Mode::Daily,
            at: "06:00".into(),
            keep_alive: true,
            ids: Vec::new(),
        }
    }
}

impl Book {
    /// The book kept; off and empty when there is none or it cannot be read.
    pub fn load(app: &AppDir) -> Book {
        app.read_to_string(&app.path().join(FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, app: &AppDir) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        app.write_private(&app.path().join(FILE), &(text + "\n"))
    }

    /// Adds a run, newest first, keeping [`KEPT`].
    pub fn record(&mut self, run: Run) {
        self.runs.insert(0, run);
        self.runs.truncate(KEPT);
    }
}

/// Settings the owner may keep: a clock time, ChatGPT accounts only.
pub fn check(settings: &Settings) -> Result<(), &'static str> {
    minutes(&settings.at).ok_or("at")?;
    let chatgpt =
        |id: &String| id.strip_prefix(CHATGPT_ID).is_some_and(|n| n.parse::<u32>().is_ok());
    if settings.ids.len() > IDS_MAX || !settings.ids.iter().all(chatgpt) {
        return Err("ids");
    }
    Ok(())
}

/// `06:00` → 360.
fn minutes(at: &str) -> Option<i64> {
    let (h, m) = at.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Today's daily time at `now`, the Mac's day.
fn slot(now: OffsetDateTime, offset: UtcOffset, minutes: i64) -> i64 {
    let day = now.to_offset(offset).replace_time(Time::MIDNIGHT);
    (day + Duration::minutes(minutes)).unix_timestamp()
}

/// Why a run cannot help `seen` now: its sign-in was refused, its week is
/// used up, or its window is open (read, or opened by a run of the app's
/// less than five hours ago).
fn blocked(seen: &Seen, runs: &[Run], t: i64) -> Option<Outcome> {
    let read = seen.five_used.is_some_and(|u| u > 0.0)
        && seen.five_resets.is_some_and(|r| r.unix_timestamp() > t);
    let woke = runs.iter().any(|r| {
        r.id == seen.id && r.result == Outcome::Started && (0..WINDOW).contains(&(t - r.at))
    });
    if seen.login == Login::Relogin {
        Some(Outcome::Relogin)
    } else if seen.week_left.is_some_and(|l| l <= 0.0) {
        Some(Outcome::Limited)
    } else if read || woke {
        Some(Outcome::Open)
    } else {
        None
    }
}

/// The accounts due at `now`, in the order chosen.
pub fn due(
    settings: &Settings,
    runs: &[Run],
    seen: &[Seen],
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Vec<Plan> {
    if !settings.on {
        return Vec::new();
    }
    let t = now.unix_timestamp();
    let today = minutes(&settings.at).map(|m| slot(now, offset, m));
    let mut out = Vec::new();
    for id in &settings.ids {
        // An account removed since it was chosen is left alone.
        let Some(seen) = seen.iter().find(|s| &s.id == id) else { continue };
        let mine: Vec<&Run> = runs.iter().filter(|r| &r.id == id).collect();
        let since_last = mine.iter().map(|r| t - r.at).min();
        let today_runs = mine.iter().filter(|r| (0..DAY).contains(&(t - r.at))).count();
        let plan = |why, skip| Some(Plan { id: id.clone(), why, skip });
        let mut found = match settings.mode {
            Mode::Daily => match today {
                Some(at)
                    if (0..=CATCH_UP).contains(&(t - at))
                        && !mine.iter().any(|r| r.why == Why::Daily && r.at >= at) =>
                {
                    plan(Why::Daily, blocked(seen, runs, t))
                }
                _ => None,
            },
            Mode::Reset => {
                let rested = since_last.is_none_or(|d| d >= WINDOW);
                if blocked(seen, runs, t).is_none() && rested && today_runs < DAILY_CAP {
                    plan(Why::Reset, None)
                } else {
                    None
                }
            }
        };
        // The account Codex uses renews its own sign-in.
        if found.is_none()
            && settings.keep_alive
            && seen.waiting
            && seen.login == Login::Idle
            && since_last.is_none_or(|d| d >= KEEP_ALIVE_GAP)
            && today_runs < DAILY_CAP
        {
            found = plan(Why::KeepAlive, None);
        }
        out.extend(found);
    }
    out
}

/// "Chạy ngay": every chosen account but one a run cannot help.
pub fn now(settings: &Settings, runs: &[Run], seen: &[Seen], now: OffsetDateTime) -> Vec<Plan> {
    let t = now.unix_timestamp();
    settings
        .ids
        .iter()
        .filter_map(|id| {
            let seen = seen.iter().find(|s| &s.id == id)?;
            Some(Plan { id: id.clone(), why: Why::Now, skip: blocked(seen, runs, t) })
        })
        .collect()
}

/// When the next daily wake-up is due; `None` in the reset mode or off.
pub fn next(
    settings: &Settings,
    runs: &[Run],
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<OffsetDateTime> {
    if !settings.on || settings.mode != Mode::Daily {
        return None;
    }
    let at = slot(now, offset, minutes(&settings.at)?);
    let t = now.unix_timestamp();
    let done = settings
        .ids
        .iter()
        .all(|id| runs.iter().any(|r| &r.id == id && r.why == Why::Daily && r.at >= at));
    let next = if t < at || (t - at <= CATCH_UP && !done) { at } else { at + DAY };
    OffsetDateTime::from_unix_timestamp(next).ok()
}

/// What a run ended as, from `codex exec --json`'s events and its error
/// output. The model's answer is never looked at or kept.
pub fn outcome(success: bool, events: &str, stderr: &str) -> (Outcome, Option<String>) {
    let mut completed = false;
    let mut error = None;
    for line in events.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        match v.get("type").and_then(Value::as_str) {
            Some("turn.completed") => completed = true,
            Some("turn.failed") => error = text(v.pointer("/error/message")).or(error),
            Some("error") => error = text(v.get("message")).or(error),
            _ => {}
        }
    }
    if completed || (success && error.is_none()) {
        return (Outcome::Started, None);
    }
    let said = error
        .or_else(|| stderr.lines().map(str::trim).rfind(|l| !l.is_empty()).map(str::to_string))
        .unwrap_or_default();
    let message = plain(&said);
    let low = message.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| low.contains(w));
    let result = if has(&["usage limit", "usage_limit", "rate limit", "rate_limit", "429"]) {
        Outcome::Limited
    } else if has(&[
        "401",
        "unauthorized",
        "refresh_token",
        "token_expired",
        "sign in again",
        "log in again",
        "login again",
    ]) {
        Outcome::Relogin
    } else {
        Outcome::Failed
    };
    let detail = (result == Outcome::Failed && !message.is_empty()).then_some(message);
    (result, detail)
}

/// An error as said, without its address, request id or any link.
fn plain(message: &str) -> String {
    let mut m = message;
    for cut in [", url:", " url:", ", cf-ray:", ", request id:"] {
        if let Some(i) = m.find(cut) {
            m = &m[..i];
        }
    }
    let words: Vec<&str> = m
        .split_whitespace()
        .filter(|w| !w.starts_with("http://") && !w.starts_with("https://"))
        .collect();
    let out = words.join(" ");
    out.trim_end_matches([',', ';']).chars().take(DETAIL_MAX).collect()
}
