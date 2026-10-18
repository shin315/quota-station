//! The five alerts (F09), each sent once for its event, as a macOS card with
//! buttons and as the same card in the Cảnh báo tab:
//!
//! 1. The whole Claude pool about to run dry, and back: Firstmate's
//!    quota-pool rule on claude-swap's numbers. Dry: the account in use has
//!    10 % or less, and no other account the switcher may choose now has
//!    15 % or more. Back: no longer dry.
//! 2. An account about to leave the rotation: 30 minutes before its
//!    `expire` in the rotation file.
//! 3. A login to renew: a Claude account whose login ends within 3 days, or
//!    whose last reading failed with a login only a new one mends; an
//!    Antigravity account here that needs a new login or a verification.
//! 4. An Antigravity switch overwritten: the six-hour watch after a switch
//!    (M4c) saw the old account back in use.
//! 5. Firstmate holding new Claude work: `pool=dry` in its state file.
//!
//! An alert's key names its event (the account, the time it ends), and a
//! key is sent once; "Nhắc ngày mai" sends it again the next morning while
//! it still holds. The cards name accounts the way the windows do (Team A,
//! Cá nhân, Ultra 1), never by email. What each button does is
//! [`answer`]'s: some only note the answer, the others return an [`Effect`]
//! for the app to carry out.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset, Weekday};

use crate::antigravity::fs::AppDir;
use crate::antigravity::index::{self, Standing};
use crate::antigravity::switch::{How, Outcome as AgyOutcome};
use crate::antigravity::watch;
use crate::axi::{Response, SwapSlot};
use crate::firstmate;
use crate::local_time;
use crate::rotation::Rules;
use crate::snapshot::{Role, Snapshot};
use crate::time_util::parse_ts;

/// In the app's folder, `alerts/alerts.json`.
pub const DIR: &str = "alerts";
pub const FILE: &str = "alerts.json";
/// Alert 2 comes this long before an account leaves the rotation.
pub const LEAVING_BEFORE: i64 = 30 * 60;
/// Alert 3 comes when a Claude login ends within this long.
pub const LOGIN_WITHIN: i64 = 3 * 86400;
/// Firstmate's dry rule: the account in use at this percent or less…
pub const DRY_AT_MOST: f64 = 10.0;
/// …and no other usable account with this much.
pub const ROOM_AT_LEAST: f64 = 15.0;
/// Alert 4 offers to switch back for this many overwrites of one switch.
pub const OVERWRITE_OFFERS: u32 = 2;
/// "Nhắc ngày mai" reminds at this hour.
const REMIND_HOUR: u8 = 9;
const RECENT_KEEP: usize = 30;
/// A sent key is forgotten after this long.
const SENT_KEEP: i64 = 60 * 86400;
/// claude-swap's errors that only a new login mends: a refresh token
/// Anthropic refused, or a login without one.
const RELOGIN_ERRORS: [&str; 4] =
    ["invalid_grant", "no_refresh_token", "no-access-token", "relogin_required"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Pool,
    Leaving,
    Login,
    Overwrite,
    Firstmate,
}

impl Kind {
    pub const ALL: [Kind; 5] =
        [Kind::Pool, Kind::Leaving, Kind::Login, Kind::Overwrite, Kind::Firstmate];

    /// Its number in the plan and the Cảnh báo tab, 1 to 5.
    pub fn number(self) -> u8 {
        match self {
            Kind::Pool => 1,
            Kind::Leaving => 2,
            Kind::Login => 3,
            Kind::Overwrite => 4,
            Kind::Firstmate => 5,
        }
    }
}

/// Which alerts are on: the first four by default; Firstmate's only where
/// Firstmate runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Switches {
    pub pool: bool,
    pub leaving: bool,
    pub login: bool,
    pub overwrite: bool,
    pub firstmate: bool,
}

impl Default for Switches {
    fn default() -> Self {
        Switches { pool: true, leaving: true, login: true, overwrite: true, firstmate: false }
    }
}

impl Switches {
    pub fn on(&self, kind: Kind) -> bool {
        match kind {
            Kind::Pool => self.pool,
            Kind::Leaving => self.leaving,
            Kind::Login => self.login,
            Kind::Overwrite => self.overwrite,
            Kind::Firstmate => self.firstmate,
        }
    }

