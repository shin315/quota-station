// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's policy of the Claude pool, which the app knows only
//! through the hooks ([`crate::ext::Hooks::claude_pool`]): the rotation.
//!
//! - At a read of the accounts the app keeps, the rotation file's keys and
//!   the rotation's room go into the report's accounts, in their keys of
//!   the extension's own (`rotation`, `pool`, `poolReason`: [`Rotation`],
//!   [`RoomPool`]).
//! - In alert 1's check only the accounts the switcher may choose now
//!   count: the Team accounts not past their `expire`, and the personal
//!   account during its hours, from the report's rotation, else the
//!   rotation file; never a disabled one.
//! - Alert 1's card speaks of the rotation's quota once "Bỏ qua" is
//!   pressed ([`ANSWERS`]).
//!
//! [`PoolHooks`] gives the core these; [`Policy`] is the policy for one
//! read or one pass of the alert rules.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset};

use crate::alerts::ExtAnswer;
use crate::claude::Paths;
use crate::ext::Hooks;
use crate::local_time;
use crate::quota::claude::PoolPolicy;
use crate::quota::{iso, ClaudeAccounts, Provider, Slot, Window};
use crate::rotation::{self, FileState, Rules};
use crate::time_util::parse_ts;

/// The report's key of the rotation file's keys the windows show
/// ([`Rotation`]).
pub const ROTATION: &str = "rotation";
/// The report's key of the rotation's room ([`RoomPool`]).
pub const POOL: &str = "pool";
/// The report's key of why the rotation's room is not counted
/// (`claude_row_not_fresh`, `no_other_slot`).
pub const POOL_REASON: &str = "poolReason";

/// The rotation file's keys the windows show.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rotation {
    #[serde(default)]
    pub team: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_hours: Option<String>,
    #[serde(default)]
    pub full: f64,
    #[serde(default)]
    pub expire: Vec<Expire>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expire {
    pub slot: u32,
    /// RFC 3339.
    pub at: String,
}

/// The room the rotation has: each account it may move to now, with the
/// room it has before the switch threshold.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomPool {
    /// `all_models`.
    pub scope: String,
    pub members: u32,
    /// Room left, in fresh accounts (2.3 is two and a third).
    pub slots_of_room: f64,
    /// The room's sum in percent of one account, at most 100.
    pub effective_percent_remaining: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_refill_at: Option<String>,
    #[serde(default)]
    pub slots: Vec<RoomSlot>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSlot {
    pub slot: u32,
    /// 100 is a whole account's worth of room, 0 an account at its threshold.
    pub room_percent: f64,
    /// The first reset that gives it more room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refill_at: Option<String>,
}

/// The rotation in the report's accounts, when the read wrote one.
pub fn rotation_in(accounts: &ClaudeAccounts) -> Option<Rotation> {
    read(accounts, ROTATION)
}

/// The rotation's room in the report's accounts, when the read counted one.
pub fn pool_in(accounts: &ClaudeAccounts) -> Option<RoomPool> {
    read(accounts, POOL)
}

/// Why the report's accounts have no room counted, when the read said.
pub fn pool_reason_in(accounts: &ClaudeAccounts) -> Option<String> {
    read(accounts, POOL_REASON)
}

fn read<T: for<'de> Deserialize<'de>>(accounts: &ClaudeAccounts, key: &str) -> Option<T> {
    serde_json::from_value(accounts.ext.get(key)?.clone()).ok()
}

/// Puts `key` in the report's accounts, or takes it out for `None`.
fn write<T: Serialize>(accounts: &mut ClaudeAccounts, key: &str, value: Option<T>) {
    match value.and_then(|v| serde_json::to_value(v).ok()) {
        Some(value) => accounts.ext.insert(key, value),
        None => {
            accounts.ext.remove(key);
        }
    }
}

const ACCOUNT_WINDOWS: [&str; 2] = ["five_hour", "seven_day"];

