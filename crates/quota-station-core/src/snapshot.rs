// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The snapshot the tray and the popover draw: every Claude and Antigravity
//! account on this Mac, Codex's ChatGPT account, how much room is left, and
//! how fresh the numbers are.
//!
//! Built only from the app's own readings ([`crate::quota`]). Numbers are
//! never guessed: a window past its reset stays empty (`pending`) until the
//! owner measures again.

use serde::Serialize;
use time::OffsetDateTime;

#[cfg(feature = "author-tools")]
use crate::quota::RoomPool;
use crate::quota::{AgyAccounts, Provider, Response, Scope, Slot, Window};
#[cfg(feature = "author-tools")]
use crate::time_util::is_after;
use crate::time_util::{latest, parse_ts};

/// A reading older than this is stale.
pub const STALE_AFTER_SECONDS: i64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Session,
    Week,
    FableWeek,
    Gemini,
    ClaudeGpt,
    /// Antigravity's four windows, each on its own (`Account::windows`).
    GeminiSession,
    GeminiWeek,
    ClaudeGptSession,
    ClaudeGptWeek,
}

/// One allowance line of an account: percent left and when it resets.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    pub percent: Option<f64>,
    pub resets_at: Option<String>,
    /// The window passed its reset after the reading: no number until the
    /// next reading, never an assumed 100 %.
    pub pending: bool,
    /// `percent` is a lower bound: another window limits first, and only the
    /// tighter number is known for this account.
    pub at_least: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Reading {
    Fresh,
    Stale,
    ResetPending,
    NoData,
}

#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Team,
    Personal,
    /// No rotation file says what the account is.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// The live reading of the signed-in account.
    Live,
    /// The app's own list of the accounts it keeps.
    List,
}

#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Membership {
    In,
    Out,
    Ended,
    /// No rotation file.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// The account's number in the app's list; 0 for the one account a
    /// provider row reads when the app lists no others.
    pub number: u32,
    #[cfg(feature = "author-tools")]
    pub role: Role,
    pub active: bool,
    #[cfg(feature = "author-tools")]
    pub disabled: bool,
    pub reading: Reading,
    pub fetched_at: Option<String>,
    pub source: Source,
    #[cfg(feature = "author-tools")]
    pub membership: Membership,
    /// A rotation end still ahead.
    #[cfg(feature = "author-tools")]
    pub leaves_at: Option<String>,
    /// The effective numbers: the tighter window only, when that is all
    /// the list knows.
    pub lines: Vec<Line>,
    /// Every window on its own, from the app's own readings; empty when it
    /// has none of the account.
    /// Filled after the build (`detail`).
    pub windows: Vec<Line>,
    /// The account's email; filled only while the owner shows names, and
    /// left out of the JSON otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// The Claude account's organization, under the same rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PoolKind {
    /// The whole rotation was counted.
    #[cfg(feature = "author-tools")]
    Rotation,
    /// Only the signed-in account's own number.
    Active,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pool {
    pub kind: PoolKind,
    /// Rotation: room left, in fresh accounts (2.3 is two and a third).
    #[cfg(feature = "author-tools")]
    pub room: Option<f64>,
    /// 0-100 for the bar: the active account's percent left.
    pub percent: Option<f64>,
    /// Active: the line the percent comes from.
    pub line: Option<LineKind>,
    /// The scope counted (`all_models`, `gemini`).
    pub scope: Option<String>,
    /// Rotation: the accounts counted.
    #[cfg(feature = "author-tools")]
    pub members: u32,
    /// Rotation: each counted account's room before its switch threshold.
    #[cfg(feature = "author-tools")]
    pub rooms: Vec<Room>,
    pub next_refill_at: Option<String>,
    /// Accounts whose room comes back at `next_refill_at`.
    #[cfg(feature = "author-tools")]
    pub refills: Vec<u32>,
    pub exhausted: bool,
    /// Why there is no wider count than the active account's own numbers,
    /// when known.
    pub reason: Option<String>,
}