    pub fn set(&mut self, kind: Kind, on: bool) {
        match kind {
            Kind::Pool => self.pool = on,
            Kind::Leaving => self.leaving = on,
            Kind::Login => self.login = on,
            Kind::Overwrite => self.overwrite = on,
            Kind::Firstmate => self.firstmate = on,
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Switches::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Ok,
    Warn,
    Bad,
    Info,
}

/// One button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    /// `dismiss`, `snooze`, `switch:N`, `agy:keep`, `agy:<how>:N`,
    /// `agy:login` or `agy:verify:N`.
    pub id: String,
    pub label: String,
}

fn act(id: impl Into<String>, label: impl Into<String>) -> Act {
    Act { id: id.into(), label: label.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    /// Names the event; one key is sent once.
    pub key: String,
    pub kind: Kind,
    pub tone: Tone,
    pub title: String,
    pub body: String,
    pub actions: Vec<Act>,
    /// Unix seconds.
    pub at: i64,
    /// Sent with "Gửi thử": its buttons change nothing.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub test: bool,
}

/// What a button led to, shown under the card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Said {
    pub tone: Tone,
    pub title: String,
    pub body: String,
}

fn said(tone: Tone, title: impl Into<String>, body: impl Into<String>) -> Said {
    Said { tone, title: title.into(), body: body.into() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub action: String,
    pub label: String,
    pub at: i64,
    /// `None` while the action runs.
    pub result: Option<Said>,
}

/// One alert as sent: a macOS card and a card in the Cảnh báo tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// The card's id, unique to this sending (a reminder is a new one).
    pub id: String,
    pub alert: Alert,
    /// macOS took the card; false when notifications are off or cannot
    /// be shown.
    pub notified: bool,
    pub answer: Option<Answer>,
}

/// What the app keeps between reads and restarts, in `alerts/alerts.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Memory {
    /// The pool rule's last answer, for alert 1's two edges.
    pub pool_dry: Option<bool>,
    /// Firstmate's last `pool=`, for alert 5's edge.
    pub firstmate_dry: Option<bool>,
    /// Keys sent, and when first.
    pub sent: BTreeMap<String, i64>,
    /// Keys to send again ("Nhắc ngày mai"), and when.
    pub snoozed: BTreeMap<String, i64>,
    /// Newest first.
    pub recent: Vec<Record>,
}

impl Memory {
    pub fn record(&self, id: &str) -> Option<&Record> {
        self.recent.iter().find(|r| r.id == id)
    }

    /// Cards with buttons nobody has pressed yet.
    pub fn unanswered(&self) -> usize {
        self.recent
            .iter()
            .filter(|r| !r.alert.test && r.answer.is_none() && !r.alert.actions.is_empty())
            .count()
    }
}

pub fn path(app: &AppDir) -> PathBuf {
    app.path().join(DIR).join(FILE)
}

/// What was kept; none, or an unreadable file, starts afresh.
pub fn load(app: &AppDir) -> Memory {
    app.read_to_string(&path(app))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppDir, memory: &Memory) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(memory).map_err(std::io::Error::other)?;
    app.write_private(&path(app), &text)
}

// ===== What the rules look at =====

/// An Antigravity account logged in here that needs the owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgyLogin {
    /// Its place in the app's list, from 1.
    pub number: u32,
    pub id: String,
    pub standing: Standing,
}

/// A switch a running Antigravity.app undid, not answered yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overwrite {
    /// The account switched to, by id and number.
    pub to_id: String,
    pub to: u32,
    /// The account back in use.
    pub from: u32,
    /// When the switch was made, Unix seconds.
    pub switched_at: i64,
    /// When a reading showed the old account back.
    pub at: i64,
    pub overwrites: u32,
    /// Antigravity.app runs; `None` when that could not be told.
    pub running: Option<bool>,
}

pub struct Facts<'a> {
    pub now: OffsetDateTime,
    /// This Mac's offset, for the clock times on the cards and the personal
    /// hours.
    pub offset: UtcOffset,
    pub claude: Option<&'a Response>,
    /// The rotation file's rules, for a report without quota-axi's rotation
    /// block.
    pub rules: Option<&'a Rules>,
    pub agy: Vec<AgyLogin>,
    pub overwrite: Option<Overwrite>,
    pub firstmate: Option<&'a firstmate::Reading>,
}

/// The Antigravity accounts here that need a new login or a verification.
pub fn agy_logins(app: &AppDir) -> Vec<AgyLogin> {
    let Ok(index) = index::load(app) else { return Vec::new() };
    let statuses = index::load_statuses(app);
    index
        .accounts
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            statuses.accounts.get(&a.id).map(|s| AgyLogin {
                number: i as u32 + 1,
                id: a.id.clone(),
                standing: s.clone(),
            })
        })
        .collect()
}

/// The overwrite the six-hour watch holds, with the accounts by number.
pub fn overwrite(app: &AppDir, running: Option<bool>) -> Option<Overwrite> {
    let w = watch::load(app)?;
    let at = w.alert()?;
    let accounts = index::load(app).ok()?.accounts;
    let number = |id: &str| accounts.iter().position(|a| a.id == id).map(|i| i as u32 + 1);
    Some(Overwrite {
        to: number(&w.to)?,
        from: number(&w.from)?,
        to_id: w.to.clone(),
        switched_at: w.at,
        at,
        overwrites: w.overwrites,
        running,
    })
}

/// The pool rule on claude-swap's numbers, as Firstmate's watch counts.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolCheck {
    pub dry: bool,
    pub active: Option<u32>,
    /// Percent left of the account in use; 100 when none is known.
    pub active_left: f64,
    /// The accounts the switcher may choose now besides the one in use,
    /// with their percent left.
    pub others: Vec<(u32, f64)>,
    /// The first reset among the usable accounts under 15 %, and whose.
    pub next_reset: Option<(u32, String)>,
    /// quota-axi's room for the whole rotation, in accounts.
    pub room: Option<f64>,
}

impl PoolCheck {
    /// The other account with the most left, when it has more than the one
    /// in use.
    pub fn better(&self) -> Option<(u32, f64)> {
        self.best_other().filter(|(_, left)| *left > self.active_left)
    }

