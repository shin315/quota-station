// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the Thống kê tab shows for a period: the totals, a column per hour
//! (Hôm nay) or per day (7 ngày, 30 ngày) on this Mac's clock, and the
//! counts by agent, model, Claude account and project, with what they would
//! cost at the API prices ([`super::prices`]).
//!
//! The Claude account is approximate: each quarter hour goes to the account
//! the switch history says was in use at its middle ([`Timeline`]).

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use time::{Date, Duration, OffsetDateTime, Time, UtcOffset};

use super::index::{self, Index, Skipped, BUCKET};
use super::{prices, Agent, Tokens};
use crate::history::{App, Cause, Entry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Period {
    /// Hôm nay, by hour.
    Today,
    /// 7 ngày, by day, today the last.
    Week,
    /// 30 ngày.
    Month,
}

impl Period {
    fn days(self) -> i64 {
        match self {
            Period::Today => 1,
            Period::Week => 7,
            Period::Month => 30,
        }
    }
}

/// One column: an hour or a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// `14:00`, or `18/10`.
    pub label: String,
    pub claude: u64,
    pub codex: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRow {
    pub model: String,
    pub agent: Agent,
    /// Input, written to a cache or not.
    pub input: u64,
    pub cache_read: u64,
    pub output: u64,
    pub total: u64,
    /// `None`: the price table does not have the model.
    pub usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRow {
    /// The folder's own name; empty when the logs do not say.
    pub name: String,
    /// The whole path, `~` for the home.
    pub path: String,
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    /// claude-swap's slot; `None` when the history cannot tell.
    pub slot: Option<u32>,
    pub tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub period: Period,
    pub points: Vec<Point>,
    pub claude: u64,
    pub codex: u64,
    pub total: u64,
    /// At the API prices, for the models the table has.
    pub usd: f64,
    /// Models with tokens and no price: left out of `usd`.
    pub unpriced: Vec<String>,
    pub models: Vec<ModelRow>,
    pub projects: Vec<ProjectRow>,
    pub accounts: Vec<AccountRow>,
    pub prices_as_of: &'static str,
    pub prices_source: &'static str,
    pub skipped: Vec<Skipped>,
    /// When the logs were last read (Unix seconds).
    pub read_at: Option<i64>,
}

/// Which Claude account was in use when, from the switch history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Timeline {
    /// `(Unix seconds, slot)`, oldest first.
    pub switches: Vec<(i64, u32)>,
    /// The account before the first switch, when the history says.
    pub before: Option<u32>,
    /// The account in use now, for a history with no switch.
    pub now: Option<u32>,
}

impl Timeline {
    /// From the History tab's lines: the owner's switches here, the
    /// switcher's, and the changes the readings showed.
    pub fn from_history(entries: &[Entry], now: Option<u32>) -> Self {
        let mut switches: Vec<(i64, Option<u32>, u32)> = entries
            .iter()
            .filter(|e| e.app == App::Claude)
            .filter(|e| match &e.cause {
                Cause::Manual { result, .. } => result == "switched" || result == "unconfirmed",
                Cause::Switcher { .. } | Cause::Observed => true,
                _ => false,
            })
            .filter_map(|e| Some((super::unix(&e.at)?, e.from, e.to?)))
            .collect();
        switches.sort_by_key(|s| s.0);
        Timeline {
            before: switches.first().and_then(|s| s.1),
            switches: switches.into_iter().map(|(at, _, to)| (at, to)).collect(),
            now,
        }
    }

    pub fn slot_at(&self, at: i64) -> Option<u32> {
        match self.switches.iter().rev().find(|s| s.0 <= at) {
            Some(s) => Some(s.1),
            None if self.switches.is_empty() => self.now,
            None => self.before,
        }
    }
}

/// Midnight of `date` on this Mac's clock.
fn midnight(date: Date, offset: &dyn Fn(OffsetDateTime) -> UtcOffset) -> OffsetDateTime {
    let naive = date.with_time(Time::MIDNIGHT);
    let first = offset(naive.assume_utc());
    naive.assume_offset(offset(naive.assume_offset(first)))
}

/// `/Users/x/Projects/web` → `~/Projects/web` for a home of `/Users/x`.
fn show_path(path: &str, home: &Path) -> String {
    match Path::new(path).strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.to_string(),
    }
}

