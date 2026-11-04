// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The rotation's decision, ported from the owner's `cswap-prefer-team`
//! script (its Python block), so Quota Station keeps Claude on the same
//! accounts the script did, with the same rules but two: the early switch,
//! which the script has off, is on here, and the Team accounts are spent in
//! the order their weekly windows reset, which the script does not look at.
//!
//! - The Team account whose weekly window resets soonest goes first, so a
//!   week's quota is used before it resets unused. While the one in use has
//!   room, the rotation moves to another Team account with room whose weekly
//!   window resets sooner, unless that account is out of Fable while the one
//!   in use is not, or the one in use ends for good (`expire`). So when the
//!   five-hour window of the account whose week resets first fills up, the
//!   rotation moves to the other account, and comes back once that window
//!   has reset. Weekly resets less than [`WEEK_TIE_SECONDS`] apart count as
//!   the same time, and an unknown one counts as last.
//! - Else, while the one in use has room, the rotation moves to another
//!   Team account whose five-hour window has started and ends sooner, so
//!   that window is used before it resets unused: the account must have
//!   room, at least [`EARLY_MIN_SECONDS`] left of that window and at least
//!   [`EARLY_MIN_POINTS`] points of it before `full`, its weekly window must
//!   reset no later than the one in use's, and it must not be out of Fable
//!   while the account in use is not. Of several, the one whose window ends
//!   soonest is taken. When its window resets and is primed, the other
//!   account's ends sooner, so the rotation goes back the same way. Each
//!   switch costs every running session one re-read of its context.
//! - With no such account, the one in use is kept until any of its windows
//!   reaches `full` percent.
//! - Then, of the Team accounts with room, the one whose weekly window
//!   resets soonest is taken, and of those that reset at the same time, the
//!   one whose five-hour window ends soonest; a window not started yet counts
//!   as ending last. An account whose weekly Fable window is at or above
//!   `fable_full` is taken only when no other Team account with room has
//!   Fable left.
//! - The personal account is daytime overflow only (`personal_hours`), taken
//!   only when no Team account has room.
//! - With no room anywhere the account in use is kept until it is truly used
//!   up (100 %); then the Macs park on the Team account whose window ends
//!   soonest and wait for that reset.
//! - A Team account with an end time (`expire`) is used first while it has
//!   room, and from that time on it is out of every choice.
//! - Priming: a Team account not in use whose five-hour window has not
//!   started gets one tiny message, so its window starts now, at most every
//!   30 minutes per account, and only where the owner turned priming on.
//!
//! An account the owner held out of the rotation is never switched to, as
//! claude-swap's `disable` holds an account out of its own rotation; the
//! account in use is kept or left by the rules above, held out or not.
//!
//! The decision is the script's line, word for word (`switch 3 (4.0%, ends
//! 15:00)`), so [`crate::switcher::parse_decision`] and the log reader read
//! it as they read the script's. When the weekly order takes the account,
//! the reason in the brackets says so (`switch 3 (week resets
//! 2026-11-05T03:00 with 66% left, before slot 2)`): a reason the parser
//! reads as a switch to that account with a reason it does not word, as it
//! reads any reason it does not know.

use std::collections::BTreeMap;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::local_time;
use crate::rotation::Rules;

/// The age at which a reading stops counting, as the script counts it.
pub use super::usage::FRESH_SECONDS;
/// What an early switch needs left of the other account's five-hour window:
/// less is not worth the switch.
pub const EARLY_MIN_SECONDS: f64 = 20.0 * 60.0;
/// The points an early switch needs left of the other account's five-hour
/// window before `full`, so it is worth more than the switch back.
pub const EARLY_MIN_POINTS: f64 = 10.0;
/// The shortest time between two primings of one account.
pub const PRIME_GAP_SECONDS: f64 = 30.0 * 60.0;
/// Weekly resets closer than this count as the same time, so two readings
/// a few seconds apart never turn the weekly order over and back.
pub const WEEK_TIE_SECONDS: f64 = 60.0 * 60.0;

/// One window of a reading: percent used and when it resets.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Window {
    pub used: Option<f64>,
    pub resets_at: Option<OffsetDateTime>,
}

/// What the rotation knows of one account.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reading {
    pub number: u32,
    /// The reading worked and is at most [`FRESH_SECONDS`] old.
    pub fresh: bool,
    pub five_hour: Window,
    pub seven_day: Window,
    pub fable: Window,
    /// Held out of the rotation by the owner.
    pub disabled: bool,
}