    fn best_other(&self) -> Option<(u32, f64)> {
        self.others.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// `8-23` → (8, 23).
fn hours_of(text: &str) -> Option<(u8, u8)> {
    let (from, to) = text.split_once('-')?;
    Some((from.trim().parse().ok()?, to.trim().parse().ok()?))
}

/// An `expire` time: RFC 3339 from quota-axi, local `2026-10-16T17:45`
/// from the rotation file.
fn instant(text: &str) -> Option<OffsetDateTime> {
    parse_ts(text).or_else(|| local_time::instant(text))
}

/// Firstmate's dry rule, as its quota-pool watch counts it. Only the
/// accounts the switcher may choose now count: the Team accounts not past
/// their `expire`, and the personal account during its hours; never a
/// disabled one. An account past its reset counts as full. `None` without
/// claude-swap's accounts.
pub fn pool_check(
    report: &Response,
    rules: Option<&Rules>,
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<PoolCheck> {
    let row = report.primary("claude")?;
    let swap = row.claude_swap.as_ref().filter(|s| s.status == "ok")?;
    let defaults = Rules::defaults();
    let (team, personal, hours, expire) =
        match row.claude_pool.as_ref().and_then(|p| p.rotation.as_ref()) {
            Some(r) => (
                r.team.clone(),
                r.personal,
                r.personal_hours.as_deref().and_then(hours_of).unwrap_or((8, 23)),
                r.expire.iter().map(|e| (e.slot, instant(&e.at))).collect::<Vec<_>>(),
            ),
            None => {
                let r = rules.unwrap_or(&defaults);
                (
                    r.team.clone(),
                    r.personal,
                    (r.personal_from, r.personal_to),
                    r.expire.iter().map(|e| (e.slot, instant(&e.at))).collect(),
                )
            }
        };
    let gone: Vec<u32> =
        expire.iter().filter(|(_, at)| at.is_some_and(|t| t <= now)).map(|(s, _)| *s).collect();
    let hour = now.to_offset(offset).hour();
    let mut allowed: Vec<u32> = team.into_iter().filter(|s| !gone.contains(s)).collect();
    if let Some(p) = personal.filter(|p| hour >= hours.0 && hour <= hours.1 && !gone.contains(p)) {
        allowed.push(p);
    }
    let left = |s: &SwapSlot| {
        if s.status == "reset_pending" {
            100.0
        } else {
            s.effective_percent_remaining.unwrap_or(0.0)
        }
    };
    let active = swap.active_slot;
    let usable: Vec<&SwapSlot> =
        swap.slots.iter().filter(|s| !s.disabled && allowed.contains(&s.slot)).collect();
    let others: Vec<(u32, f64)> =
        usable.iter().filter(|s| Some(s.slot) != active).map(|s| (s.slot, left(s))).collect();
    let active_left =
        active.and_then(|a| swap.slots.iter().find(|s| s.slot == a)).map_or(100.0, left);
    let dry = active_left <= DRY_AT_MOST && !others.iter().any(|(_, l)| *l >= ROOM_AT_LEAST);
    let next_reset = usable
        .iter()
        .filter(|s| left(s) < ROOM_AT_LEAST)
        .filter_map(|s| {
            let at = s.resets_at.as_ref()?;
            parse_ts(at).map(|t| (t, s.slot, at.clone()))
        })
        .min_by_key(|(t, _, _)| *t)
        .map(|(_, slot, at)| (slot, at));
    let room = row
        .claude_pool
        .as_ref()
        .filter(|p| p.status == "pool")
        .and_then(|p| p.scopes.iter().find(|s| s.scope == "all_models").or(p.scopes.first()))
        .map(|s| s.slots_of_room);
    Some(PoolCheck { dry, active, active_left, others, next_reset, room })
}

/// Claude accounts by the names the windows give them: Cá nhân, then Team
/// A, B… in slot order; `Tài khoản N` without a rotation file.
pub fn claude_names(report: Option<&Response>) -> BTreeMap<u32, String> {
    let accounts = Snapshot::build(report, None).claude.accounts;
    let mut team: Vec<u32> =
        accounts.iter().filter(|a| a.role == Role::Team).map(|a| a.number).collect();
    team.sort_unstable();
    accounts
        .iter()
        .filter(|a| a.number > 0)
        .map(|a| {
            let name = match a.role {
                Role::Personal => "Cá nhân".to_string(),
                Role::Team => {
                    let i = team.iter().position(|n| *n == a.number).unwrap_or(0);
                    match "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().nth(i) {
                        Some(l) => format!("Team {l}"),
                        None => format!("Team {}", a.number),
                    }
                }
                Role::Unknown => format!("Tài khoản {}", a.number),
            };
            (a.number, name)
        })
        .collect()
}

fn agy_name(n: u32) -> String {
    format!("Ultra {n}")
}

// ===== Words =====

fn local(at: i64, offset: UtcOffset) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(at).unwrap_or(OffsetDateTime::UNIX_EPOCH).to_offset(offset)
}

/// `14:20` on this Mac's clock.
pub fn clock(at: i64, offset: UtcOffset) -> String {
    let t = local(at, offset);
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn weekday(w: Weekday) -> &'static str {
    match w {
        Weekday::Monday => "Thứ Hai",
        Weekday::Tuesday => "Thứ Ba",
        Weekday::Wednesday => "Thứ Tư",
        Weekday::Thursday => "Thứ Năm",
        Weekday::Friday => "Thứ Sáu",
        Weekday::Saturday => "Thứ Bảy",
        Weekday::Sunday => "Chủ nhật",
    }
}

/// `Chủ nhật 19/10`.
pub fn day(at: i64, offset: UtcOffset) -> String {
    let t = local(at, offset);
    format!("{} {:02}/{:02}", weekday(t.weekday()), t.day(), u8::from(t.month()))
}

/// `2 ngày`, `5 giờ`, `40 phút`.
fn span(secs: i64) -> String {
    if secs >= 86400 {
        format!("{} ngày", secs / 86400)
    } else if secs >= 3600 {
        format!("{} giờ", secs / 3600)
    } else {
        format!("{} phút", (secs / 60).max(1))
    }
}

/// `2,3`, the way the menu bar writes room.
fn room_text(room: f64) -> String {
    format!("{:.1}", (room * 10.0).round() / 10.0).replace('.', ",")
}

fn name_of(names: &BTreeMap<u32, String>, slot: u32) -> String {
    names.get(&slot).cloned().unwrap_or_else(|| format!("Tài khoản {slot}"))
}

/// The next morning at [`REMIND_HOUR`], this Mac's clock.
pub fn remind_at(now: i64, offset: UtcOffset) -> i64 {
    let today = local(now, offset).date();
    let date = today.next_day().unwrap_or(today);
    date.with_hms(REMIND_HOUR, 0, 0)
        .map(|t| t.assume_offset(offset).unix_timestamp())
        .unwrap_or(now + 86400)
}

// ===== The rules =====

fn pool_amount(c: &PoolCheck, names: &BTreeMap<u32, String>) -> String {
    match (c.room, c.active) {
        (Some(room), _) => format!("Còn tương đương {} tài khoản", room_text(room)),
        (None, Some(a)) => format!("{} đã dùng {}%", name_of(names, a), (100.0 - c.active_left).round()),
        (None, None) => "Hạn mức Claude gần hết".to_string(),
    }
}

fn pool_alerts(
    c: &PoolCheck,
    was_dry: bool,
    names: &BTreeMap<u32, String>,
    f: &Facts,
) -> Option<Alert> {
    let now = f.now.unix_timestamp();
    if c.dry && !was_dry {
        let mut body = pool_amount(c, names) + ".";
        if let Some((slot, at)) = &c.next_reset {
            if let Some(t) = parse_ts(at) {
                body += &format!(
                    " {} có lại hạn mức lúc {}.",
                    name_of(names, *slot),
                    clock(t.unix_timestamp(), f.offset)
                );
            }
        }
        let mut actions = Vec::new();
        if let Some((slot, left)) = c.better() {
            actions.push(act(
                format!("switch:{slot}"),
                format!("Sang {} (đã dùng {}%)", name_of(names, slot), (100.0 - left).round()),
            ));
        }
        actions.push(act("dismiss", "Thôi"));
        return Some(Alert {
            key: format!("pool:dry:{now}"),
            kind: Kind::Pool,
            tone: Tone::Bad,
            title: "Hạn mức Claude trong vòng sắp cạn".into(),
            body,
            actions,
            at: now,
            test: false,
        });
    }
    if !c.dry && was_dry {
        let body = match c.room {
            Some(room) => format!("Còn tương đương {} tài khoản trong vòng.", room_text(room)),
            None => pool_amount(c, names) + ".",
        };
        return Some(Alert {
            key: format!("pool:back:{now}"),
            kind: Kind::Pool,
            tone: Tone::Ok,
            title: "Claude đã có lại hạn mức".into(),
            body,
            actions: Vec::new(),
            at: now,
            test: false,
        });
    }
    None
}

fn leaving_alerts(
    report: &Response,
    check: Option<&PoolCheck>,
    names: &BTreeMap<u32, String>,
    f: &Facts,
) -> Vec<Alert> {
    let now = f.now.unix_timestamp();
    // quota-axi's rotation, else the rotation file this Mac reads, as for the pool.
    let expire: Vec<(u32, &str)> = match report
        .primary("claude")
        .and_then(|r| r.claude_pool.as_ref())
        .and_then(|p| p.rotation.as_ref())
    {
        Some(r) => r.expire.iter().map(|e| (e.slot, e.at.as_str())).collect(),
        None => f
            .rules
            .map_or_else(Vec::new, |r| r.expire.iter().map(|e| (e.slot, e.at.as_str())).collect()),
    };
    let active = check.and_then(|c| c.active);
    expire
        .into_iter()
        .filter_map(|(slot, at)| {
            let at = instant(at)?.unix_timestamp();
            if !(now < at && at - now <= LEAVING_BEFORE) {
                return None;
            }
            let name = name_of(names, slot);
            let mut body = format!("Còn {}.", span(at - now));
            let mut actions = vec![act("dismiss", "Ổn")];
            if active == Some(slot) {
                body += " Sau đó switcher chuyển sang tài khoản khác trong vòng.";
                if let Some((slot, _)) = check.and_then(PoolCheck::best_other) {
                    actions.push(act(format!("switch:{slot}"), "Chuyển ngay"));
                }
            } else {
                body += " Sau đó tài khoản này không còn trong vòng.";
            }
            Some(Alert {
                key: format!("leave:{slot}:{at}"),
                kind: Kind::Leaving,
                tone: Tone::Warn,
                title: format!("{name} rời vòng lúc {}", clock(at, f.offset)),
                body,
                actions,
                at: now,
                test: false,
            })
        })
        .collect()
}

/// A claude-swap error that only a new login mends, as one of the known
/// codes (`relogin` for any other that asks for one). The card and its key
/// carry only this code, never claude-swap's own text.
pub fn relogin_code(error: &str) -> Option<&'static str> {
    let e = error.trim().to_ascii_lowercase();
    if let Some(code) = RELOGIN_ERRORS.iter().find(|c| **c == e) {
        Some(code)
    } else if e.contains("relogin") || e.contains("re-login") {
        Some("relogin")
    } else {
        None
    }
}

