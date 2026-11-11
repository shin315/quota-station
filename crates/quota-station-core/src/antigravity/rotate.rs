// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity rotation: every three minutes, from its own LaunchAgent
//! ([`LABEL`], `quota-station antigravity rotate`), whether or not the app is
//! open, as the Claude rotation runs. Off until the owner turns it on.
//!
//! One run reads what is due ([`World::read`]: the account in use, and the
//! others whose reading is older than ten minutes), then [`decide`]s: while
//! the account in use has more Gemini left than the threshold (5% unless set
//! otherwise) nothing happens; at or under it, the account with the most
//! Gemini left is switched to, through the app's own switch
//! ([`super::switch`]: the same log, Antigravity.app quit and opened again
//! when it runs), only when its reading is fresh, it has more than the
//! threshold left and at least [`Settings::min_lead`] points more than the
//! account in use, and the last switch (the app's, the command line's or
//! the rotation's: any line of the switch log) is older than the cooldown.
//! When the last switch was undone (the six-hour watch saw the account
//! from before back in use: an Antigravity.app or an `agy` that kept running
//! wrote its own login back), it does not switch again until the owner
//! answers that in the app or switches.
//!
//! Its files, in the app's `antigravity/` folder: `rotation.json` (the
//! settings), `rotation-log.jsonl` (one line per switch it tried) and
//! `rotation-state.json` (its last run, for `rotation status`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::index::{self, AccountIndex, Standing, Statuses};
use super::switch::{self, How, Outcome};
use super::watch;
use crate::app_dir::AppDir;
use crate::claude::agent::{self, AgentError};
use crate::quota::semantics::AGY_SCOPES;
use crate::quota::{iso, AgyAccounts, Response};

/// The rotation's LaunchAgent.
pub const LABEL: &str = "com.shin315.quota-station.agy-rotation";
/// What the LaunchAgent runs, after the program.
pub const ARGS: [&str; 2] = ["antigravity", "rotate"];
pub const SETTINGS_FILE: &str = "rotation.json";
pub const LOG_FILE: &str = "rotation-log.jsonl";
pub const STATE_FILE: &str = "rotation-state.json";
/// The log keeps this many switches.
const LOG_KEEP: usize = 200;
/// Why turning it on is refused where no LaunchAgent can run it.
pub const UNSUPPORTED: &str =
    "the Antigravity rotation runs from a LaunchAgent: on macOS only for now";

/// What the owner may set; a file without a key keeps its default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Gemini left in the account in use, in percent, at or under which it
    /// switches.
    pub threshold: f64,
    /// The least time between two switches, in minutes.
    pub cooldown_minutes: u32,
    /// How many points more Gemini the account switched to must have.
    pub min_lead: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { threshold: 5.0, cooldown_minutes: 30, min_lead: 10.0 }
    }
}

pub fn settings_path(app: &AppDir) -> PathBuf {
    index::dir(app).join(SETTINGS_FILE)
}

pub fn log_path(app: &AppDir) -> PathBuf {
    index::dir(app).join(LOG_FILE)
}

pub fn state_path(app: &AppDir) -> PathBuf {
    index::dir(app).join(STATE_FILE)
}

/// The settings; none yet, or a file that cannot be read, gives the defaults.
pub fn load_settings(app: &AppDir) -> Settings {
    app.read_to_string(&settings_path(app))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_settings(app: &AppDir, settings: &Settings) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    app.write_private(&settings_path(app), &text)
}

/// One account as the rotation weighs it.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    /// Its place in the app's list, from 1.
    pub number: u32,
    pub active: bool,
    /// Gemini left, when its reading says.
    pub gemini_left: Option<f64>,
    /// Its reading is at most 15 minutes old.
    pub fresh: bool,
    /// Needs a new login, a verification, or was refused: not switched to.
    pub blocked: bool,
}