/// Alert 1's words once "Bỏ qua" is pressed: it comes back when the
/// rotation's quota does.
pub const ANSWERS: [ExtAnswer; 1] = [ExtAnswer {
    action: "dismiss",
    title: "Đã bỏ qua thông báo này",
    body: "Không lặp lại cho cùng sự kiện. Quota Station báo lại khi hạn mức trong vòng có lại.",
}];

/// The advanced part's hooks for the Claude pool: the rotation's policy,
/// read from the rotation file, and alert 1's words for it. The app's
/// extension gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct PoolHooks;

impl Hooks for PoolHooks {
    fn claude_pool(&self, paths: &Paths) -> Option<Box<dyn PoolPolicy>> {
        Some(Box::new(Policy::read(paths)))
    }

    fn pool_answers(&self) -> &'static [ExtAnswer] {
        &ANSWERS
    }
}

/// The rotation as the policy of the Claude pool: the rotation file's
/// rules, when it read. Without them the report's rotation still decides
/// who counts in alert 1's check, and a disabled account never does.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Policy {
    pub rules: Option<Rules>,
}

impl Policy {
    /// The policy with the rotation file the app follows where it keeps the
    /// accounts at `paths`.
    pub fn read(paths: &Paths) -> Policy {
        Policy { rules: rules(paths) }
    }
}

/// The rules of the rotation file the app follows where it keeps the
/// accounts at `paths`, when it reads.
pub fn rules(paths: &Paths) -> Option<Rules> {
    match rotation::load(&paths.rotation_file()) {
        FileState::Ok { parsed, .. } => Some(parsed.rules),
        _ => None,
    }
}

impl PoolPolicy for Policy {
    /// With the rotation file's rules, its keys and the rotation's room,
    /// or why there is no room to count; without them, nothing.
    fn room(&self, row: &Provider, accounts: &mut ClaudeAccounts, now: OffsetDateTime) {
        let Some(rules) = &self.rules else { return };
        let (pool, pool_reason) = match room(rules, row, &accounts.slots, accounts.active_slot, now)
        {
            Ok(pool) => (Some(pool), None),
            Err(reason) => (None, Some(reason.to_string())),
        };
        write(accounts, ROTATION, Some(rotation_of(rules)));
        write(accounts, POOL, pool);
        write::<Value>(accounts, POOL_REASON, pool_reason.map(Value::String));
    }

    fn counts(
        &self,
        accounts: &ClaudeAccounts,
        slot: &Slot,
        now: OffsetDateTime,
        offset: UtcOffset,
    ) -> bool {
        !super::held::slot(slot)
            && allowed(accounts, self.rules.as_ref(), now, offset)
                .is_none_or(|allowed| allowed.contains(&slot.slot))
    }
}

/// `8-23` → (8, 23).
fn hours_of(text: &str) -> Option<(u8, u8)> {
    let (from, to) = text.split_once('-')?;
    Some((from.trim().parse().ok()?, to.trim().parse().ok()?))
}

/// An `expire` time: RFC 3339 from the report, local `2026-10-16T17:45`
/// from the rotation file.
fn instant(text: &str) -> Option<OffsetDateTime> {
    parse_ts(text).or_else(|| local_time::instant(text))
}