pub fn relogin_error(error: &str) -> bool {
    relogin_code(error).is_some()
}

const CLAUDE_RELOGIN: &str = "Đăng nhập lại bằng claude-swap trong Terminal.";

fn login_alerts(report: &Response, names: &BTreeMap<u32, String>, f: &Facts) -> Vec<Alert> {
    let now = f.now.unix_timestamp();
    let Some(swap) = report.primary("claude").and_then(|r| r.claude_swap.as_ref()) else {
        return Vec::new();
    };
    if swap.status != "ok" {
        return Vec::new();
    }
    let mut out = Vec::new();
    for slot in &swap.slots {
        let name = name_of(names, slot.slot);
        let actions = vec![act("snooze", "Nhắc ngày mai"), act("dismiss", "Đã biết")];
        let ends = slot.login_expires_at.as_deref().and_then(|t| parse_ts(t).map(|p| (t, p)));
        if let Some((text, at)) = ends.filter(|(_, at)| at.unix_timestamp() - now <= LOGIN_WITHIN) {
            let at = at.unix_timestamp();
            let body = if at > now {
                format!(
                    "Đăng nhập hết hạn sau {} ({}). {CLAUDE_RELOGIN}",
                    span(at - now),
                    day(at, f.offset)
                )
            } else {
                format!(
                    "Đăng nhập đã hết hạn lúc {} {}. {CLAUDE_RELOGIN}",
                    clock(at, f.offset),
                    day(at, f.offset)
                )
            };
            out.push(Alert {
                key: format!("login:claude:{}:expires:{text}", slot.slot),
                kind: Kind::Login,
                tone: Tone::Warn,
                title: format!("{name} cần đăng nhập lại"),
                body,
                actions,
                at: now,
                test: false,
            });
        } else if let Some(error) = slot.error.as_deref().and_then(relogin_code) {
            out.push(Alert {
                key: format!("login:claude:{}:error:{error}", slot.slot),
                kind: Kind::Login,
                tone: Tone::Warn,
                title: format!("{name} cần đăng nhập lại"),
                body: format!(
                    "claude-swap không làm mới được đăng nhập của tài khoản này ({error}). {CLAUDE_RELOGIN}"
                ),
                actions,
                at: now,
                test: false,
            });
        }
    }
    out
}