/// What one run decided.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Decision {
    /// The app lists no accounts here, or one only, or cannot list them.
    NoAccounts {
        reason: String,
    },
    /// Which account Antigravity is on could not be told.
    UnknownActive,
    /// The account in use has no fresh Gemini reading.
    ActiveUnread {
        active: u32,
    },
    /// The account in use has more Gemini left than the threshold.
    Enough {
        active: u32,
        left: f64,
    },
    /// At or under the threshold, and no other account is clearly better.
    NoBetter {
        active: u32,
        left: f64,
        best: Option<u32>,
        best_left: Option<f64>,
    },
    /// It would switch, but the last switch is too recent.
    Cooldown {
        active: u32,
        left: f64,
        to: u32,
        to_left: f64,
        until: i64,
    },
    /// It would switch, but the last switch was undone (the six-hour watch,
    /// [`super::watch`]): an Antigravity or agy that kept running wrote its
    /// old login back, and would undo this one too. It waits until the
    /// owner answers the overwrite in the app, or switches.
    Overwritten {
        active: u32,
        overwrites: u32,
    },
    Switch {
        from: u32,
        to: u32,
        from_left: f64,
        to_left: f64,
    },
}

/// The rotation's rule. `last_switch` is the last line of the switch log, in
/// Unix seconds, as `now`.
pub fn decide(accounts: &[Seen], last_switch: Option<i64>, now: i64, s: &Settings) -> Decision {
    if accounts.len() < 2 {
        let reason = if accounts.is_empty() { "none" } else { "one" };
        return Decision::NoAccounts { reason: reason.into() };
    }
    let Some(active) = accounts.iter().find(|a| a.active) else {
        return Decision::UnknownActive;
    };
    let Some(left) = active.gemini_left.filter(|_| active.fresh) else {
        return Decision::ActiveUnread { active: active.number };
    };
    if left > s.threshold {
        return Decision::Enough { active: active.number, left };
    }
    let best = accounts
        .iter()
        .filter(|a| !a.active && !a.blocked && a.fresh)
        .filter_map(|a| a.gemini_left.map(|l| (a.number, l)))
        // The most left; of two alike, the first in the list.
        .fold(None, |best: Option<(u32, f64)>, (n, l)| match best {
            Some((_, b)) if b >= l => best,
            _ => Some((n, l)),
        });
    let Some((to, to_left)) = best.filter(|(_, l)| *l > s.threshold && *l >= left + s.min_lead)
    else {
        return Decision::NoBetter {
            active: active.number,
            left,
            best: best.map(|b| b.0),
            best_left: best.map(|b| b.1),
        };
    };
    let until = last_switch.map(|at| at + i64::from(s.cooldown_minutes) * 60);
    if let Some(until) = until.filter(|u| *u > now) {
        return Decision::Cooldown { active: active.number, left, to, to_left, until };
    }
    Decision::Switch { from: active.number, to, from_left: left, to_left }
}

/// The accounts of the Antigravity row as the rotation weighs them.
pub fn seen(list: &AgyAccounts, index: &AccountIndex, statuses: &Statuses) -> Vec<Seen> {
    list.accounts
        .iter()
        .map(|a| {
            let id = index.accounts.get((a.account as usize).wrapping_sub(1)).map(|s| &s.id);
            Seen {
                number: a.account,
                active: a.active,
                gemini_left: a
                    .scopes
                    .iter()
                    .find(|s| s.scope == AGY_SCOPES[0] && s.status == "known")
                    .and_then(|s| s.effective_percent_remaining),
                fresh: a.status == "fresh",
                blocked: id.is_none()
                    || id.and_then(|id| statuses.accounts.get(id)).is_some_and(|s| {
                        matches!(
                            s,
                            Standing::NeedsLogin { .. }
                                | Standing::Forbidden { .. }
                                | Standing::NeedsVerification { .. }
                        )
                    }),
            }
        })
        .collect()
}

/// How fresh a read must be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fresh {
    /// The last readings, without asking Google; agy is asked only when the
    /// app's last reading of it is older than two minutes.
    Cached,
    /// What the rotation reads: the account in use, and the others whose
    /// reading is older than ten minutes.
    Due,
    /// Every account, each at most once a minute.
    All,
}

/// What a run reads and switches with: the system's
/// ([`super::cli::System`]), or a test's.
pub trait World {
    /// The Antigravity row, with the app's list on it.
    fn read(&self, app: &AppDir, fresh: Fresh, now: OffsetDateTime) -> Response;
    /// Switches to the account `id`; `from` is the one in use. `how`, when
    /// given, is the button the owner pressed; else a running
    /// Antigravity.app is quit and opened again.
    fn switch(&self, app: &AppDir, id: &str, from: Option<String>, how: Option<How>) -> Outcome;
}

/// One run: what it decided and, when it switched, how that ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub at: i64,
    #[serde(flatten)]
    pub decision: Decision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    pub dry_run: bool,
}

