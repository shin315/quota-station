//! The snapshot the tray and the popover draw: every Claude and Antigravity
//! account on this Mac, the whole-rotation numbers, and how fresh they are.
//!
//! Built only from quota-axi's report. Numbers are never guessed: a window
//! past its reset stays empty (`pending`) until the owner measures again.

use serde::Serialize;
use time::OffsetDateTime;

use crate::axi::{self, Provider, Response, Scope, SwapSlot, Window};
use crate::time_util::{is_after, latest, parse_ts};

/// A reading older than this is stale, as quota-axi, claude-swap and the
/// `cswap-prefer-team` switcher all count it.
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
    /// `percent` is a lower bound: another window limits first, and quota-axi
    /// reports only the tighter number for this account.
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
    /// quota-axi's own live reading of the signed-in account.
    Live,
    ClaudeSwap,
    Cockpit,
}

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
    /// claude-swap slot or Cockpit account number; 0 for the one account a
    /// provider row reads when no sidecar lists the others.
    pub number: u32,
    pub role: Role,
    pub active: bool,
    pub disabled: bool,
    pub reading: Reading,
    pub fetched_at: Option<String>,
    pub source: Source,
    pub membership: Membership,
    /// A rotation end still ahead.
    pub leaves_at: Option<String>,
    /// quota-axi's numbers: the tighter window only, when it reports one.
    pub lines: Vec<Line>,
    /// Every window on its own, from claude-swap's list (Claude) or the
    /// app's own reading (Antigravity); empty when neither has the account.
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
    /// quota-axi counted the whole rotation.
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
    pub room: Option<f64>,
    /// 0-100 for the bar: room per counted account, or the active percent.
    pub percent: Option<f64>,
    /// Active: the line the percent comes from.
    pub line: Option<LineKind>,
    /// Rotation: the scope counted (`all_models`, `gemini`).
    pub scope: Option<String>,
    pub members: u32,
    /// Rotation: each counted account's room before its switch threshold.
    pub rooms: Vec<Room>,
    pub next_refill_at: Option<String>,
    /// Accounts whose room comes back at `next_refill_at`.
    pub refills: Vec<u32>,
    pub exhausted: bool,
    /// Why quota-axi kept the active account's numbers, when it said.
    pub reason: Option<String>,
}

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
            room: None,
            percent: None,
            line: None,
            scope: None,
            members: 0,
            rooms: Vec::new(),
            next_refill_at: None,
            refills: Vec::new(),
            exhausted: false,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionStatus {
    /// The claude-swap or Cockpit account list was read.
    Ok,
    /// The provider row has no account list (no Cockpit on this Mac, or a
    /// quota-axi without the `+cswap` fork): only the signed-in account.
    NoSidecar,
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
    pub reason: Option<String>,
    pub accounts: Vec<Account>,
    pub pool: Pool,
    /// When quota-axi wrote the report behind this section.
    pub generated_at: Option<String>,
    /// The newest reading behind the accounts.
    pub newest_reading_at: Option<String>,
    /// Every reading is older than [`STALE_AFTER_SECONDS`].
    pub stale: bool,
    /// The rotation's personal hours, `8-23`.
    pub personal_hours: Option<String>,
    /// Who read the Antigravity accounts quota-axi lists: `quota-station`
    /// (this app) or Cockpit Tools; none without that list.
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
            personal_hours: None,
            source: None,
        }
    }

    pub fn active(&self) -> Option<&Account> {
        self.accounts.iter().find(|a| a.active)
    }
}

/// Whether quota-axi may read Claude Code's Keychain item.
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
    pub keychain: Keychain,
}

impl Snapshot {
    /// Builds the snapshot from the last `--provider claude` and
    /// `--provider agy` reports; either may be missing.
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
            keychain,
        }
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
    if state.reason.as_deref() == Some("keychain_access_required")
        || state.error.as_deref() == Some("keychain_prompt_required")
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
    let pool = row.claude_pool.as_ref();
    let rotation = pool.and_then(|p| p.rotation.as_ref());

    let mut section = match &row.claude_swap {
        Some(swap) if swap.status == "ok" => {
            let mut s = Section::empty(SectionStatus::Ok);
            s.accounts = swap
                .slots
                .iter()
                .map(|slot| swap_account(slot, rotation, live.as_ref(), &live_at, now))
                .collect();
            s.accounts.sort_by_key(|a| a.number);
            s
        }
        Some(swap) => {
            let mut s = Section::empty(SectionStatus::Unavailable);
            s.reason = swap.reason.clone();
            s.accounts.extend(live_only_account(live.as_ref(), &live_at));
            s
        }
        None if row.not_set_up => Section::empty(SectionStatus::NotSetUp),
        None => {
            let mut s = Section::empty(SectionStatus::NoSidecar);
            s.accounts.extend(live_only_account(live.as_ref(), &live_at));
            s
        }
    };
    // The account quota-axi reads live has its windows from that reading.
    for account in section.accounts.iter_mut().filter(|a| a.source == Source::Live) {
        account.windows = live_claude_windows(row);
    }
    section.personal_hours = rotation.and_then(|r| r.personal_hours.clone());
    section.pool = match pool {
        Some(p) if p.status == "pool" => rotation_pool(
            p.scopes.iter().find(|s| s.scope == "all_models").or(p.scopes.first()),
            p.members.len(),
        ),
        _ => active_pool(&section, pool.and_then(|p| p.reason.clone())),
    };
    finish(&mut section, now);
    (section, keychain)
}