fn agy_login_alerts(logins: &[AgyLogin], now: i64) -> Vec<Alert> {
    logins
        .iter()
        .map(|l| {
            let name = agy_name(l.number);
            let (state, since, title, body, url) = match &l.standing {
                Standing::NeedsLogin { since } => (
                    "login",
                    *since,
                    format!("{name} cần đăng nhập lại"),
                    "Google không nhận đăng nhập của tài khoản này nữa. Đăng nhập lại trong tab Antigravity.".to_string(),
                    false,
                ),
                Standing::NeedsVerification { since, url } => (
                    "verify",
                    *since,
                    format!("{name} cần xác minh"),
                    "Google cần xác minh tài khoản này trước khi cho đọc hạn mức.".to_string(),
                    url.is_some(),
                ),
                Standing::Forbidden { since, url } => (
                    "forbidden",
                    *since,
                    format!("{name} cần đăng nhập lại"),
                    "Google chặn đọc hạn mức của tài khoản này. App đọc lại sau khi đăng nhập lại.".to_string(),
                    url.is_some(),
                ),
            };
            let mut actions = Vec::new();
            if url {
                actions.push(act(format!("agy:verify:{}", l.number), "Mở trang xác minh"));
            }
            if state != "verify" {
                actions.push(act("agy:login", "Đăng nhập lại"));
            }
            actions.push(act("dismiss", "Đã biết"));
            Alert {
                key: format!("login:agy:{}:{state}:{since}", l.id),
                kind: Kind::Login,
                tone: Tone::Warn,
                title,
                body,
                actions,
                at: now,
                test: false,
            }
        })
        .collect()
}