/// The accounts the switcher may choose at `now`, from the report's
/// rotation, else `rules`: the Team accounts not past their `expire`, and
/// the personal account during its hours. `None` without a rotation:
/// every account.
fn allowed(
    accounts: &ClaudeAccounts,
    rules: Option<&Rules>,
    now: OffsetDateTime,
    offset: UtcOffset,
) -> Option<Vec<u32>> {
    let (team, personal, hours, expire) = match rotation_in(accounts) {
        Some(r) => (
            r.team,
            r.personal,
            r.personal_hours.as_deref().and_then(hours_of).unwrap_or((8, 23)),
            r.expire.iter().map(|e| (e.slot, instant(&e.at))).collect::<Vec<_>>(),
        ),
        None => rules.map(|r| {
            (
                r.team.clone(),
                r.personal,
                (r.personal_from, r.personal_to),
                r.expire.iter().map(|e| (e.slot, instant(&e.at))).collect(),
            )
        })?,
    };
    let gone: Vec<u32> =
        expire.iter().filter(|(_, at)| at.is_some_and(|t| t <= now)).map(|(s, _)| *s).collect();
    let hour = now.to_offset(offset).hour();
    let mut allowed: Vec<u32> = team.into_iter().filter(|s| !gone.contains(s)).collect();
    if let Some(p) = personal.filter(|p| hour >= hours.0 && hour <= hours.1 && !gone.contains(p)) {
        allowed.push(p);
    }
    Some(allowed)
}

fn rotation_of(rules: &Rules) -> Rotation {
    let mut expire: Vec<Expire> = rules
        .expire
        .iter()
        .filter_map(|e| Some(Expire { slot: e.slot, at: iso(instant(&e.at)?) }))
        .collect();
    expire.sort_by_key(|e| e.slot);
    Rotation {
        team: rules.team.clone(),
        personal: rules.personal,
        personal_hours: Some(format!("{}-{}", rules.personal_from, rules.personal_to)),
        full: rules.full,
        expire,
    }
}

struct Member<'a> {
    slot: u32,
    windows: &'a [Window],
    leaves: Option<OffsetDateTime>,
}

/// Percent used now: none past its reset.
fn used_now(w: &Window, now: OffsetDateTime) -> Option<f64> {
    if w.reset_passed(now) {
        return Some(0.0);
    }
    w.percent_used.or(w.percent_remaining.map(|r| 100.0 - r))
}

fn round(v: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (v * f + 0.5).floor() / f
}

/// The room the rotation has in every model's scope: for each account it
/// may move to now (the Team accounts before their end, the personal one in
/// its hours, each with a reading at most 15 minutes old, the one in use
/// from the row), the room before `full`, as a percent of that room, in the
/// tighter of the five-hour and weekly windows; and the first reset that
/// gives an account more room. `Err` says why there is no rotation to count.
fn room(
    rules: &Rules,
    row: &Provider,
    slots: &[Slot],
    active: Option<u32>,
    now: OffsetDateTime,
) -> Result<RoomPool, &'static str> {
    if row.state.status != "fresh" {
        return Err("claude_row_not_fresh");
    }
    let active = active.ok_or("active_slot_unknown")?;
    let known = row.quota_semantics.as_ref().is_some_and(|s| s.status == "known");
    if !known {
        return Err("claude_row_unmeasured");
    }
    let offset = local_time::offset_at(now);
    let local = now.to_offset(offset);
    let in_hours = rules.personal_from <= local.hour() && local.hour() <= rules.personal_to;
    let hours_end = local.replace_time(time::Time::MIDNIGHT)
        + time::Duration::hours(i64::from(rules.personal_to) + 1);
    let mut candidates: Vec<(u32, bool)> = rules.team.iter().map(|s| (*s, false)).collect();
    if let Some(p) = rules.personal.filter(|p| !rules.team.contains(p)) {
        candidates.push((p, true));
    }
    let mut members = Vec::new();
    for (slot, personal) in candidates {
        let ends = rules.expire.iter().find(|e| e.slot == slot).and_then(|e| instant(&e.at));
        if ends.is_some_and(|e| now >= e) || (personal && !in_hours) {
            continue;
        }
        let Some(s) = slots.iter().find(|s| s.slot == slot) else { continue };
        let windows: &[Window] = if slot == active {
            &row.windows
        } else if s.status == "stale" || s.status == "no_data" {
            continue;
        } else {
            &s.windows
        };
        if windows.is_empty() {
            continue;
        }
        let leaves = [ends, personal.then_some(hours_end)].into_iter().flatten().min();
        members.push(Member { slot, windows, leaves });
    }
    if !members.iter().any(|m| m.slot != active) {
        return Err("no_other_slot");
    }
    let rooms: Vec<(u32, f64, Option<OffsetDateTime>)> = members
        .iter()
        .map(|m| {
            let (left, refill) = member_room(m, rules.full, now);
            (m.slot, left, refill)
        })
        .collect();
    let total: f64 = rooms.iter().map(|r| r.1).sum();
    let next = rooms.iter().filter_map(|r| r.2).min();
    Ok(RoomPool {
        scope: "all_models".into(),
        members: u32::try_from(members.len()).unwrap_or(u32::MAX),
        slots_of_room: round(total / 100.0, 2),
        effective_percent_remaining: round(total.min(100.0), 1),
        next_refill_at: next.map(iso),
        slots: rooms
            .iter()
            .map(|(slot, left, refill)| RoomSlot {
                slot: *slot,
                room_percent: round(*left, 1),
                refill_at: refill.map(iso),
            })
            .collect(),
    })
}