impl Run {
    /// One line for a terminal.
    pub fn print(&self) -> String {
        let pct = |l: f64| format!("{l:.0}%");
        let what = match &self.decision {
            Decision::NoAccounts { reason } => format!("nothing to rotate ({reason} account)"),
            Decision::UnknownActive => "the account in use is not known: no switch".into(),
            Decision::ActiveUnread { active } => {
                format!("account {active} has no fresh Gemini reading: no switch")
            }
            Decision::Enough { active, left } => {
                format!("account {active} has {} of Gemini left: no switch", pct(*left))
            }
            Decision::NoBetter { active, left, best, best_left } => match (best, best_left) {
                (Some(b), Some(l)) => format!(
                    "account {active} has {} left; the best other, {b}, has {}: no switch",
                    pct(*left),
                    pct(*l)
                ),
                _ => format!("account {active} has {} left; no other has a reading", pct(*left)),
            },
            Decision::Overwritten { active, overwrites } => format!(
                "account {active} is low, but the last switch was undone {overwrites} time(s) \
                 by an Antigravity or agy that kept running: no switch until it is answered \
                 in the app"
            ),
            Decision::Cooldown { active, to, until, .. } => format!(
                "account {active} is low, {to} is better, but the last switch was too recent: \
                 waiting until {}",
                at_text(*until)
            ),
            Decision::Switch { from, to, from_left, to_left } => {
                let head = format!(
                    "account {from} has {} of Gemini left: {} account {to} ({})",
                    pct(*from_left),
                    if self.dry_run { "would switch to" } else { "switching to" },
                    pct(*to_left)
                );
                match &self.outcome {
                    Some(o) => format!("{head}: {}", outcome_text(o)),
                    None => head,
                }
            }
        };
        format!("{what}\n")
    }
}

/// An outcome in words.
pub fn outcome_text(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Switched { number, .. } => format!("switched to account {number}"),
        Outcome::Unconfirmed { number, active, .. } => match active {
            Some(a) => {
                format!("account {number}'s login was written, but account {a} shows in use")
            }
            None => format!(
                "account {number}'s login was written; which one is in use is not known yet"
            ),
        },
        Outcome::Failed { number, error, detail, .. } => {
            let detail = if detail.is_empty() { String::new() } else { format!(": {detail}") };
            format!("switching to account {number} failed ({error}{detail})")
        }
    }
}

fn at_text(at: i64) -> String {
    OffsetDateTime::from_unix_timestamp(at).map(iso).unwrap_or_else(|_| at.to_string())
}

/// One run of the rotation at `now`. A dry run reads, decides and says what
/// it would do; it switches and writes nothing.
pub fn run(app: &AppDir, world: &dyn World, now: OffsetDateTime, dry_run: bool) -> Run {
    let settings = load_settings(app);
    let at = now.unix_timestamp();
    let decision = match index::load(app) {
        Err(_) => Decision::NoAccounts { reason: "unreadable".into() },
        Ok(index) if index.accounts.len() < 2 => {
            let reason = if index.accounts.is_empty() { "none" } else { "one" };
            Decision::NoAccounts { reason: reason.into() }
        }
        Ok(index) => {
            let report = world.read(app, Fresh::Due, now);
            let statuses = index::load_statuses(app);
            match report.primary("agy").and_then(|r| r.agy_accounts.as_ref()) {
                Some(list) if list.status == "ok" => {
                    let last = switch::read_log(app).last().map(|l| l.at);
                    // The watch hears of this reading, as of the app's.
                    if !dry_run {
                        if let Some(id) = switch::shown_active_id(&report, &index) {
                            watch::observe(app, Some(&id), at);
                        }
                    }
                    let undone = watch::load(app).filter(|w| w.alert().is_some());
                    match (decide(&seen(list, &index, &statuses), last, at, &settings), undone) {
                        (
                            Decision::Switch { from: active, .. }
                            | Decision::Cooldown { active, .. },
                            Some(w),
                        ) => Decision::Overwritten { active, overwrites: w.overwrites },
                        (decision, _) => decision,
                    }
                }
                _ => Decision::NoAccounts { reason: "unlisted".into() },
            }
        }
    };
    let mut run = Run { at, decision, outcome: None, dry_run };
    if dry_run {
        return run;
    }
    if let Decision::Switch { from, to, from_left, to_left } = run.decision {
        let index = index::load(app).unwrap_or_default();
        let id_of = |n: u32| index.accounts.get(n as usize - 1).map(|a| a.id.clone());
        let outcome = match id_of(to) {
            Some(id) => world.switch(app, &id, id_of(from), None),
            None => Outcome::Failed {
                number: to as usize,
                error: "unknownAccount",
                detail: String::new(),
                reopened: None,
            },
        };
        let _ = append_log(
            app,
            &LogLine {
                at,
                from,
                to,
                from_left,
                to_left,
                threshold: settings.threshold,
                result: result_word(&outcome).into(),
                error: match &outcome {
                    Outcome::Failed { error, .. } => Some((*error).to_string()),
                    _ => None,
                },
            },
        );
        run.outcome = Some(outcome);
    }
    let _ = write_state(app, &run);
    run
}

