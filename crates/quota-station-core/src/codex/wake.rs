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
use time::{OffsetDateTime, UtcOffset};

use crate::app_dir::AppDir;

/// The file, in the app's folder.
pub const FILE: &str = "codex/wake.json";
/// The runs kept, newest first.
pub const KEPT: usize = 100;
/// The most wake-ups of one account in 24 hours.
pub const DAILY_CAP: usize = 5;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub settings: Settings,
    #[serde(default)]
    pub runs: Vec<Run>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { on: false, mode: Mode::Daily, at: "06:00".into(), keep_alive: true, ids: Vec::new() }
    }
}

impl Default for Book {
    fn default() -> Self {
        Book { settings: Settings::default(), runs: Vec::new() }
    }
}

impl Book {
    pub fn load(app: &AppDir) -> Book {
        let _ = app;
        todo!()
    }

    pub fn save(&self, app: &AppDir) -> std::io::Result<()> {
        let _ = app;
        todo!()
    }

    /// Adds a run, newest first, keeping [`KEPT`].
    pub fn record(&mut self, run: Run) {
        let _ = run;
        todo!()
    }
}

/// Settings the owner may keep: a clock time, ChatGPT accounts only.
pub fn check(settings: &Settings) -> Result<(), &'static str> {
    let _ = settings;
    todo!()
}

/// The accounts due at `now`, in the order chosen.
pub fn due(
    settings: &Settings,
    runs: &[Run],
    seen: &[Seen],
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Vec<Plan> {
    let _ = (settings, runs, seen, now, offset);
    todo!()
}

/// "Chạy ngay": every chosen account but one a run cannot help.
pub fn now(settings: &Settings, runs: &[Run], seen: &[Seen], now: OffsetDateTime) -> Vec<Plan> {
    let _ = (settings, runs, seen, now);
    todo!()
}

/// When the next daily wake-up is due; `None` in the reset mode or off.
pub fn next(
    settings: &Settings,
    runs: &[Run],
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<OffsetDateTime> {
    let _ = (settings, runs, now, offset);
    todo!()
}

/// What a run ended as, from `codex exec --json`'s events and its error
/// output. The model's answer is never looked at or kept.
pub fn outcome(success: bool, events: &str, stderr: &str) -> (Outcome, Option<String>) {
    let _ = (success, events, stderr);
    todo!()
}