fn overwrite_alert(o: &Overwrite, f: &Facts) -> Alert {
    let now = f.now.unix_timestamp();
    let (to, from) = (agy_name(o.to), agy_name(o.from));
    let mut body = format!(
        "Lúc {} Antigravity.app đổi về {from}. Bạn đã chuyển sang {to} lúc {}.",
        clock(o.at, f.offset),
        clock(o.switched_at, f.offset)
    );
    if o.overwrites >= 2 {
        body += &format!(" Đây là lần thứ {}.", o.overwrites);
    }
    let mut actions = Vec::new();
    if o.overwrites <= OVERWRITE_OFFERS {
        match o.running {
            Some(true) => {
                actions.push(act(format!("agy:quitFirst:{}", o.to), "Thoát app rồi chuyển lại"));
                actions.push(act(format!("agy:keepRunning:{}", o.to), "Chuyển lại"));
            }
            Some(false) => actions.push(act(format!("agy:plain:{}", o.to), "Chuyển lại")),
            None => actions.push(act(format!("agy:keepRunning:{}", o.to), "Chuyển lại")),
        }
    } else {
        body += " App thôi đề nghị chuyển lại.";
    }
    actions.push(act("agy:keep", "Thôi"));
    Alert {
        key: format!("overwrite:{}:{}", o.to_id, o.at),
        kind: Kind::Overwrite,
        tone: Tone::Warn,
        title: "Antigravity đổi về tài khoản cũ".into(),
        body,
        actions,
        at: now,
        test: false,
    }
}

fn firstmate_alert(reading: &firstmate::Reading, f: &Facts) -> Alert {
    let now = f.now.unix_timestamp();
    let mut body =
        "Hạn mức trong vòng dưới ngưỡng: Firstmate giữ việc Claude mới đến khi có lại.".to_string();
    if let Some(t) = reading.line.next_reset.as_deref().and_then(parse_ts) {
        body += &format!(" Có lại lúc {}.", clock(t.unix_timestamp(), f.offset));
    }
    Alert {
        key: format!("firstmate:dry:{now}"),
        kind: Kind::Firstmate,
        tone: Tone::Info,
        title: "Firstmate đang chờ hạn mức Claude".into(),
        body,
        actions: Vec::new(),
        at: now,
        test: false,
    }
}

/// Every alert whose event holds now. Alerts 1 and 5 come only on their
/// edges, which `memory` keeps.
fn found(memory: &mut Memory, f: &Facts) -> Vec<Alert> {
    let now = f.now.unix_timestamp();
    let names = claude_names(f.claude);
    let check = f.claude.and_then(|r| pool_check(r, f.rules, f.now, f.offset));
    let mut out = Vec::new();
    if let Some(c) = &check {
        out.extend(pool_alerts(c, memory.pool_dry.unwrap_or(false), &names, f));
        memory.pool_dry = Some(c.dry);
    }
    if let Some(report) = f.claude {
        out.extend(leaving_alerts(report, check.as_ref(), &names, f));
        out.extend(login_alerts(report, &names, f));
    }
    out.extend(agy_login_alerts(&f.agy, now));
    out.extend(f.overwrite.as_ref().map(|o| overwrite_alert(o, f)));
    if let Some(reading) = f.firstmate {
        if let Some(dry) = reading.holding(now) {
            if dry && !memory.firstmate_dry.unwrap_or(false) {
                out.push(firstmate_alert(reading, f));
            }
            memory.firstmate_dry = Some(dry);
        }
    }
    out
}

/// Runs the rules on the newest readings. Returns the ids of the cards to
/// show now, each kept in `memory.recent`: alerts of kinds turned off are
/// left out, a key already sent is not sent again, and a reminder whose
/// time came is sent again while its event still holds.
pub fn evaluate(memory: &mut Memory, facts: &Facts, switches: &Switches) -> Vec<String> {
    let now = facts.now.unix_timestamp();
    let found = found(memory, facts);
    let holding: BTreeSet<String> = found.iter().map(|a| a.key.clone()).collect();
    // A dead login whose error went away (logged in again) may come back
    // later under the same key; forget it, but only while the accounts are read.
    let claude_read = facts
        .claude
        .and_then(|r| r.primary("claude"))
        .and_then(|r| r.claude_swap.as_ref())
        .is_some_and(|s| s.status == "ok");
    if claude_read {
        memory.sent.retain(|k, _| {
            !(k.starts_with("login:claude:") && k.contains(":error:")) || holding.contains(k)
        });
    }
    let mut shown = Vec::new();
    for alert in found {
        if !switches.on(alert.kind) {
            continue;
        }
        let due = memory.snoozed.get(&alert.key).is_some_and(|t| *t <= now);
        if memory.sent.contains_key(&alert.key) && !due {
            continue;
        }
        memory.snoozed.remove(&alert.key);
        memory.sent.entry(alert.key.clone()).or_insert(now);
        let id = format!("{}@{now}", alert.key);
        memory.recent.retain(|r| r.id != id);
        memory.recent.insert(0, Record { id: id.clone(), alert, notified: false, answer: None });
        shown.push(id);
    }
    memory.snoozed.retain(|k, _| holding.contains(k));
    memory.sent.retain(|_, at| now - *at <= SENT_KEEP);
    memory.recent.truncate(RECENT_KEEP);
    shown
}

// ===== Cards and buttons =====

/// A card for macOS, as the notification center shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub body: String,
    pub actions: Vec<Act>,
}

impl Record {
    pub fn card(&self) -> Card {
        Card {
            id: self.id.clone(),
            kind: self.alert.kind,
            title: self.alert.title.clone(),
            body: self.alert.body.clone(),
            actions: self.alert.actions.clone(),
        }
    }
}