fn result_word(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Switched { .. } => "switched",
        Outcome::Unconfirmed { .. } => "unconfirmed",
        Outcome::Failed { .. } => "failed",
    }
}

/// One line of `rotation-log.jsonl`: account numbers, never an email.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// Unix seconds.
    pub at: i64,
    pub from: u32,
    pub to: u32,
    pub from_left: f64,
    pub to_left: f64,
    pub threshold: f64,
    /// `switched`, `unconfirmed` or `failed`.
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The log's lines, oldest first; lines it cannot read are skipped.
pub fn read_log(app: &AppDir) -> Vec<LogLine> {
    let text = app.read_to_string(&log_path(app)).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

fn append_log(app: &AppDir, line: &LogLine) -> std::io::Result<()> {
    let text = app.read_to_string(&log_path(app)).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    lines.push(serde_json::to_string(line).map_err(std::io::Error::other)?);
    let skip = lines.len().saturating_sub(LOG_KEEP);
    let mut out = lines[skip..].join("\n");
    out.push('\n');
    app.write_private(&log_path(app), &out)
}

fn write_state(app: &AppDir, run: &Run) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(run).map_err(std::io::Error::other)?;
    app.write_private(&state_path(app), &text)
}

/// The last run, as `rotation-state.json` keeps it.
pub fn last_run(app: &AppDir) -> Option<serde_json::Value> {
    let text = app.read_to_string(&state_path(app)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The rotation's LaunchAgent through `launchctl` on macOS; on Windows it
/// is never on and cannot be turned on.
#[derive(Debug, Clone)]
pub struct Schedule {
    pub launchctl: PathBuf,
}

impl Schedule {
    pub fn system() -> Schedule {
        Schedule { launchctl: PathBuf::from(crate::switcher::LAUNCHCTL) }
    }

    /// Whether the LaunchAgent is loaded; `None` when that cannot be told.
    pub fn loaded(&self) -> Option<bool> {
        if !cfg!(unix) {
            return Some(false);
        }
        crate::switcher::service_loaded(&self.launchctl, LABEL)
    }

    /// Writes and loads the LaunchAgent that runs `<exe> antigravity rotate`.
    pub fn install(&self, home: &Path, exe: &Path) -> Result<(), AgentError> {
        if !cfg!(unix) {
            return Err(AgentError::Io(UNSUPPORTED.into()));
        }
        agent::install_job(&self.launchctl, home, LABEL, exe, &ARGS)
    }

    pub fn remove(&self, home: &Path) -> Result<(), AgentError> {
        if !cfg!(unix) {
            return Ok(());
        }
        agent::remove_job(&self.launchctl, home, LABEL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(number: u32, active: bool, left: Option<f64>) -> Seen {
        Seen { number, active, gemini_left: left, fresh: true, blocked: false }
    }

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn above_the_threshold_nothing_happens() {
        let s = Settings::default();
        let list = [acc(1, true, Some(5.5)), acc(2, false, Some(90.0))];
        assert_eq!(decide(&list, None, NOW, &s), Decision::Enough { active: 1, left: 5.5 });
    }

    #[test]
    fn at_the_threshold_it_switches_to_the_account_with_the_most_gemini() {
        let s = Settings::default();
        let list = [acc(1, false, Some(60.0)), acc(2, true, Some(5.0)), acc(3, false, Some(80.0))];
        assert_eq!(
            decide(&list, None, NOW, &s),
            Decision::Switch { from: 2, to: 3, from_left: 5.0, to_left: 80.0 }
        );
    }

    #[test]
    fn of_two_alike_the_first_in_the_list_is_taken() {
        let s = Settings::default();
        let list = [acc(1, true, Some(0.0)), acc(2, false, Some(70.0)), acc(3, false, Some(70.0))];
        assert!(matches!(decide(&list, None, NOW, &s), Decision::Switch { to: 2, .. }));
    }

    #[test]
    fn the_threshold_can_be_set() {
        let s = Settings { threshold: 20.0, ..Settings::default() };
        let list = [acc(1, true, Some(18.0)), acc(2, false, Some(90.0))];
        assert!(matches!(decide(&list, None, NOW, &s), Decision::Switch { from: 1, to: 2, .. }));
        let s = Settings { threshold: 10.0, ..Settings::default() };
        assert!(matches!(decide(&list, None, NOW, &s), Decision::Enough { .. }));
    }

    #[test]
    fn no_switch_when_no_other_account_is_clearly_better() {
        let s = Settings::default();
        // The others are spent too, or barely better.
        let spent = [acc(1, true, Some(2.0)), acc(2, false, Some(4.0)), acc(3, false, Some(0.0))];
        assert_eq!(
            decide(&spent, None, NOW, &s),
            Decision::NoBetter { active: 1, left: 2.0, best: Some(2), best_left: Some(4.0) }
        );
        let close = [acc(1, true, Some(3.0)), acc(2, false, Some(12.0))];
        assert!(matches!(decide(&close, None, NOW, &s), Decision::NoBetter { best: Some(2), .. }));
        // Better by the lead, and over the threshold: it goes.
        let lead = [acc(1, true, Some(3.0)), acc(2, false, Some(13.0))];
        assert!(matches!(decide(&lead, None, NOW, &s), Decision::Switch { to: 2, .. }));
    }

    #[test]
    fn stale_unread_and_blocked_accounts_are_never_switched_to() {
        let s = Settings::default();
        let mut stale = acc(2, false, Some(90.0));
        stale.fresh = false;
        let mut blocked = acc(3, false, Some(95.0));
        blocked.blocked = true;
        let unread = acc(4, false, None);
        let list = [acc(1, true, Some(1.0)), stale, blocked, unread];
        assert_eq!(
            decide(&list, None, NOW, &s),
            Decision::NoBetter { active: 1, left: 1.0, best: None, best_left: None }
        );
    }

    #[test]
    fn nothing_is_decided_without_a_fresh_reading_of_the_account_in_use() {
        let s = Settings::default();
        let mut active = acc(1, true, Some(1.0));
        active.fresh = false;
        let list = [active, acc(2, false, Some(90.0))];
        assert_eq!(decide(&list, None, NOW, &s), Decision::ActiveUnread { active: 1 });
        let list = [acc(1, false, Some(1.0)), acc(2, false, Some(90.0))];
        assert_eq!(decide(&list, None, NOW, &s), Decision::UnknownActive);
        assert_eq!(
            decide(&[acc(1, true, Some(0.0))], None, NOW, &s),
            Decision::NoAccounts { reason: "one".into() }
        );
    }

    #[test]
    fn a_recent_switch_holds_the_next_one_until_the_cooldown_ends() {
        let s = Settings { cooldown_minutes: 30, ..Settings::default() };
        let list = [acc(1, true, Some(1.0)), acc(2, false, Some(90.0))];
        let last = NOW - 29 * 60;
        assert_eq!(
            decide(&list, Some(last), NOW, &s),
            Decision::Cooldown { active: 1, left: 1.0, to: 2, to_left: 90.0, until: last + 1800 }
        );
        assert!(matches!(decide(&list, Some(NOW - 30 * 60), NOW, &s), Decision::Switch { .. }));
        let none = Settings { cooldown_minutes: 0, ..s };
        assert!(matches!(decide(&list, Some(NOW), NOW, &none), Decision::Switch { .. }));
    }

    #[test]
    fn settings_missing_keys_keep_their_defaults() {
        let s: Settings = serde_json::from_str(r#"{"threshold": 12}"#).unwrap();
        assert_eq!(s, Settings { threshold: 12.0, ..Settings::default() });
    }

    #[test]
    fn the_launch_agent_runs_the_antigravity_rotation() {
        let plist = agent::job_plist(LABEL, Path::new("/A & B/quota-station"), &ARGS);
        assert!(plist.contains(&format!("<string>{LABEL}</string>")), "{plist}");
        assert!(plist.contains(
            "<string>/A &amp; B/quota-station</string>\n    <string>antigravity</string>\n    \
             <string>rotate</string>\n  </array>"
        ));
    }
}
