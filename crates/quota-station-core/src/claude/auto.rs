// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Tự chuyển khi hết quota": the rotation's decision on a computer with no
//! rotation rules file, for any kind of account (Pro, Max, Team). It looks
//! at the account in use only:
//!
//! - Its fullest window, the 5-hour or the weekly one, is the one that runs
//!   out first. While that window is under the threshold (`at`, Settings'
//!   `autoSwitchAt`, 90 % by default) Claude stays where it is.
//! - At the threshold, Claude moves to the account with the most room in
//!   that same window, of those that can take it: a fresh reading (an
//!   account that must sign in again has none), not held out of the
//!   rotation, and both windows under the threshold, so a switch never lands
//!   on an account that would be moved off again at the next check. Equal
//!   room goes to the account whose other window has more room, then to the
//!   lower number.
//! - With no such account Claude stays, and the decision says so
//!   (`wait (no account with room)`); alert 1 tells the owner.
//! - With no fresh reading of the account in use, nothing moves.
//!
//! The decision is a line the rotation's readers already know: `stay`,
//! `stay (slot 2 usage unknown)`, `wait (no account with room)`, or
//! `switch 3 (full 5h: slot 1 at 91%, 12% used)`, which
//! [`crate::switcher::parse_decision`] reads as [`crate::switcher::Why::Full`].
//! It primes nothing.

use super::engine::{Plan, Reading, Window};

/// The windows the decision looks at, as the decision line names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    FiveHour,
    Week,
}

impl Which {
    /// `5h` or `week`.
    pub fn word(self) -> &'static str {
        match self {
            Which::FiveHour => "5h",
            Which::Week => "week",
        }
    }

    fn of(self, r: &Reading) -> Window {
        match self {
            Which::FiveHour => r.five_hour,
            Which::Week => r.seven_day,
        }
    }

    fn other(self) -> Which {
        match self {
            Which::FiveHour => Which::Week,
            Which::Week => Which::FiveHour,
        }
    }
}

/// Percent used of `which`; a window with no number counts as unused.
fn used(r: &Reading, which: Which) -> f64 {
    which.of(r).used.unwrap_or(0.0)
}

/// The fullest window of `r`, with its percent used: the 5-hour one when
/// both are as full.
fn fullest(r: &Reading) -> (Which, f64) {
    let (five, week) = (used(r, Which::FiveHour), used(r, Which::Week));
    if week > five {
        (Which::Week, week)
    } else {
        (Which::FiveHour, five)
    }
}

/// Whole percent, for the decision line: `91`, never `91.0`.
fn whole(v: f64) -> i64 {
    v.floor() as i64
}