/// The macOS notification center, or a fake one in the tests. A button
/// pressed on a card comes back through [`answer`] with the card's id.
pub trait Notifier: Send + Sync {
    fn show(&self, card: &Card) -> Result<(), String>;
    /// Takes a card off screen once it was answered in the app.
    fn withdraw(&self, id: &str);
}

/// Shows the cards `ids`, noting the ones macOS took.
pub fn deliver(memory: &mut Memory, ids: &[String], center: &dyn Notifier) {
    for id in ids {
        if let Some(r) = memory.recent.iter_mut().find(|r| &r.id == id) {
            r.notified = center.show(&r.card()).is_ok();
        }
    }
}

/// What the app does for a button, beyond noting the answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    /// The switch sheet for this Claude account, to confirm there.
    OpenClaudeSwitch {
        slot: u32,
    },
    /// Switch Antigravity back to this account; [`settle`] takes how it went.
    SwitchAgy {
        number: u32,
        how: How,
    },
    /// Stop the six-hour watch.
    DismissOverwrite,
    /// The Antigravity tab, where the account logs in again.
    OpenAgyTab,
    OpenVerification {
        number: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerError {
    /// No card with this id is kept.
    Unknown,
    /// Pressed already, here or on the other card.
    Answered,
    NoSuchAction,
}

impl AnswerError {
    pub fn code(&self) -> &'static str {
        match self {
            AnswerError::Unknown => "unknown",
            AnswerError::Answered => "answered",
            AnswerError::NoSuchAction => "noSuchAction",
        }
    }
}

fn how(word: &str) -> Option<How> {
    match word {
        "plain" => Some(How::Plain),
        "quitFirst" => Some(How::QuitFirst),
        "keepRunning" => Some(How::KeepRunning),
        _ => None,
    }
}

/// The button `action` on card `id`, pressed at `now`, on the macOS card or
/// the one in the app: notes the answer (once per card), sets a reminder
/// for "Nhắc ngày mai", and says what else the app must do.
pub fn answer(
    memory: &mut Memory,
    id: &str,
    action: &str,
    now: i64,
    offset: UtcOffset,
) -> Result<Effect, AnswerError> {
    let Memory { recent, snoozed, .. } = memory;
    let record = recent.iter_mut().find(|r| r.id == id).ok_or(AnswerError::Unknown)?;
    if record.answer.is_some() {
        return Err(AnswerError::Answered);
    }
    let pressed = record
        .alert
        .actions
        .iter()
        .find(|a| a.id == action)
        .cloned()
        .ok_or(AnswerError::NoSuchAction)?;
    let a = &record.alert;
    let (effect, said) = if a.test {
        (
            Effect::None,
            Some(said(
                Tone::Info,
                "Thông báo thử",
                format!("Bạn đã bấm “{}”. Thông báo thử không làm gì cả.", pressed.label),
            )),
        )
    } else if action == "dismiss" {
        let said = match a.kind {
            Kind::Pool => said(
                Tone::Info,
                "Đã bỏ qua cảnh báo này",
                "Không lặp lại cho cùng sự kiện. Quota Station báo lại khi hạn mức trong vòng có lại.",
            ),
            Kind::Leaving => {
                said(Tone::Info, "Đã ghi nhận", "Không nhắc lại cho lần rời vòng này.")
            }
            _ => said(Tone::Info, "Đã ghi nhận", "Không nhắc lại cho lần này."),
        };
        (Effect::None, Some(said))
    } else if action == "snooze" {
        let at = remind_at(now, offset);
        snoozed.insert(a.key.clone(), at);
        let said = said(
            Tone::Info,
            format!("Sẽ nhắc lại lúc {} ngày mai", clock(at, offset)),
            format!("{}.", day(at, offset)),
        );
        (Effect::None, Some(said))
    } else if action == "agy:keep" {
        let said = said(
            Tone::Info,
            "Đã bỏ qua",
            "Antigravity giữ tài khoản đang dùng. App thôi canh lần chuyển này.",
        );
        (Effect::DismissOverwrite, Some(said))
    } else if action == "agy:login" {
        let said = said(
            Tone::Info,
            "Đã mở tab Antigravity",
            "Bấm Đăng nhập lại ở dòng của tài khoản này.",
        );
        (Effect::OpenAgyTab, Some(said))
    } else if let Some(n) = action.strip_prefix("agy:verify:").and_then(|n| n.parse().ok()) {
        let said =
            said(Tone::Info, "Đã mở trang xác minh", "Trang của Google mở trong trình duyệt.");
        (Effect::OpenVerification { number: n }, Some(said))
    } else if let Some(slot) = action.strip_prefix("switch:").and_then(|n| n.parse().ok()) {
        let said = said(
            Tone::Info,
            "Đã mở tấm hỏi chuyển",
            "Xác nhận trên tấm để chuyển. Chưa có gì thay đổi.",
        );
        (Effect::OpenClaudeSwitch { slot }, Some(said))
    } else if let Some((h, n)) = action.strip_prefix("agy:").and_then(|r| r.split_once(':')) {
        match (how(h), n.parse()) {
            (Some(how), Ok(number)) => (Effect::SwitchAgy { number, how }, None),
            _ => return Err(AnswerError::NoSuchAction),
        }
    } else {
        return Err(AnswerError::NoSuchAction);
    };
    record.answer =
        Some(Answer { action: pressed.id, label: pressed.label, at: now, result: said });
    Ok(effect)
}