/// One account's room in percent, and the first reset before it leaves the
/// rotation after which it has more.
fn member_room(m: &Member, full: f64, now: OffsetDateTime) -> (f64, Option<OffsetDateTime>) {
    let windows: Vec<&Window> =
        m.windows.iter().filter(|w| ACCOUNT_WINDOWS.contains(&w.id.as_str())).collect();
    let percent: Vec<f64> = windows
        .iter()
        .map(|w| used_now(w, now).map_or(0.0, |u| 100.0 * (full - u).max(0.0) / full))
        .collect();
    let left = percent.iter().copied().fold(f64::INFINITY, f64::min);
    let left = if left.is_finite() { left } else { 0.0 };
    let mut resets: Vec<OffsetDateTime> = windows
        .iter()
        .filter_map(|w| w.resets_at.as_deref().and_then(instant))
        .filter(|t| *t > now)
        .collect();
    resets.sort();
    resets.dedup();
    for reset in resets {
        if m.leaves.is_some_and(|l| reset >= l) {
            break;
        }
        let after = windows
            .iter()
            .zip(&percent)
            .map(|(w, p)| {
                let r = w.resets_at.as_deref().and_then(instant);
                if r.is_some_and(|r| r > now && r <= reset) {
                    100.0
                } else {
                    *p
                }
            })
            .fold(f64::INFINITY, f64::min);
        if after > left {
            return (left, Some(reset));
        }
    }
    (left, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quota::parse_iso;

    fn window(id: &str, used: f64, resets: Option<&str>) -> Window {
        let kind = if id == "five_hour" { "session" } else { "weekly" };
        Window::used(id, kind, used, resets.map(String::from))
    }

    #[test]
    fn an_accounts_room_is_its_tighter_window_before_full() {
        let now = parse_iso("2030-01-15T12:00:00Z").unwrap();
        let windows = [
            window("five_hour", 45.0, Some("2030-01-15T14:00:00Z")),
            window("seven_day", 72.0, Some("2030-01-18T00:00:00Z")),
        ];
        let m = Member { slot: 2, windows: &windows, leaves: None };
        let (left, refill) = member_room(&m, 90.0, now);
        assert_eq!(round(left, 1), 20.0, "weekly: 18 points of 90");
        // The five-hour reset gives no more room: the weekly window holds.
        assert_eq!(refill, parse_iso("2030-01-18T00:00:00Z"));
        let leaving = Member { leaves: parse_iso("2030-01-16T00:00:00Z"), ..m };
        assert_eq!(member_room(&leaving, 90.0, now).1, None, "it leaves before");
        let past = [window("five_hour", 95.0, Some("2030-01-15T11:00:00Z"))];
        let m = Member { slot: 3, windows: &past, leaves: None };
        assert_eq!(member_room(&m, 90.0, now).0, 100.0, "a passed reset is room again");
    }
}