/// Everything one decision depends on.
pub struct Input<'a> {
    pub rules: &'a Rules,
    pub accounts: &'a [Reading],
    pub active: Option<u32>,
    pub now: OffsetDateTime,
    /// The hour on this Mac's clock, for the personal account's hours.
    pub hour: u8,
    /// Priming is on: when each account was last primed (Unix seconds).
    pub prime: Option<&'a BTreeMap<u32, f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The script's decision line.
    pub decision: String,
    /// The accounts to prime now.
    pub primes: Vec<u32>,
}

impl Plan {
    /// The account to switch to, when the decision is a switch.
    pub fn target(&self) -> Option<u32> {
        let rest = self.decision.strip_prefix("switch ")?;
        rest.split_whitespace().next()?.parse().ok()
    }
}

/// An end time as the script reads it with `datetime.fromisoformat`: local
/// time unless it carries `Z` or an offset.
fn end_instant(at: &str) -> Option<OffsetDateTime> {
    let tail = at.get(16..).unwrap_or("");
    let zoned = tail.ends_with('Z') || tail.contains('+') || tail.contains('-');
    if zoned {
        let with_seconds = if tail.starts_with(':') {
            at.to_string()
        } else {
            format!("{}:00{}", &at[..16], tail)
        };
        OffsetDateTime::parse(&with_seconds, &Rfc3339).ok()
    } else {
        local_time::instant(at)
    }
}

/// Seconds since the epoch, as the script compares times.
fn secs(t: OffsetDateTime) -> f64 {
    t.unix_timestamp_nanos() as f64 / 1e9
}

/// Python's `str` of a float: `4.0`, `37.5`.
fn py_float(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e16 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

/// `HH:MM` on this Mac's clock.
fn hm(t: f64) -> String {
    let at =
        OffsetDateTime::from_unix_timestamp(t.floor() as i64).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    local_time::clock(at)
}

/// `2026-11-05T03:00` on this Mac's clock: a weekly reset can be days away.
fn minute(t: f64) -> String {
    let at =
        OffsetDateTime::from_unix_timestamp(t.floor() as i64).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    local_time::minute(at)
}

struct View<'a> {
    accts: BTreeMap<u32, &'a Reading>,
    rules: &'a Rules,
    now: f64,
}

impl View<'_> {
    fn used(&self, n: u32) -> Option<f64> {
        let a = self.accts.get(&n)?;
        if !a.fresh {
            return None;
        }
        let pcts: Vec<f64> = [a.five_hour.used, a.seven_day.used].into_iter().flatten().collect();
        Some(
            pcts.into_iter()
                .fold(None, |m: Option<f64>, p| Some(m.map_or(p, |m| m.max(p))))
                .unwrap_or(0.0),
        )
    }

    fn room(&self, n: u32) -> bool {
        self.used(n).is_some_and(|m| m < self.rules.full)
    }

    /// The end of the running five-hour window; none when it has not started
    /// (or already ended).
    fn ends(&self, n: u32) -> Option<f64> {
        let r = self.accts.get(&n)?.five_hour.resets_at.map(secs)?;
        (r > self.now).then_some(r)
    }

    fn key(&self, n: u32) -> f64 {
        self.ends(n).unwrap_or(f64::INFINITY)
    }

    fn fable_out(&self, n: u32) -> bool {
        let Some(a) = self.accts.get(&n) else { return false };
        a.fresh && a.fable.used.is_some_and(|p| p >= self.rules.fable_full)
    }

    fn five_used(&self, n: u32) -> Option<f64> {
        self.accts.get(&n)?.five_hour.used
    }

    /// When the running weekly window resets; none when that is not known
    /// (or already past).
    fn week(&self, n: u32) -> Option<f64> {
        let r = self.accts.get(&n)?.seven_day.resets_at.map(secs)?;
        (r > self.now).then_some(r)
    }

    /// `x`'s weekly window resets before `y`'s, by [`WEEK_TIE_SECONDS`] or
    /// more; an unknown reset comes after every known one.
    fn week_sooner(&self, x: u32, y: u32) -> bool {
        self.week(x).is_some_and(|wx| self.week(y).is_none_or(|wy| wx + WEEK_TIE_SECONDS <= wy))
    }

    /// `items` in the order the script takes them: out of Fable last, then
    /// the five-hour window that ends soonest, then the least used.
    fn by_five_hour(&self, mut items: Vec<u32>) -> Vec<u32> {
        // Python's sort is stable: equal keys keep the rotation's order.
        items.sort_by(|x, y| {
            let kx = (self.fable_out(*x), self.key(*x), self.used(*x).unwrap_or(0.0));
            let ky = (self.fable_out(*y), self.key(*y), self.used(*y).unwrap_or(0.0));
            kx.partial_cmp(&ky).unwrap_or(std::cmp::Ordering::Equal)
        });
        items
    }

    /// The account the weekly order takes of `order` (as [`View::by_five_hour`]
    /// sorts it): of those as far from Fable's limit as the first, the first
    /// whose weekly window no other resets sooner than.
    fn by_week(&self, order: &[u32]) -> Option<u32> {
        let first = *order.first()?;
        let peers: Vec<u32> =
            order.iter().copied().filter(|n| self.fable_out(*n) == self.fable_out(first)).collect();
        peers.iter().copied().find(|&n| !peers.iter().any(|&m| self.week_sooner(m, n)))
    }

    /// The decision for a switch to `n` that the weekly order made, ahead
    /// of `before`.
    fn week_switch(&self, n: u32, before: u32) -> String {
        let at = self.week(n).map_or_else(|| "?".to_string(), minute);
        let used = self.accts.get(&n).and_then(|a| a.seven_day.used);
        let left = (100.0 - used.unwrap_or(0.0)) as i64;
        format!("switch {n} (week resets {at} with {left}% left, before slot {before})")
    }
}