/// [`answer`] for a button pressed on the card in the app: once the answer
/// is taken, the macOS card has nothing left to ask and is withdrawn.
pub fn answer_in_app(
    memory: &mut Memory,
    id: &str,
    action: &str,
    now: i64,
    offset: UtcOffset,
    center: &dyn Notifier,
) -> Result<Effect, AnswerError> {
    let effect = answer(memory, id, action, now, offset)?;
    center.withdraw(id);
    Ok(effect)
}

/// How a switch started from a card ended.
pub fn settle(memory: &mut Memory, id: &str, said: Said) {
    if let Some(answer) =
        memory.recent.iter_mut().find(|r| r.id == id).and_then(|r| r.answer.as_mut())
    {
        answer.result = Some(said);
    }
}

/// The line under the card after "Chuyển lại".
pub fn agy_result(outcome: &AgyOutcome) -> Said {
    match outcome {
        AgyOutcome::Switched { number, reopened, .. } => said(
            Tone::Ok,
            format!("Đã chuyển lại sang {}", agy_name(*number as u32)),
            match reopened {
                Some(true) => "Antigravity.app đã mở lại với tài khoản này.",
                Some(false) => "Chưa mở lại được Antigravity.app; mở nó bằng tay.",
                None => "Antigravity.app vẫn mở nên có thể đổi về lần nữa. Nếu vậy, app báo thêm.",
            },
        ),
        AgyOutcome::Unconfirmed { number, .. } => said(
            Tone::Warn,
            format!("Đã ghi đăng nhập {}", agy_name(*number as u32)),
            "agy chưa cho thấy tài khoản mới. Xem lại tab Antigravity sau ít phút.",
        ),
        AgyOutcome::Failed { number, error, .. } => said(
            Tone::Bad,
            format!("Chưa chuyển lại được sang {}", agy_name(*number as u32)),
            format!("Lý do: {error}. Không có gì thay đổi."),
        ),
    }
}

/// A switch from a card that could not start.
pub fn agy_refused(number: u32, why: &str) -> Said {
    said(
        Tone::Bad,
        format!("Chưa chuyển lại được sang {}", agy_name(number)),
        format!("Lý do: {why}. Không có gì thay đổi."),
    )
}

/// The card "Gửi thử" sends for `kind`: the prototype's words, marked as a
/// test, with buttons that change nothing.
pub fn sample(kind: Kind, now: i64, offset: UtcOffset) -> Alert {
    let later = clock(now + 2 * 3600, offset);
    let (tone, title, body, actions): (Tone, String, String, Vec<Act>) = match kind {
        Kind::Pool => (
            Tone::Bad,
            "Hạn mức Claude trong vòng sắp cạn".into(),
            format!("Thử: còn tương đương 0,4 tài khoản. Team B có lại hạn mức lúc {later}."),
            vec![act("switch:0", "Sang Cá nhân (đã dùng 81%)"), act("dismiss", "Thôi")],
        ),
        Kind::Leaving => (
            Tone::Warn,
            format!("Team A rời vòng lúc {}", clock(now + 30 * 60, offset)),
            "Thử: còn 30 phút. Sau đó switcher chuyển sang tài khoản khác trong vòng.".into(),
            vec![act("dismiss", "Ổn"), act("switch:0", "Chuyển ngay")],
        ),
        Kind::Login => (
            Tone::Warn,
            "Team B cần đăng nhập lại".into(),
            format!(
                "Thử: đăng nhập hết hạn sau 2 ngày ({}). {CLAUDE_RELOGIN}",
                day(now + 2 * 86400, offset)
            ),
            vec![act("snooze", "Nhắc ngày mai"), act("dismiss", "Đã biết")],
        ),
        Kind::Overwrite => (
            Tone::Warn,
            "Antigravity đổi về tài khoản cũ".into(),
            format!(
                "Thử: lúc {} Antigravity.app đổi về Ultra 1. Bạn đã chuyển sang Ultra 2 lúc {}.",
                clock(now, offset),
                clock(now - 20 * 60, offset)
            ),
            vec![
                act("agy:quitFirst:0", "Thoát app rồi chuyển lại"),
                act("agy:keepRunning:0", "Chuyển lại"),
                act("agy:keep", "Thôi"),
            ],
        ),
        Kind::Firstmate => (
            Tone::Info,
            "Firstmate đang chờ hạn mức Claude".into(),
            format!("Thử: hạn mức trong vòng dưới ngưỡng. Có lại lúc {later}."),
            Vec::new(),
        ),
    };
    Alert {
        key: format!("test:{}:{now}", kind.number()),
        kind,
        tone,
        title,
        body,
        actions,
        at: now,
        test: true,
    }
}

/// Keeps a test card in `memory.recent` and returns its id.
pub fn keep_test(memory: &mut Memory, alert: Alert) -> String {
    let id = format!("{}@{}", alert.key, alert.at);
    memory.recent.retain(|r| r.id != id);
    memory.recent.insert(0, Record { id: id.clone(), alert, notified: false, answer: None });
    memory.recent.truncate(RECENT_KEEP);
    id
}