#[cfg(feature = "author-tools")]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub number: u32,
    /// 100 is a whole account's worth of room, 0 an account at its threshold.
    pub percent: f64,
}

impl Pool {
    fn none() -> Self {
        Pool {
            kind: PoolKind::None,
            #[cfg(feature = "author-tools")]
            room: None,
            percent: None,
            line: None,
            scope: None,
            #[cfg(feature = "author-tools")]
            members: 0,
            #[cfg(feature = "author-tools")]
            rooms: Vec::new(),
            next_refill_at: None,
            #[cfg(feature = "author-tools")]
            refills: Vec::new(),
            exhausted: false,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionStatus {
    /// The app's account list was read.
    Ok,
    /// The app lists no accounts of the provider here: only the signed-in
    /// account.
    Single,
    /// The account list could not be read; `reason` says why.
    Unavailable,
    /// The provider is not set up on this Mac.
    NotSetUp,
    /// No report has been read yet.
    Unread,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub status: SectionStatus,
    /// Why the list could not be read (`unavailable`), or why the one login
    /// read gave no number (`single`).
    pub reason: Option<String>,
    pub accounts: Vec<Account>,
    pub pool: Pool,
    /// When the report behind this section was built.
    pub generated_at: Option<String>,
    /// The newest reading behind the accounts.
    pub newest_reading_at: Option<String>,
    /// Every reading is older than [`STALE_AFTER_SECONDS`].
    pub stale: bool,
    /// The rotation's personal hours, `8-23`.
    #[cfg(feature = "author-tools")]
    pub personal_hours: Option<String>,
    /// `quota-station` when the Antigravity accounts are the app's own list;
    /// none without that list.
    pub source: Option<String>,
}

impl Section {
    fn empty(status: SectionStatus) -> Self {
        Section {
            status,
            reason: None,
            accounts: Vec::new(),
            pool: Pool::none(),
            generated_at: None,
            newest_reading_at: None,
            stale: false,
            #[cfg(feature = "author-tools")]
            personal_hours: None,
            source: None,
        }
    }

    pub fn active(&self) -> Option<&Account> {
        self.accounts.iter().find(|a| a.active)
    }
}

/// Whether the app may read Claude Code's Keychain item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Keychain {
    Granted,
    Required,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub claude: Section,
    pub agy: Section,
    /// Codex's own ChatGPT login: one account, its five-hour and weekly
    /// windows. Unread until [`Snapshot::with_codex`] adds a report.
    pub codex: Section,
    pub keychain: Keychain,
}

impl Snapshot {
    /// Builds the snapshot from the last Claude and Antigravity reports;
    /// either may be missing.
    pub fn build(claude: Option<&Response>, agy: Option<&Response>) -> Self {
        let stamp = |mut section: Section, report: &Response| {
            section.generated_at = Some(report.generated_at.clone());
            section
        };
        let (claude_section, keychain) = match claude {
            Some(report) => {
                let (section, keychain) = claude_section(report);
                (stamp(section, report), keychain)
            }
            None => (Section::empty(SectionStatus::Unread), Keychain::Unknown),
        };
        Snapshot {
            claude: claude_section,
            agy: agy.map_or_else(
                || Section::empty(SectionStatus::Unread),
                |report| stamp(agy_section(report), report),
            ),
            codex: Section::empty(SectionStatus::Unread),
            keychain,
        }
    }

    /// Adds the last Codex report, when there is one.
    pub fn with_codex(mut self, report: Option<&Response>) -> Self {
        if let Some(report) = report {
            self.codex = codex_section(report);
            self.codex.generated_at = Some(report.generated_at.clone());
        }
        self
    }
}

fn reading(status: &str) -> Reading {
    match status {
        "fresh" => Reading::Fresh,
        "stale" => Reading::Stale,
        "reset_pending" => Reading::ResetPending,
        _ => Reading::NoData,
    }
}

fn keychain(row: &Provider) -> Keychain {
    let state = &row.state;
    if state.reason.as_deref() == Some(crate::quota::claude::ACCESS_REQUIRED)
        || state.error.as_deref() == Some(crate::quota::claude::PROMPT_REQUIRED)
    {
        Keychain::Required
    } else if state.status == "fresh" {
        Keychain::Granted
    } else {
        Keychain::Unknown
    }
}

fn claude_section(report: &Response) -> (Section, Keychain) {
    let Some(row) = report.primary("claude") else {
        return (Section::empty(SectionStatus::NotSetUp), Keychain::Unknown);
    };
    let keychain = keychain(row);
    let now = parse_ts(&report.generated_at);
    let live = live_claude_lines(row);
    let live_at = row.state.refreshed_at.clone().unwrap_or_else(|| report.generated_at.clone());
    let accounts = row.claude_accounts.as_ref();
    #[cfg(feature = "author-tools")]
    let rotation = accounts.and_then(|a| a.rotation.as_ref());

    let mut section = match accounts {
        Some(list) => {
            let mut s = Section::empty(SectionStatus::Ok);
            s.accounts = list
                .slots
                .iter()
                .map(|slot| {
                    #[cfg(feature = "author-tools")]
                    return list_account(slot, rotation, live.as_ref(), &live_at, now);
                    #[cfg(not(feature = "author-tools"))]
                    list_account(slot, live.as_ref(), &live_at)
                })
                .collect();
            s.accounts.sort_by_key(|a| a.number);
            s
        }
        None if row.not_set_up => Section::empty(SectionStatus::NotSetUp),
        None => {
            // The one login read: its last numbers, stale ones marked so.
            let mut s = Section::empty(SectionStatus::Single);
            s.accounts.extend(live_only_account(row, &live_at));
            if s.accounts.is_empty() {
                s.reason = row.state.error.clone();
            }
            s
        }
    };
    // The account read live has its windows from that reading.
    for account in section.accounts.iter_mut().filter(|a| a.source == Source::Live) {
        account.windows = live_claude_windows(row);
    }
    #[cfg(feature = "author-tools")]
    {
        section.personal_hours = rotation.and_then(|r| r.personal_hours.clone());
    }
    #[cfg(feature = "author-tools")]
    let pool = match accounts.and_then(|a| a.pool.as_ref()) {
        Some(pool) => room_pool(pool),
        None => active_pool(&section, accounts.and_then(|a| a.pool_reason.clone())),
    };
    #[cfg(not(feature = "author-tools"))]
    let pool = active_pool(&section, None);
    section.pool = pool;
    finish(&mut section, now);
    (section, keychain)
}

/// The active account's lines from the row's own reading: the tighter of
/// the five-hour and weekly windows, and the Fable week on its own.
fn live_claude_lines(row: &Provider) -> Option<Vec<Line>> {
    if row.state.status != "fresh" {
        return None;
    }
    claude_lines(row)
}

/// The row's lines, however old its reading.
fn claude_lines(row: &Provider) -> Option<Vec<Line>> {
    let find = |id: &str| row.windows.iter().find(|w| w.id == id && w.percent_remaining.is_some());
    let five = find("five_hour")?;
    let account: Vec<&Window> = [Some(five), find("seven_day")].into_iter().flatten().collect();
    let mut lines = vec![tightest(&account, |w| {
        if w.id == "seven_day" {
            LineKind::Week
        } else {
            LineKind::Session
        }
    })];
    // The weekly Fable window itself.
    if let Some(fable) = find("model:fable") {
        lines.push(Line {
            kind: LineKind::FableWeek,
            percent: fable.percent_remaining,
            resets_at: fable.resets_at.clone(),
            pending: false,
            at_least: false,
        });
    }
    Some(lines)
}

/// The live reading's five-hour, weekly and Fable windows, each on its own.
fn live_claude_windows(row: &Provider) -> Vec<Line> {
    [
        ("five_hour", LineKind::Session),
        ("seven_day", LineKind::Week),
        ("model:fable", LineKind::FableWeek),
    ]
    .into_iter()
    .filter_map(|(id, kind)| {
        let w = row.windows.iter().find(|w| w.id == id && w.percent_remaining.is_some())?;
        Some(Line {
            kind,
            percent: w.percent_remaining,
            resets_at: w.resets_at.clone(),
            pending: false,
            at_least: false,
        })
    })
    .collect()
}

/// The smallest percent left among `windows`, with the latest reset among
/// the windows at that percent: the effective number and when it recovers.
fn tightest(windows: &[&Window], kind_of: impl Fn(&Window) -> LineKind) -> Line {
    let min = windows.iter().filter_map(|w| w.percent_remaining).fold(f64::INFINITY, f64::min);
    let limiting: Vec<&&Window> =
        windows.iter().filter(|w| w.percent_remaining == Some(min)).collect();
    // A tie names the longer window: the number cannot recover before it resets.
    let kind = limiting
        .iter()
        .map(|w| kind_of(w))
        .find(|k| *k == LineKind::Week)
        .unwrap_or_else(|| kind_of(limiting[0]));
    Line {
        kind,
        percent: Some(min),
        resets_at: latest(limiting.iter().filter_map(|w| w.resets_at.as_deref())),
        pending: false,
        at_least: false,
    }
}

fn list_account(
    slot: &Slot,
    #[cfg(feature = "author-tools")] rotation: Option<&crate::quota::Rotation>,
    live: Option<&Vec<Line>>,
    live_at: &str,
    #[cfg(feature = "author-tools")] now: Option<OffsetDateTime>,
) -> Account {
    #[cfg(feature = "author-tools")]
    let (role, mut membership) = match rotation {
        None => (Role::Unknown, Membership::Unknown),
        Some(r) if r.personal == Some(slot.slot) => (Role::Personal, Membership::In),
        Some(r) if r.team.contains(&slot.slot) => (Role::Team, Membership::In),
        Some(_) => (Role::Unknown, Membership::Out),
    };
    #[cfg(feature = "author-tools")]
    let mut leaves_at = None;
    #[cfg(feature = "author-tools")]
    if let Some(end) = rotation.and_then(|r| r.expire.iter().find(|e| e.slot == slot.slot)) {
        if is_after(&end.at, now) {
            leaves_at = Some(end.at.clone());
        } else {
            membership = Membership::Ended;
        }
    }
    let live = live.filter(|_| slot.active);
    let lines = match live {
        Some(lines) => lines.clone(),
        None => slot_lines(slot),
    };
    Account {
        number: slot.slot,
        #[cfg(feature = "author-tools")]
        role,
        active: slot.active,
        #[cfg(feature = "author-tools")]
        disabled: slot.disabled,
        reading: if live.is_some() { Reading::Fresh } else { reading(&slot.status) },
        fetched_at: if live.is_some() {
            Some(live_at.to_string())
        } else {
            slot.fetched_at.clone()
        },
        source: if live.is_some() { Source::Live } else { Source::List },
        #[cfg(feature = "author-tools")]
        membership,
        #[cfg(feature = "author-tools")]
        leaves_at,
        windows: Vec::new(),
        email: None,
        org: None,
        lines,
    }
}

fn slot_lines(slot: &Slot) -> Vec<Line> {
    let week = limited_by(slot.limited_by.as_deref(), "seven_day");
    let mut lines = vec![Line {
        kind: if week { LineKind::Week } else { LineKind::Session },
        percent: slot.effective_percent_remaining,
        resets_at: slot.resets_at.clone(),
        pending: slot.status == "reset_pending" && slot.effective_percent_remaining.is_none(),
        at_least: false,
    }];
    if let Some(fable) = slot.scopes.iter().find(|s| s.scope == "model:fable") {
        let mut line = scope_line(fable, LineKind::FableWeek);
        // The scope is the effective number: bounded by the account windows
        // too. When the Fable window is not among the limits, its own
        // percent is only known to be at least that, and its reset is unknown.
        let fable_limits = limited_by(fable.limited_by.as_deref(), "model:fable");
        if line.percent.is_some() && !line.pending && !fable_limits {
            line.at_least = true;
            line.resets_at = None;
        }
        lines.push(line);
    }
    lines
}

fn limited_by(list: Option<&str>, window: &str) -> bool {
    list.is_some_and(|by| by.split('+').any(|w| w.trim() == window))
}

fn scope_line(scope: &Scope, kind: LineKind) -> Line {
    Line {
        kind,
        percent: scope.effective_percent_remaining,
        resets_at: scope.resets_at.clone(),
        pending: scope.status == "reset_pending",
        at_least: false,
    }
}

/// The one account a provider row measures, when the app lists no others.
fn live_only_account(row: &Provider, live_at: &str) -> Option<Account> {
    let lines = claude_lines(row)?;
    Some(Account {
        number: 0,
        #[cfg(feature = "author-tools")]
        role: Role::Unknown,
        active: true,
        #[cfg(feature = "author-tools")]
        disabled: false,
        reading: reading(&row.state.status),
        fetched_at: Some(live_at.to_string()),
        source: Source::Live,
        #[cfg(feature = "author-tools")]
        membership: Membership::Unknown,
        #[cfg(feature = "author-tools")]
        leaves_at: None,
        windows: Vec::new(),
        email: None,
        org: None,
        lines,
    })
}

fn agy_section(report: &Response) -> Section {
    let Some(row) = report.primary("agy") else {
        return Section::empty(SectionStatus::NotSetUp);
    };
    let now = parse_ts(&report.generated_at);
    let mut section = match &row.agy_accounts {
        Some(list) if list.status == "ok" => listed_agy(row, list),
        Some(list) => {
            let mut s = Section::empty(SectionStatus::Unavailable);
            s.reason = list.reason.clone();
            s.source = Some(LIST_SOURCE.into());
            s.accounts.extend(live_agy_account(row, &report.generated_at));
            s
        }
        None if row.not_set_up => Section::empty(SectionStatus::NotSetUp),
        None => {
            let mut s = Section::empty(SectionStatus::Single);
            s.accounts.extend(live_agy_account(row, &report.generated_at));
            if s.accounts.is_empty() {
                s.reason = row.state.error.clone();
            }
            s
        }
    };
    section.pool = agy_active_pool(row, &section, None);
    finish(&mut section, now);
    section
}

/// [`Section::source`] of the app's own Antigravity list.
pub const LIST_SOURCE: &str = "quota-station";

fn listed_agy(row: &Provider, list: &AgyAccounts) -> Section {
    let mut s = Section::empty(SectionStatus::Ok);
    s.accounts = list
        .accounts
        .iter()
        .map(|a| Account {
            number: a.account,
            #[cfg(feature = "author-tools")]
            role: Role::Unknown,
            active: a.active,
            #[cfg(feature = "author-tools")]
            disabled: false,
            reading: reading(&a.status),
            fetched_at: a.fetched_at.clone(),
            source: Source::List,
            #[cfg(feature = "author-tools")]
            membership: Membership::Unknown,
            #[cfg(feature = "author-tools")]
            leaves_at: None,
            windows: Vec::new(),
            email: None,
            org: None,
            lines: [("gemini", LineKind::Gemini), ("claude_gpt", LineKind::ClaudeGpt)]
                .into_iter()
                .filter_map(|(id, kind)| {
                    a.scopes.iter().find(|s| s.scope == id).map(|s| scope_line(s, kind))
                })
                .collect(),
        })
        .collect();
    s.accounts.sort_by_key(|a| a.number);
    // The account in use has its four windows from the live row.
    for account in s.accounts.iter_mut().filter(|a| a.active) {
        account.windows = agy_live_windows(row);
    }
    s.source = Some(LIST_SOURCE.into());
    s
}

/// The live `agy` row's four windows, each on its own.
fn agy_live_windows(row: &Provider) -> Vec<Line> {
    [
        ("gemini_5h", LineKind::GeminiSession),
        ("gemini_weekly", LineKind::GeminiWeek),
        ("claude_gpt_5h", LineKind::ClaudeGptSession),
        ("claude_gpt_weekly", LineKind::ClaudeGptWeek),
    ]
    .into_iter()
    .filter_map(|(id, kind)| {
        row.windows.iter().find(|w| w.id == id).map(|w| Line {
            kind,
            percent: w.percent_remaining,
            resets_at: w.resets_at.clone(),
            pending: false,
            at_least: false,
        })
    })
    .collect()
}

/// The signed-in Antigravity account from the live `agy` row: its Gemini
/// five-hour and weekly windows.
fn live_agy_account(row: &Provider, generated_at: &str) -> Option<Account> {
    let lines: Vec<Line> = [("gemini_5h", LineKind::Session), ("gemini_weekly", LineKind::Week)]
        .into_iter()
        .filter_map(|(id, kind)| {
            row.windows.iter().find(|w| w.id == id).map(|w| Line {
                kind,
                percent: w.percent_remaining,
                resets_at: w.resets_at.clone(),
                pending: false,
                at_least: false,
            })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some(Account {
        number: 0,
        #[cfg(feature = "author-tools")]
        role: Role::Unknown,
        active: true,
        #[cfg(feature = "author-tools")]
        disabled: false,
        reading: reading(&row.state.status),
        fetched_at: row.state.refreshed_at.clone().or_else(|| Some(generated_at.to_string())),
        source: Source::Live,
        #[cfg(feature = "author-tools")]
        membership: Membership::Unknown,
        #[cfg(feature = "author-tools")]
        leaves_at: None,
        windows: agy_live_windows(row),
        email: None,
        org: None,
        lines,
    })
}

/// Codex's own login (`codex-home`): the one Codex itself signs in with.
/// Its five-hour and weekly windows.
fn codex_section(report: &Response) -> Section {
    let rows: Vec<&Provider> = report.providers.iter().filter(|p| p.provider == "codex").collect();
    let is_home = |p: &Provider| p.account_keys.iter().any(|k| k == "codex-home");
    let Some(home) = rows.iter().copied().find(|p| is_home(p)).or(rows.first().copied()) else {
        return Section::empty(SectionStatus::NotSetUp);
    };
    if home.not_set_up && rows.len() == 1 {
        return Section::empty(SectionStatus::NotSetUp);
    }
    // One account per ChatGPT sign-in the app keeps (`codex:<n>`); Codex's
    // own login alone, before the app keeps any, is number 0. The one in
    // the Codex folder is the active one.
    // Another login's row (quota-axi reads Pi's too) is not Codex's.
    let mut accounts: Vec<Account> = rows
        .iter()
        .filter_map(|row| {
            let at_home = std::ptr::eq(*row, home);
            let kept = row.account_keys.iter().any(|k| k.starts_with("codex:"));
            (at_home || kept).then(|| codex_account(row, report, at_home))?
        })
        .collect();
    accounts.sort_by_key(|a| a.number);
    let mut section = if accounts.is_empty() {
        let mut s = Section::empty(SectionStatus::Unavailable);
        s.reason = home.state.error.clone().or_else(|| home.state.reason.clone());
        s
    } else {
        let mut s = Section::empty(SectionStatus::Ok);
        s.accounts = accounts;
        s
    };
    finish(&mut section, parse_ts(&report.generated_at));
    section
}

/// One Codex row's account, when it has the five hours or the week.
fn codex_account(row: &Provider, report: &Response, home: bool) -> Option<Account> {
    let lines: Vec<Line> = [("five_hour", LineKind::Session), ("weekly", LineKind::Week)]
        .into_iter()
        .filter_map(|(id, kind)| {
            let w = row.windows.iter().find(|w| w.id == id && w.percent_remaining.is_some())?;
            Some(Line {
                kind,
                percent: w.percent_remaining,
                resets_at: w.resets_at.clone(),
                pending: false,
                at_least: false,
            })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    let number = row
        .account_keys
        .iter()
        .find_map(|k| k.strip_prefix("codex:")?.parse::<u32>().ok())
        .unwrap_or(0);
    Some(Account {
        number,
        #[cfg(feature = "author-tools")]
        role: Role::Unknown,
        active: home,
        #[cfg(feature = "author-tools")]
        disabled: false,
        reading: reading(&row.state.status),
        fetched_at: row.state.refreshed_at.clone().or_else(|| Some(report.generated_at.clone())),
        source: Source::Live,
        #[cfg(feature = "author-tools")]
        membership: Membership::Unknown,
        #[cfg(feature = "author-tools")]
        leaves_at: None,
        windows: lines.clone(),
        email: None,
        org: None,
        lines,
    })
}

#[cfg(feature = "author-tools")]
fn room_pool(pool: &RoomPool) -> Pool {
    let members = pool.members;
    let room = pool.slots_of_room;
    let percent =
        if members == 0 { 0.0 } else { (room / f64::from(members) * 100.0).clamp(0.0, 100.0) };
    let next_refill_at = pool.next_refill_at.clone();
    let mut refills: Vec<u32> = pool
        .slots
        .iter()
        .filter(|r| next_refill_at.is_some() && r.refill_at == next_refill_at)
        .map(|r| r.slot)
        .collect();
    refills.sort_unstable();
    Pool {
        kind: PoolKind::Rotation,
        room: Some(room),
        percent: Some(percent),
        line: None,
        scope: Some(pool.scope.clone()),
        members,
        rooms: pool
            .slots
            .iter()
            .map(|r| Room { number: r.slot, percent: r.room_percent })
            .collect(),
        next_refill_at,
        refills,
        exhausted: pool.effective_percent_remaining <= 0.0,
        reason: None,
    }
}

fn active_pool(section: &Section, reason: Option<String>) -> Pool {
    let Some(line) = section.active().and_then(|a| a.lines.first()) else {
        return Pool { reason, ..Pool::none() };
    };
    let Some(percent) = line.percent else {
        return Pool { reason, ..Pool::none() };
    };
    Pool {
        kind: PoolKind::Active,
        percent: Some(percent),
        line: Some(line.kind),
        exhausted: percent <= 0.0,
        next_refill_at: line.resets_at.clone(),
        reason,
        ..Pool::none()
    }
}

/// Antigravity's signed-in account: its effective Gemini number.
fn agy_active_pool(row: &Provider, section: &Section, reason: Option<String>) -> Pool {
    let gemini = row
        .quota_semantics
        .as_ref()
        .and_then(|q| q.effective_availability.iter().find(|e| e.scope == "gemini"))
        .filter(|e| e.status == "known");
    match gemini.and_then(|e| e.effective_percent_remaining.map(|p| (e, p))) {
        Some((effective, percent)) => {
            let weekly = effective.limiting_window_ids.iter().any(|id| {
                row.windows.iter().any(|w| &w.id == id && w.kind.as_deref() == Some("weekly"))
            });
            Pool {
                kind: PoolKind::Active,
                percent: Some(percent),
                line: Some(if weekly { LineKind::Week } else { LineKind::Session }),
                scope: Some("gemini".to_string()),
                exhausted: percent <= 0.0,
                reason,
                ..Pool::none()
            }
        }
        None => active_pool(section, reason),
    }
}

fn finish(section: &mut Section, now: Option<OffsetDateTime>) {
    section.newest_reading_at =
        latest(section.accounts.iter().filter_map(|a| a.fetched_at.as_deref()));
    let measured = section.accounts.iter().filter(|a| a.reading != Reading::NoData);
    let any_fresh =
        measured.clone().any(|a| matches!(a.reading, Reading::Fresh | Reading::ResetPending));
    let newest_old = match (section.newest_reading_at.as_deref().and_then(parse_ts), now) {
        (Some(at), Some(now)) => (now - at).whole_seconds() > STALE_AFTER_SECONDS,
        _ => false,
    };
    section.stale =
        !any_fresh && (measured.clone().any(|a| a.reading == Reading::Stale) || newest_old);
}