/// The active slot's lines from quota-axi's own live reading, which is
/// fresher than claude-swap's, as `quota-axi --tui` draws it.
fn live_claude_lines(row: &Provider) -> Option<Vec<Line>> {
    if row.state.status != "fresh" {
        return None;
    }
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
    // The weekly Fable window itself, as the `--tui` fable column shows it.
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

fn swap_account(
    slot: &SwapSlot,
    rotation: Option<&axi::Rotation>,
    live: Option<&Vec<Line>>,
    live_at: &str,
    now: Option<OffsetDateTime>,
) -> Account {
    let (role, mut membership) = match rotation {
        None => (Role::Unknown, Membership::Unknown),
        Some(r) if r.personal == Some(slot.slot) => (Role::Personal, Membership::In),
        Some(r) if r.team.contains(&slot.slot) => (Role::Team, Membership::In),
        Some(_) => (Role::Unknown, Membership::Out),
    };
    let mut leaves_at = None;
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
        None => swap_lines(slot),
    };
    Account {
        number: slot.slot,
        role,
        active: slot.active,
        disabled: slot.disabled,
        reading: if live.is_some() { Reading::Fresh } else { reading(&slot.status) },
        fetched_at: if live.is_some() {
            Some(live_at.to_string())
        } else {
            slot.fetched_at.clone()
        },
        source: if live.is_some() { Source::Live } else { Source::ClaudeSwap },
        membership,
        leaves_at,
        windows: Vec::new(),
        email: None,
        org: None,
        lines,
    }
}

fn swap_lines(slot: &SwapSlot) -> Vec<Line> {
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
        // claude-swap's scope is the effective number: bounded by the account
        // windows too. When the Fable window is not among the limits, its own
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

/// The one account a provider row measures, when no sidecar lists accounts.
fn live_only_account(lines: Option<&Vec<Line>>, live_at: &str) -> Option<Account> {
    let lines = lines?.clone();
    Some(Account {
        number: 0,
        role: Role::Unknown,
        active: true,
        disabled: false,
        reading: Reading::Fresh,
        fetched_at: Some(live_at.to_string()),
        source: Source::Live,
        membership: Membership::Unknown,
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
    let mut section = match &row.cockpit {
        Some(cockpit) if cockpit.status == "ok" => {
            let mut s = Section::empty(SectionStatus::Ok);
            s.accounts = cockpit
                .accounts
                .iter()
                .map(|a| Account {
                    number: a.account,
                    role: Role::Unknown,
                    active: a.active,
                    disabled: false,
                    reading: reading(&a.status),
                    fetched_at: a.fetched_at.clone(),
                    source: Source::Cockpit,
                    membership: Membership::Unknown,
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
            s.source = cockpit.source.clone();
            s
        }
        Some(cockpit) => {
            let mut s = Section::empty(SectionStatus::Unavailable);
            s.reason = cockpit.reason.clone();
            s.source = cockpit.source.clone();
            s.accounts.extend(live_agy_account(row, &report.generated_at));
            s
        }
        None if row.not_set_up => Section::empty(SectionStatus::NotSetUp),
        None => {
            let mut s = Section::empty(SectionStatus::NoSidecar);
            s.accounts.extend(live_agy_account(row, &report.generated_at));
            s
        }
    };
    section.pool = match &row.agy_pool {
        Some(p) if p.status == "pool" => {
            let scope = p.rotation.as_ref().map(|r| r.scope.as_str());
            let found =
                p.scopes.iter().find(|s| Some(s.scope.as_str()) == scope).or(p.scopes.first());
            let mut pool = Pool::none();
            if let Some(s) = found {
                pool = room_pool(
                    &s.scope,
                    s.accounts_of_room,
                    s.effective_percent_remaining,
                    p.members.len(),
                    s.next_refill_at.clone(),
                    &s.accounts,
                );
            }
            pool
        }
        other => agy_active_pool(row, &section, other.as_ref().and_then(|p| p.reason.clone())),
    };
    finish(&mut section, now);
    section
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
        role: Role::Unknown,
        active: true,
        disabled: false,
        reading: reading(&row.state.status),
        fetched_at: row.state.refreshed_at.clone().or_else(|| Some(generated_at.to_string())),
        source: Source::Live,
        membership: Membership::Unknown,
        leaves_at: None,
        windows: agy_live_windows(row),
        email: None,
        org: None,
        lines,
    })
}

fn rotation_pool(scope: Option<&axi::ClaudePoolScope>, members: usize) -> Pool {
    let Some(s) = scope else { return Pool::none() };
    room_pool(
        &s.scope,
        s.slots_of_room,
        s.effective_percent_remaining,
        members,
        s.next_refill_at.clone(),
        &s.slots,
    )
}

fn room_pool(
    scope: &str,
    room: f64,
    effective: f64,
    members: usize,
    next_refill_at: Option<String>,
    counted: &[axi::RoomSlot],
) -> Pool {
    let members = u32::try_from(members).unwrap_or(u32::MAX);
    let percent =
        if members == 0 { 0.0 } else { (room / f64::from(members) * 100.0).clamp(0.0, 100.0) };
    let mut refills: Vec<u32> = counted
        .iter()
        .filter(|r| next_refill_at.is_some() && r.refill_at == next_refill_at)
        .map(|r| r.slot)
        .collect();
    refills.sort_unstable();
    let rooms = counted.iter().map(|r| Room { number: r.slot, percent: r.room_percent }).collect();
    Pool {
        kind: PoolKind::Rotation,
        room: Some(room),
        percent: Some(percent),
        line: None,
        scope: Some(scope.to_string()),
        members,
        rooms,
        next_refill_at,
        refills,
        exhausted: effective <= 0.0,
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

/// Antigravity's signed-in account: quota-axi's effective Gemini number, the
/// scope the agy switcher keeps room in by default.
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