/// The report for `period` ending `now`; `offset` is this Mac's offset at
/// an instant ([`crate::local_time::offset_at`]).
pub fn report(
    index: &Index,
    period: Period,
    now: OffsetDateTime,
    offset: &dyn Fn(OffsetDateTime) -> UtcOffset,
    accounts: &Timeline,
    home: &Path,
) -> Report {
    let today = now.to_offset(offset(now)).date();
    let first_day = today - Duration::days(period.days() - 1);
    let from = midnight(first_day, offset).unix_timestamp();
    let to = midnight(today + Duration::days(1), offset).unix_timestamp();

    let mut points: Vec<Point> = match period {
        Period::Today => {
            (0..24).map(|h| Point { label: format!("{h:02}:00"), claude: 0, codex: 0 }).collect()
        }
        _ => (0..period.days())
            .map(|i| {
                let d = first_day + Duration::days(i);
                Point {
                    label: format!("{:02}/{:02}", d.day(), u8::from(d.month())),
                    claude: 0,
                    codex: 0,
                }
            })
            .collect(),
    };
    let mut models: BTreeMap<(Agent, String), Tokens> = BTreeMap::new();
    let mut projects: BTreeMap<String, u64> = BTreeMap::new();
    let mut slots: BTreeMap<Option<u32>, u64> = BTreeMap::new();

    for b in index::between(index, from, to) {
        let Ok(start) = OffsetDateTime::from_unix_timestamp(b.at) else { continue };
        let local = start.to_offset(offset(start));
        let column = match period {
            Period::Today => local.hour() as usize,
            _ => (local.date() - first_day).whole_days() as usize,
        };
        let n = b.tokens.total();
        if let Some(p) = points.get_mut(column) {
            match b.agent {
                Agent::Codex => p.codex += n,
                _ => p.claude += n,
            }
        }
        models.entry((b.agent, b.model.clone())).or_default().add(&b.tokens);
        *projects.entry(b.project.clone()).or_default() += n;
        if b.agent == Agent::Claude {
            *slots.entry(accounts.slot_at(b.at + BUCKET / 2)).or_default() += n;
        }
    }

    let claude = points.iter().map(|p| p.claude).sum();
    let codex = points.iter().map(|p| p.codex).sum();
    let mut usd = 0.0;
    let mut unpriced = Vec::new();
    let mut model_rows: Vec<ModelRow> = models
        .into_iter()
        .map(|((agent, model), t)| {
            let cost = prices::price(&model).map(|p| prices::cost(&p, &t));
            match cost {
                Some(c) => usd += c,
                None if !t.is_empty() => unpriced.push(model.clone()),
                None => {}
            }
            ModelRow {
                model,
                agent,
                input: t.input + t.cache_write,
                cache_read: t.cache_read,
                output: t.output,
                total: t.total(),
                usd: cost,
            }
        })
        .collect();
    model_rows.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.model.cmp(&b.model)));
    let mut project_rows: Vec<ProjectRow> = projects
        .into_iter()
        .map(|(path, tokens)| ProjectRow {
            name: Path::new(&path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: if path.is_empty() { String::new() } else { show_path(&path, home) },
            tokens,
        })
        .collect();
    project_rows.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.path.cmp(&b.path)));
    let mut account_rows: Vec<AccountRow> =
        slots.into_iter().map(|(slot, tokens)| AccountRow { slot, tokens }).collect();
    account_rows.sort_by(|a, b| b.tokens.cmp(&a.tokens).then_with(|| a.slot.cmp(&b.slot)));

    Report {
        period,
        points,
        claude,
        codex,
        total: claude + codex,
        usd,
        unpriced,
        models: model_rows,
        projects: project_rows,
        accounts: account_rows,
        prices_as_of: prices::AS_OF,
        prices_source: prices::SOURCE,
        skipped: index.skipped(),
        read_at: index.read_at,
    }
}