/// The decision for the accounts' readings, the account in use and the
/// threshold `at` (percent).
pub fn decide(accounts: &[Reading], active: Option<u32>, at: f64) -> Plan {
    let plan = |decision: String| Plan { decision, primes: Vec::new() };
    let Some(a) = active.and_then(|n| accounts.iter().find(|r| r.number == n)) else {
        // Claude Code is on an account the app does not keep: leave it.
        return plan("stay".into());
    };
    if !a.fresh {
        return plan(format!("stay (slot {} usage unknown)", a.number));
    }
    let (which, full) = fullest(a);
    if full < at {
        return plan("stay".into());
    }
    let mut room: Vec<&Reading> = accounts
        .iter()
        .filter(|r| {
            r.number != a.number
                && r.fresh
                && !r.disabled
                // A reading with no number at all says nothing about room.
                && (r.five_hour.used.is_some() || r.seven_day.used.is_some())
                && used(r, Which::FiveHour) < at
                && used(r, Which::Week) < at
        })
        .collect();
    room.sort_by(|x, y| {
        let key = |r: &Reading| (used(r, which), used(r, which.other()), r.number);
        let (kx, ky) = (key(x), key(y));
        kx.0.total_cmp(&ky.0).then(kx.1.total_cmp(&ky.1)).then(kx.2.cmp(&ky.2))
    });
    match room.first() {
        Some(to) => plan(format!(
            "switch {} (full {}: slot {} at {}%, {}% used)",
            to.number,
            which.word(),
            a.number,
            whole(full),
            whole(used(to, which)),
        )),
        None => plan("wait (no account with room)".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::switcher::{parse_decision, Decision, Why};

    fn win(used: f64) -> Window {
        Window { used: Some(used), resets_at: None }
    }

    /// Account `n` with a fresh reading of `five` and `week` percent used.
    fn acct(n: u32, five: f64, week: f64) -> Reading {
        Reading {
            number: n,
            fresh: true,
            five_hour: win(five),
            seven_day: win(week),
            ..Reading::default()
        }
    }

    fn line(accounts: &[Reading], active: u32) -> String {
        decide(accounts, Some(active), 90.0).decision
    }

    #[test]
    fn under_the_threshold_claude_stays() {
        let accounts = [acct(1, 89.9, 50.0), acct(2, 0.0, 0.0)];
        assert_eq!(line(&accounts, 1), "stay");
    }

    #[test]
    fn at_the_five_hour_threshold_it_takes_the_most_room_in_that_window() {
        let accounts = [acct(1, 90.0, 40.0), acct(2, 30.0, 10.0), acct(3, 12.0, 70.0)];
        // 3 has more room in the 5-hour window, though its week is fuller.
        assert_eq!(line(&accounts, 1), "switch 3 (full 5h: slot 1 at 90%, 12% used)");
    }

    #[test]
    fn at_the_weekly_threshold_it_takes_the_most_room_in_the_week() {
        let accounts = [acct(1, 20.0, 93.5), acct(2, 5.0, 60.0), acct(3, 50.0, 15.0)];
        assert_eq!(line(&accounts, 1), "switch 3 (full week: slot 1 at 93%, 15% used)");
    }

    #[test]
    fn an_account_that_must_sign_in_again_or_is_held_out_is_never_taken() {
        let mut relogin = acct(2, 0.0, 0.0);
        relogin.fresh = false;
        let mut held = acct(3, 1.0, 1.0);
        held.disabled = true;
        let accounts = [acct(1, 95.0, 10.0), relogin, held, acct(4, 40.0, 40.0)];
        assert_eq!(line(&accounts, 1), "switch 4 (full 5h: slot 1 at 95%, 40% used)");
    }

    #[test]
    fn a_reading_with_no_number_is_not_taken() {
        let mut blank = acct(2, 0.0, 0.0);
        blank.five_hour.used = None;
        blank.seven_day.used = None;
        // One window known is enough: the other counts as unused.
        let mut week_only = acct(3, 0.0, 30.0);
        week_only.five_hour.used = None;
        let accounts = [acct(1, 95.0, 10.0), blank, week_only];
        assert_eq!(line(&accounts, 1), "switch 3 (full 5h: slot 1 at 95%, 0% used)");
    }

    #[test]
    fn an_account_full_in_either_window_is_not_taken() {
        // 2 has the 5-hour room but its week is spent; 3 is spent in 5 hours.
        let accounts = [acct(1, 92.0, 30.0), acct(2, 5.0, 91.0), acct(3, 90.0, 5.0)];
        assert_eq!(line(&accounts, 1), "wait (no account with room)");
    }

    #[test]
    fn with_no_room_anywhere_claude_stays_and_says_so() {
        let accounts = [acct(1, 100.0, 100.0), acct(2, 99.0, 20.0)];
        assert_eq!(line(&accounts, 1), "wait (no account with room)");
        assert_eq!(decide(&accounts[..1], Some(1), 90.0).decision, "wait (no account with room)");
    }

    #[test]
    fn without_a_fresh_reading_of_the_account_in_use_nothing_moves() {
        let mut stale = acct(1, 99.0, 99.0);
        stale.fresh = false;
        assert_eq!(line(&[stale, acct(2, 0.0, 0.0)], 1), "stay (slot 1 usage unknown)");
    }

    #[test]
    fn an_account_the_app_does_not_keep_is_left_alone() {
        let accounts = [acct(1, 99.0, 99.0), acct(2, 0.0, 0.0)];
        assert_eq!(decide(&accounts, None, 90.0).decision, "stay");
        assert_eq!(decide(&accounts, Some(7), 90.0).decision, "stay");
    }

    #[test]
    fn the_threshold_is_the_settings() {
        let accounts = [acct(1, 80.0, 10.0), acct(2, 0.0, 0.0)];
        assert_eq!(decide(&accounts, Some(1), 90.0).decision, "stay");
        assert!(decide(&accounts, Some(1), 75.0).decision.starts_with("switch 2 "));
        // The target must be under the same threshold.
        let accounts = [acct(1, 80.0, 10.0), acct(2, 76.0, 0.0)];
        assert_eq!(decide(&accounts, Some(1), 75.0).decision, "wait (no account with room)");
    }

    #[test]
    fn ties_go_to_the_other_window_then_the_lower_number() {
        let accounts =
            [acct(1, 95.0, 0.0), acct(3, 10.0, 20.0), acct(2, 10.0, 20.0), acct(4, 10.0, 5.0)];
        assert!(line(&accounts, 1).starts_with("switch 4 "));
        let accounts = [acct(1, 95.0, 0.0), acct(3, 10.0, 20.0), acct(2, 10.0, 20.0)];
        assert!(line(&accounts, 1).starts_with("switch 2 "));
    }

    #[test]
    fn it_never_primes() {
        let accounts = [acct(1, 95.0, 0.0), acct(2, 0.0, 0.0)];
        assert!(decide(&accounts, Some(1), 90.0).primes.is_empty());
    }

    #[test]
    fn its_lines_read_back_as_the_switchers_decisions() {
        let accounts = [acct(1, 91.2, 0.0), acct(3, 12.0, 0.0)];
        let plan = decide(&accounts, Some(1), 90.0);
        assert_eq!(plan.target(), Some(3));
        assert_eq!(
            parse_decision(&plan.decision),
            Decision::Switch {
                slot: 3,
                why: Why::Full { window: "5h".into(), from: 1, at: 91.0, used: 12.0 }
            }
        );
        assert_eq!(parse_decision("wait (no account with room)"), Decision::Wait);
    }
}