/// The first item of `items` with the smallest `key`, as Python's `min`.
fn first_min(items: &[u32], key: impl Fn(u32) -> f64) -> Option<u32> {
    let mut best: Option<(u32, f64)> = None;
    for &n in items {
        let k = key(n);
        if best.is_none_or(|(_, b)| k < b) {
            best = Some((n, k));
        }
    }
    best.map(|(n, _)| n)
}

/// The decision for `input`, and the accounts to prime.
pub fn decide(input: &Input) -> Plan {
    let rules = input.rules;
    let now = secs(input.now);
    let accts: BTreeMap<u32, &Reading> = input.accounts.iter().map(|a| (a.number, a)).collect();
    let mut expire: BTreeMap<u32, f64> = BTreeMap::new();
    for e in &rules.expire {
        if let Some(t) = end_instant(&e.at) {
            expire.insert(e.slot, secs(t));
        }
    }
    let gone = |n: u32| expire.get(&n).is_some_and(|t| now >= *t);
    let team: Vec<u32> =
        rules.team.iter().copied().filter(|n| accts.contains_key(n) && !gone(*n)).collect();
    let personal = rules.personal.filter(|p| !gone(*p));
    let day = rules.personal_from <= input.hour && input.hour <= rules.personal_to;
    let v = View { accts, rules, now };
    let held = |n: u32| v.accts.get(&n).is_some_and(|a| a.disabled);
    let active = input.active;
    let has = |n: Option<u32>| n.is_some_and(|n| v.accts.contains_key(&n));

    let urgent: Vec<u32> =
        team.iter().copied().filter(|n| expire.contains_key(n) && v.room(*n)).collect();
    let urgent_targets: Vec<u32> = urgent.iter().copied().filter(|n| !held(*n)).collect();
    let decision = if let Some(a) =
        active.filter(|a| has(Some(*a)) && !gone(*a) && v.used(*a).is_none())
    {
        format!("stay (slot {a} usage unknown)")
    } else if !urgent_targets.is_empty() && !active.is_some_and(|a| urgent.contains(&a)) {
        let n = first_min(&urgent_targets, |n| expire[&n]).unwrap_or(urgent_targets[0]);
        format!("switch {n} (ends for good at {}, use it first)", hm(expire[&n]))
    } else if let Some(a) = active.filter(|a| team.contains(a) && v.room(*a)) {
        // An account that ends for good is used first: never left for a
        // week that resets sooner.
        let sooner: Vec<u32> = if urgent.contains(&a) {
            Vec::new()
        } else {
            team.iter()
                .copied()
                .filter(|&o| {
                    o != a
                        && !held(o)
                        && v.room(o)
                        && v.week_sooner(o, a)
                        && !(v.fable_out(o) && !v.fable_out(a))
                })
                .collect()
        };
        let early: Vec<u32> = team
            .iter()
            .copied()
            .filter(|&o| {
                let Some(e) = v.ends(o) else { return false };
                o != a
                    && !held(o)
                    && v.room(o)
                    && e < v.key(a)
                    && e - now >= EARLY_MIN_SECONDS
                    && v.five_used(o).is_some_and(|f| rules.full - f >= EARLY_MIN_POINTS)
                    && !(v.fable_out(o) && !v.fable_out(a))
                    // Never spend early a week that resets later.
                    && !v.week_sooner(a, o)
            })
            .collect();
        if let Some(o) = v.by_week(&v.by_five_hour(sooner)) {
            v.week_switch(o, a)
        } else {
            match first_min(&early, |o| v.key(o)) {
                Some(o) => {
                    let left = (100.0 - v.five_used(o).unwrap_or(0.0)) as i64;
                    format!("switch {o} (ends {} with {left}% left, before slot {a})", hm(v.key(o)))
                }
                None => "stay".to_string(),
            }
        }
    } else {
        let cands = v.by_five_hour(
            team.iter()
                .copied()
                .filter(|n| Some(*n) != active && !held(*n) && v.room(*n))
                .collect(),
        );
        if let Some(&first) = cands.first() {
            let n = v.by_week(&cands).unwrap_or(first);
            if n != first {
                v.week_switch(n, first)
            } else {
                let why = match v.ends(n) {
                    Some(e) => format!("ends {}", hm(e)),
                    None => "window not started".to_string(),
                };
                format!("switch {n} ({}%, {why})", py_float(v.used(n).unwrap_or(0.0)))
            }
        } else if let Some(p) = personal
            .filter(|p| has(Some(*p)) && day && v.room(*p) && (active == Some(*p) || !held(*p)))
        {
            if active == Some(p) {
                "stay".to_string()
            } else {
                format!(
                    "switch {p} ({}%, no Team slot with room)",
                    py_float(v.used(p).unwrap_or(0.0))
                )
            }
        } else {
            let started: Vec<u32> = team
                .iter()
                .copied()
                .filter(|n| v.ends(*n).is_some() && (Some(*n) == active || !held(*n)))
                .collect();
            let last = active.filter(|a| has(Some(*a))).and_then(|a| v.used(a));
            let keeps = active.is_some_and(|a| team.contains(&a) || (Some(a) == personal && day));
            match (last, active) {
                // Never park onto an exhausted account while the one in use
                // still has quota left: use it up first.
                (Some(last), Some(a)) if last < 100.0 && keeps => format!(
                    "stay (no room anywhere, using the last {}% of slot {a})",
                    (100.0 - last) as i64
                ),
                _ => match first_min(&started, |n| v.key(n)) {
                    Some(n) if Some(n) == active => {
                        format!("stay (waiting for slot {n} at {})", hm(v.key(n)))
                    }
                    Some(n) => format!(
                        "switch {n} (park: no room anywhere, it resets first at {})",
                        hm(v.key(n))
                    ),
                    None => "wait (no account with room)".to_string(),
                },
            }
        }
    };

    let mut plan = Plan { decision, primes: Vec::new() };
    if let Some(last) = input.prime {
        let target = plan.target().or(active);
        for &n in &team {
            let fresh = v.accts.get(&n).is_some_and(|a| a.fresh);
            if Some(n) == target
                || Some(n) == active
                || held(n)
                || !fresh
                || v.ends(n).is_some()
                || !v.room(n)
            {
                continue;
            }
            if now - last.get(&n).copied().unwrap_or(0.0) >= PRIME_GAP_SECONDS {
                plan.primes.push(n);
            }
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_as_python_prints_them() {
        assert_eq!(py_float(4.0), "4.0");
        assert_eq!(py_float(37.5), "37.5");
        assert_eq!(py_float(0.0), "0.0");
    }

    #[test]
    fn end_times_read_as_local_unless_zoned() {
        let local = end_instant("2026-10-16T17:45").unwrap();
        assert_eq!(local_time::minute(local), "2026-10-16T17:45");
        let utc = end_instant("2026-10-16T17:45Z").unwrap();
        assert_eq!(utc.unix_timestamp(), 1_792_172_700);
        let zoned = end_instant("2026-10-16T17:45:00+07:00").unwrap();
        assert_eq!(zoned.unix_timestamp(), 1_792_172_700 - 7 * 3600);
        assert!(end_instant("tomorrow").is_none());
    }
}
