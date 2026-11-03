// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's part of the snapshot, which the app knows only
//! through the hooks ([`crate::ext::Hooks::decorate_snapshot`]):
//!
//! - on every account, its place in the rotation (`role`), whether the
//!   owner holds it out (`disabled`), whether it is in the rotation now
//!   (`membership`) and when it leaves it (`leavesAt`);
//! - on every section, the rotation's personal hours (`personalHours`);
//! - on every pool, the rotation's room (`room`, `members`, `rooms`,
//!   `refills`); where the read counted it (the report's `pool`), Claude's
//!   pool is that room (kind `rotation`), else Claude's pool says why not
//!   (the report's `poolReason`).
//!
//! Every section, account and pool has the keys, empty where there is
//! nothing to say, as the windows read them. They come from the rotation's
//! keys the read wrote into the report's accounts ([`super::pool`]).
//!
//! [`SnapshotHooks`] gives the core this; [`decorate`] is the decoration.

use serde::Serialize;
use serde_json::{json, Value};

use crate::ext::Hooks;
use crate::quota::Response;
use crate::snapshot::{Account, Pool, PoolKind, Section, Snapshot};
use crate::time_util::{is_after, parse_ts};

use super::pool::{self, RoomPool, Rotation};

/// Claude's pool when the read counted the rotation's room.
pub const KIND: &str = "rotation";

/// The advanced part's hooks for the snapshot: [`decorate`]. The app's
/// extension gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnapshotHooks;

impl Hooks for SnapshotHooks {
    fn decorate_snapshot(&self, snapshot: &mut Snapshot, claude: Option<&Response>) {
        decorate(snapshot, claude);
    }
}

/// An account's place in the rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Team,
    Personal,
    /// No rotation file says what the account is.
    Unknown,
}

/// Whether an account is in the rotation now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Membership {
    In,
    Out,
    Ended,
    /// No rotation file.
    Unknown,
}

/// One counted account's room before its switch threshold.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub number: u32,
    /// 100 is a whole account's worth of room, 0 an account at its threshold.
    pub percent: f64,
}

/// The rotation the Claude accounts of `section` were read with, when the
/// read wrote one.
pub fn rotation_of(section: &Section) -> Option<Rotation> {
    serde_json::from_value(section.list_ext.get(pool::ROTATION)?.clone()).ok()
}

/// Account `number`'s place in `rotation`.
pub fn role_in(rotation: Option<&Rotation>, number: u32) -> Role {
    match rotation {
        Some(r) if r.personal == Some(number) => Role::Personal,
        Some(r) if r.team.contains(&number) => Role::Team,
        _ => Role::Unknown,
    }
}

/// Adds the advanced part's keys to `snapshot`, built from the Claude report
/// `claude`.
pub fn decorate(snapshot: &mut Snapshot, claude: Option<&Response>) {
    for section in [&mut snapshot.claude, &mut snapshot.agy, &mut snapshot.codex] {
        section.ext.insert_after("stale", "personalHours", Value::Null);
        mark_pool(&mut section.pool, None, 0, &[], &[]);
        for account in &mut section.accounts {
            mark_account(account, Role::Unknown, false, Membership::Unknown, None);
        }
    }
    let Some(report) = claude else { return };
    let Some(list) = report.primary("claude").and_then(|row| row.claude_accounts.as_ref()) else {
        return;
    };
    let now = parse_ts(&report.generated_at);
    let rotation = pool::rotation_in(list);
    let section = &mut snapshot.claude;
    for account in &mut section.accounts {
        let Some(slot) = list.slots.iter().find(|s| s.slot == account.number) else { continue };
        let (role, mut membership) = match &rotation {
            None => (Role::Unknown, Membership::Unknown),
            Some(r) => match role_in(Some(r), slot.slot) {
                Role::Unknown => (Role::Unknown, Membership::Out),
                role => (role, Membership::In),
            },
        };
        let mut leaves_at = None;
        if let Some(end) =
            rotation.as_ref().and_then(|r| r.expire.iter().find(|e| e.slot == slot.slot))
        {
            if is_after(&end.at, now) {
                leaves_at = Some(end.at.clone());
            } else {
                membership = Membership::Ended;
            }
        }
        mark_account(account, role, super::held::slot(slot), membership, leaves_at);
    }
    let hours = rotation.and_then(|r| r.personal_hours);
    section.ext.insert_after("stale", "personalHours", json!(hours));
    match pool::pool_in(list) {
        Some(room) => section.pool = room_pool(&room),
        None => {
            if let Some(reason) = pool::pool_reason_in(list) {
                section.pool.reason = Some(reason);
            }
        }
    }
}

fn mark_account(
    account: &mut Account,
    role: Role,
    disabled: bool,
    membership: Membership,
    leaves_at: Option<String>,
) {
    let ext = &mut account.ext;
    ext.insert_after("number", "role", json!(role));
    ext.insert_after("active", "disabled", json!(disabled));
    ext.insert_after("source", "membership", json!(membership));
    ext.insert_after("source", "leavesAt", json!(leaves_at));
}

fn mark_pool(pool: &mut Pool, room: Option<f64>, members: u32, rooms: &[Room], refills: &[u32]) {
    let ext = &mut pool.ext;
    ext.insert_after("kind", "room", json!(room));
    ext.insert_after("scope", "members", json!(members));
    ext.insert_after("scope", "rooms", json!(rooms));
    ext.insert_after("nextRefillAt", "refills", json!(refills));
}

/// Claude's pool as the rotation's room.
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
    let rooms: Vec<Room> =
        pool.slots.iter().map(|r| Room { number: r.slot, percent: r.room_percent }).collect();
    let mut out = Pool {
        kind: PoolKind::Ext(KIND),
        percent: Some(percent),
        line: None,
        scope: Some(pool.scope.clone()),
        next_refill_at,
        exhausted: pool.effective_percent_remaining <= 0.0,
        ..Pool::none()
    };
    mark_pool(&mut out, Some(room), members, &rooms, &refills);
    out
}
