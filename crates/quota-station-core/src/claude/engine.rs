//! The rotation's decision, ported from the owner's `cswap-prefer-team`
//! script (its Python block), so Quota Station keeps Claude on the same
//! accounts the script did, with the same rules:
//!
//! - The Team accounts are equal. The one in use is kept until any of its
//!   windows reaches `full` percent.
//! - Then the Team account with room whose five-hour window ends soonest is
//!   taken; a window not started yet counts as ending last. An account whose
//!   weekly Fable window is at or above `fable_full` is taken only when no
//!   other Team account with room has Fable left.
//! - The personal account is daytime overflow only (`personal_hours`), taken
//!   only when no Team account has room.
//! - With no room anywhere the account in use is kept until it is truly used
//!   up (100 %); then the Macs park on the Team account whose window ends
//!   soonest and wait for that reset.
//! - A Team account with an end time (`expire`) is used first while it has
//!   room, and from that time on it is out of every choice.
//! - Early switching is off (the script's `EARLY_MIN=0`); the code path is
//!   kept so the decision reads like the script's.
//! - Priming: a Team account not in use whose five-hour window has not
//!   started gets one tiny message, so its window starts now, at most every
//!   30 minutes per account, and only where the owner turned priming on.
//!
//! An account the owner disabled for rotation is never chosen, as
//! claude-swap's `disable` holds an account out of its own rotation.
//!
//! The decision is the script's line, word for word (`switch 3 (4.0%, ends
//! 15:00)`), so [`crate::switcher::parse_decision`] and the log reader read
//! it as they read the script's.

use std::collections::BTreeMap;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::local_time;
use crate::rotation::Rules;

/// The age at which a reading stops counting, as the script counts it.
pub const FRESH_SECONDS: i64 = 900;
/// Early switching is off: a window ending within this many seconds of
/// now never pulls the rotation onto another account.
pub const EARLY_SECONDS: f64 = 0.0;
/// What an early switch would need left of the other account's window.
pub const EARLY_LEFT: f64 = 30.0;
/// The shortest time between two primings of one account.
pub const PRIME_GAP_SECONDS: f64 = 30.0 * 60.0;

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
    let held = |n: u32| accts.get(&n).is_some_and(|a| a.disabled);
    let team: Vec<u32> = rules
        .team
        .iter()
        .copied()
        .filter(|n| accts.contains_key(n) && !gone(*n) && !held(*n))
        .collect();
    let personal = rules.personal.filter(|p| !gone(*p) && !held(*p));
    let day = rules.personal_from <= input.hour && input.hour <= rules.personal_to;
    let v = View { accts, rules, now };
    let active = input.active;
    let has = |n: Option<u32>| n.is_some_and(|n| v.accts.contains_key(&n));

    let urgent: Vec<u32> =
        team.iter().copied().filter(|n| expire.contains_key(n) && v.room(*n)).collect();
    let decision = if let Some(a) =
        active.filter(|a| has(Some(*a)) && !gone(*a) && v.used(*a).is_none())
    {
        format!("stay (slot {a} usage unknown)")
    } else if !urgent.is_empty() && !active.is_some_and(|a| urgent.contains(&a)) {
        let n = first_min(&urgent, |n| expire[&n]).unwrap_or(urgent[0]);
        format!("switch {n} (ends for good at {}, use it first)", hm(expire[&n]))
    } else if let Some(a) = active.filter(|a| team.contains(a) && v.room(*a)) {
        let mut early = None;
        for &o in &team {
            let Some(e) = v.ends(o) else { continue };
            let five = v.five_used(o);
            if o == a
                || !v.room(o)
                || e - now > EARLY_SECONDS
                || e >= v.key(a)
                || five.is_none_or(|f| 100.0 - f < EARLY_LEFT)
                || (v.fable_out(o) && !v.fable_out(a))
            {
                continue;
            }
            let left = (100.0 - five.unwrap_or(0.0)) as i64;
            early = Some(format!("switch {o} (ends {} with {left}% left, before slot {a})", hm(e)));
            break;
        }
        early.unwrap_or_else(|| "stay".to_string())
    } else {
        let mut cands: Vec<u32> =
            team.iter().copied().filter(|n| Some(*n) != active && v.room(*n)).collect();
        // Python's sort is stable: equal keys keep the rotation's order.
        cands.sort_by(|x, y| {
            let kx = (v.fable_out(*x), v.key(*x), v.used(*x).unwrap_or(0.0));
            let ky = (v.fable_out(*y), v.key(*y), v.used(*y).unwrap_or(0.0));
            kx.partial_cmp(&ky).unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(&n) = cands.first() {
            let why = match v.ends(n) {
                Some(e) => format!("ends {}", hm(e)),
                None => "window not started".to_string(),
            };
            format!("switch {n} ({}%, {why})", py_float(v.used(n).unwrap_or(0.0)))
        } else if let Some(p) = personal.filter(|p| has(Some(*p)) && day && v.room(*p)) {
            if active == Some(p) {
                "stay".to_string()
            } else {
                format!(
                    "switch {p} ({}%, no Team slot with room)",
                    py_float(v.used(p).unwrap_or(0.0))
                )
            }
        } else {
            let started: Vec<u32> = team.iter().copied().filter(|n| v.ends(*n).is_some()).collect();
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
            if Some(n) == target || Some(n) == active || !fresh || v.ends(n).is_some() || !v.room(n)
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
