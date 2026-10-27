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
//! holds the account off for five hours, and a day has a cap. A day may hold
//! several wake times, five hours apart at least; a time that finds the
//! account held off is recorded as skipped, never moved or made up later.

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
    /// Each day at each of `times`.
    Daily,
    /// As soon as an account's five-hour window is not open.
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "Kept")]
pub struct Settings {
    pub on: bool,
    pub mode: Mode,
    /// `07:00`, `12:00`: the Mac's times, earliest first.
    pub times: Vec<String>,
    /// Also wake a waiting account whose sign-in ran out.
    pub keep_alive: bool,
    /// `chatgpt:<n>`.
    pub ids: Vec<String>,
}

/// The settings as `codex/wake.json` holds them: one `at` before the day
/// held several times.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    on: bool,
    mode: Mode,
    #[serde(default)]
    times: Vec<String>,
    #[serde(default)]
    at: Option<String>,
    keep_alive: bool,
    ids: Vec<String>,
}

impl From<Kept> for Settings {
    fn from(k: Kept) -> Self {
        let times = match (k.times, k.at) {
            (times, _) if !times.is_empty() => times,
            (_, Some(at)) => vec![at],
            _ => Settings::default().times,
        };
        Settings { on: k.on, mode: k.mode, times, keep_alive: k.keep_alive, ids: k.ids }
    }
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
    /// Run less than five hours before: nothing was sent.
    Soon,
    /// Run five times in 24 hours already: nothing was sent.
    Capped,
}

impl Outcome {
    /// Only recorded, never the end of a run: nothing was sent.
    fn skipped(self) -> bool {
        matches!(self, Outcome::Open | Outcome::Soon | Outcome::Capped)
    }
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
            times: vec!["06:00".into()],
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

/// The wake times as kept, earliest first. Refused: `at`, a time that is
/// not a clock; `same`, a time twice; `empty`, no time; `gap`, two times
/// less than five hours apart, over midnight too (so four at most).
pub fn times(list: &[String]) -> Result<Vec<String>, &'static str> {
    let mut all = list.iter().map(|at| minutes(at).ok_or("at")).collect::<Result<Vec<_>, _>>()?;
    all.sort_unstable();
    if all.is_empty() {
        return Err("empty");
    }
    if all.windows(2).any(|w| w[0] == w[1]) {
        return Err("same");
    }
    let apart = |a: i64, b: i64| (b - a) * 60 >= WINDOW;
    if all.windows(2).any(|w| !apart(w[0], w[1]))
        || (all.len() > 1 && !apart(all[all.len() - 1], all[0] + DAY / 60))
    {
        return Err("gap");
    }
    Ok(all.into_iter().map(|m| format!("{:02}:{:02}", m / 60, m % 60)).collect())
}

/// Settings the owner may keep: wake times as [`times`] keeps them,
/// ChatGPT accounts only.
pub fn check(settings: &Settings) -> Result<(), &'static str> {
    if times(&settings.times)? != settings.times {
        return Err("at");
    }
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

/// The latest wake time at or before `now`: today's, or yesterday's.
fn last_slot(settings: &Settings, now: OffsetDateTime, offset: UtcOffset) -> Option<i64> {
    let t = now.unix_timestamp();
    settings
        .times
        .iter()
        .filter_map(|at| minutes(at))
        .map(|m| match slot(now, offset, m) {
            s if s <= t => s,
            _ => slot(now - Duration::days(1), offset, m),
        })
        .max()
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

/// Why no run of `id` may go now: [`blocked`], or (`Soon`) one ran less
/// than five hours before, or (`Capped`) five ran in 24 hours. Only runs
/// that sent something count, failed ones too.
fn held(seen: &Seen, runs: &[Run], t: i64) -> Option<Outcome> {
    blocked(seen, runs, t).or_else(|| {
        let sent: Vec<i64> = runs
            .iter()
            .filter(|r| r.id == seen.id && !r.result.skipped())
            .map(|r| t - r.at)
            .collect();
        if sent.iter().any(|d| (0..WINDOW).contains(d)) {
            Some(Outcome::Soon)
        } else if sent.iter().filter(|d| (0..DAY).contains(*d)).count() >= DAILY_CAP {
            Some(Outcome::Capped)
        } else {
            None
        }
    })
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
    // Only the latest time counts: one passed while the Mac slept is not made up.
    let latest = last_slot(settings, now, offset);
    let mut out = Vec::new();
    for id in &settings.ids {
        // An account removed since it was chosen is left alone.
        let Some(seen) = seen.iter().find(|s| &s.id == id) else { continue };
        let mine: Vec<&Run> = runs.iter().filter(|r| &r.id == id).collect();
        let since_sent = mine.iter().filter(|r| !r.result.skipped()).map(|r| t - r.at).min();
        let plan = |why, skip| Some(Plan { id: id.clone(), why, skip });
        let mut found = match settings.mode {
            Mode::Daily => match latest {
                Some(at)
                    if (0..=CATCH_UP).contains(&(t - at))
                        && !mine.iter().any(|r| r.why == Why::Daily && r.at >= at) =>
                {
                    plan(Why::Daily, held(seen, runs, t))
                }
                _ => None,
            },
            Mode::Reset => match held(seen, runs, t) {
                None => plan(Why::Reset, None),
                Some(_) => None,
            },
        };
        // The account Codex uses renews its own sign-in.
        if found.is_none()
            && settings.keep_alive
            && seen.waiting
            && seen.login == Login::Idle
            && since_sent.is_none_or(|d| d >= KEEP_ALIVE_GAP)
            && held(seen, runs, t).is_none()
        {
            found = plan(Why::KeepAlive, None);
        }
        out.extend(found);
    }
    out
}

/// "Chạy ngay": every chosen account but one a run cannot help, or one run
/// five times in 24 hours. It may follow a failed run at once, so a fix
/// can be tried.
pub fn now(settings: &Settings, runs: &[Run], seen: &[Seen], now: OffsetDateTime) -> Vec<Plan> {
    let t = now.unix_timestamp();
    settings
        .ids
        .iter()
        .filter_map(|id| {
            let seen = seen.iter().find(|s| &s.id == id)?;
            let skip = blocked(seen, runs, t).or(match held(seen, runs, t) {
                Some(Outcome::Capped) => Some(Outcome::Capped),
                _ => None,
            });
            Some(Plan { id: id.clone(), why: Why::Now, skip })
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
    let t = now.unix_timestamp();
    // A time passed whose accounts are still being woken.
    if let Some(at) = last_slot(settings, now, offset) {
        let done = settings
            .ids
            .iter()
            .all(|id| runs.iter().any(|r| &r.id == id && r.why == Why::Daily && r.at >= at));
        if t - at <= CATCH_UP && !done {
            return OffsetDateTime::from_unix_timestamp(at).ok();
        }
    }
    let next = settings
        .times
        .iter()
        .filter_map(|at| minutes(at))
        .map(|m| match slot(now, offset, m) {
            s if s > t => s,
            s => s + DAY,
        })
        .min()?;
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
