// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! When each Antigravity account is read: never more often than Cockpit
//! Tools reads it, one account at a time.
//!
//! - The account in use is read with the app's own quota-axi reads (every
//!   60 seconds while a window is open, every 5 minutes while hidden, on
//!   wake and after an action); Cockpit reads it about once a minute.
//! - Every other account every 10 minutes, Cockpit's default, and when the
//!   `Antigravity` tab opens.
//! - No account more than once a minute, whatever asks: a reading younger
//!   than that is used as it is (`export::TTL_MS`), and a try that failed
//!   is not repeated sooner either.
//! - After a 429 or a 5xx the account waits 2 minutes, then 4, 8, 16 and at
//!   most 30, until a read gets through.
//! - Accounts that need a new login or that Google refused (403) are not
//!   read at all.
//!
//! All times are Unix milliseconds, passed in.

use std::collections::BTreeMap;

use super::export::TTL_MS;

pub const OTHERS_EVERY_MS: i64 = 10 * 60_000;
/// The first wait after a 429 or a 5xx is twice this.
pub const BACKOFF_BASE_MS: i64 = 60_000;
pub const BACKOFF_MAX_MS: i64 = 30 * 60_000;

/// An account the app could read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    /// When the reading in the cache was taken.
    pub read_at: Option<i64>,
    /// Needs a new login, or Google refused it: never read.
    pub skip: bool,
}

/// How a read ended, for the pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// A reading came back, or the cache already had a young one.
    Done,
    /// Any other failure: the next read comes at the usual time.
    Failed,
    /// 429 or 5xx: the next read waits longer.
    Throttled,
}

#[derive(Debug, Default, Clone)]
struct Account {
    tried_at: Option<i64>,
    asked: bool,
    /// 429s and 5xxs in a row.
    throttled: u32,
    wait_until: Option<i64>,
}

#[derive(Debug, Default)]
pub struct Pace {
    cycle: bool,
    all: bool,
    /// An account was asked for since the last [`Pace::take`].
    asked: bool,
    accounts: BTreeMap<String, Account>,
}

impl Pace {
    pub fn new() -> Self {
        Self::default()
    }

    /// The app is reading quota-axi: the account in use is due.
    pub fn cycle(&mut self) {
        self.cycle = true;
    }

    /// Every account is due: the `Antigravity` tab opened.
    pub fn ask_all(&mut self) {
        self.all = true;
    }

    /// One account is due: it just logged in.
    pub fn ask(&mut self, id: &str) {
        self.accounts.entry(id.to_string()).or_default().asked = true;
        self.asked = true;
    }

    /// Something asked for a read since the last [`Pace::take`]. An ask
    /// that has to wait (a read within the minute, a back-off) is not
    /// pending: a later look takes it once the wait is over.
    pub fn pending(&self) -> bool {
        self.cycle || self.all || self.asked
    }

    /// The account logged out: its asks and waits go with it.
    pub fn forget(&mut self, id: &str) {
        self.accounts.remove(id);
    }

    /// The accounts to read now, the one in use (`active`) first, then in
    /// the index's order. The asks they answer are forgotten.
    pub fn take(
        &mut self,
        candidates: &[Candidate],
        active: Option<&str>,
        now: i64,
    ) -> Vec<String> {
        let mut order: Vec<&Candidate> = candidates.iter().collect();
        order.sort_by_key(|c| Some(c.id.as_str()) != active);
        let mut due = Vec::new();
        for c in order {
            if c.skip {
                // Asked before it was refused: a new login asks again.
                if let Some(a) = self.accounts.get_mut(&c.id) {
                    a.asked = false;
                }
                continue;
            }
            let is_active = Some(c.id.as_str()) == active;
            let a = self.accounts.entry(c.id.clone()).or_default();
            if a.wait_until.is_some_and(|until| now < until)
                || a.tried_at.is_some_and(|t| now - t < TTL_MS)
            {
                continue;
            }
            let last = a.tried_at.max(c.read_at);
            let timed = last.is_none_or(|t| now - t >= OTHERS_EVERY_MS);
            if timed || a.asked || self.all || (is_active && self.cycle) {
                a.asked = false;
                due.push(c.id.clone());
            }
        }
        self.cycle = false;
        self.all = false;
        self.asked = false;
        due
    }

    /// A read of the account starts now.
    pub fn started(&mut self, id: &str, now: i64) {
        self.accounts.entry(id.to_string()).or_default().tried_at = Some(now);
    }

    pub fn finished(&mut self, id: &str, finish: Finish, now: i64) {
        let a = self.accounts.entry(id.to_string()).or_default();
        match finish {
            Finish::Done => {
                a.throttled = 0;
                a.wait_until = None;
            }
            Finish::Failed => {}
            Finish::Throttled => {
                a.throttled = a.throttled.saturating_add(1);
                a.wait_until = Some(now + backoff(a.throttled));
            }
        }
    }

    /// When the account may be read again after 429s or 5xxs.
    pub fn waiting_until(&self, id: &str, now: i64) -> Option<i64> {
        self.accounts.get(id)?.wait_until.filter(|&until| until > now)
    }
}

/// The wait after `n` 429s or 5xxs in a row: 2, 4, 8, 16, then 30 minutes.
pub fn backoff(n: u32) -> i64 {
    let doubled = BACKOFF_BASE_MS.saturating_mul(1_i64 << n.min(16));
    doubled.min(BACKOFF_MAX_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_768_548_000_000;
    const MIN: i64 = 60_000;

    fn two(read_at: Option<i64>) -> Vec<Candidate> {
        ["a", "b"].iter().map(|id| Candidate { id: id.to_string(), read_at, skip: false }).collect()
    }

    #[test]
    fn the_wait_doubles_to_half_an_hour() {
        let waits: Vec<i64> = (1..=7).map(|n| backoff(n) / MIN).collect();
        assert_eq!(waits, [2, 4, 8, 16, 30, 30, 30]);
    }

    #[test]
    fn the_account_in_use_follows_the_reads_and_the_others_every_ten_minutes() {
        let mut pace = Pace::new();
        let fresh = two(Some(T0));
        // Nothing asked, both read just now.
        assert!(pace.take(&fresh, Some("b"), T0 + MIN).is_empty());
        // A quota-axi read: only the account in use.
        pace.cycle();
        assert_eq!(pace.take(&fresh, Some("b"), T0 + MIN), ["b"]);
        assert!(!pace.pending());
        // Ten minutes on, the other one too, the one in use first.
        pace.cycle();
        assert_eq!(pace.take(&two(Some(T0)), Some("b"), T0 + 10 * MIN), ["b", "a"]);
    }

    #[test]
    fn nothing_is_read_twice_within_a_minute_whatever_asks() {
        let mut pace = Pace::new();
        let none = two(None);
        assert_eq!(pace.take(&none, None, T0), ["a", "b"], "never read: both due");
        pace.started("a", T0);
        pace.finished("a", Finish::Failed, T0 + 1_000);
        pace.started("b", T0);
        pace.finished("b", Finish::Done, T0 + 1_000);
        pace.ask_all();
        pace.cycle();
        pace.ask("a");
        assert!(pace.take(&none, Some("a"), T0 + 59_000).is_empty());
        // The ask for "a" outlives the minute; the tab's ask does not.
        assert_eq!(pace.take(&none, Some("a"), T0 + MIN), ["a"]);
        // A failed try waits the usual ten minutes for the next.
        assert!(pace.take(&none, None, T0 + 9 * MIN).is_empty());
        assert_eq!(pace.take(&none, None, T0 + 11 * MIN), ["a", "b"]);
    }

    #[test]
    fn an_ask_that_has_to_wait_is_not_pending() {
        let mut pace = Pace::new();
        let list = two(Some(T0 - 20 * MIN));
        for id in ["a", "b"] {
            pace.started(id, T0);
            pace.finished(id, Finish::Done, T0);
        }
        // Asked within the minute: nothing to do now, nothing pending.
        pace.ask("a");
        assert!(pace.pending());
        assert!(pace.take(&list, None, T0 + 10_000).is_empty());
        assert!(!pace.pending());
        // A later look takes it.
        assert_eq!(pace.take(&list, None, T0 + MIN), ["a"]);

        // An ask for an account that is skipped, or logged out, is dropped.
        let mut pace = Pace::new();
        let mut list = two(Some(T0));
        list[0].skip = true;
        pace.ask("a");
        assert!(pace.take(&list, None, T0 + 2 * MIN).is_empty());
        list[0].skip = false;
        assert!(pace.take(&list, None, T0 + 3 * MIN).is_empty(), "a new login asks again");
        pace.ask("c");
        pace.forget("c");
        assert!(pace.take(&list, None, T0 + 4 * MIN).is_empty());
        assert!(!pace.pending());
    }

    #[test]
    fn opening_the_tab_reads_every_account_but_skipped_ones() {
        let mut pace = Pace::new();
        let mut list = two(Some(T0));
        list.push(Candidate { id: "c".into(), read_at: Some(T0), skip: true });
        pace.ask_all();
        assert!(pace.pending());
        assert_eq!(pace.take(&list, Some("b"), T0 + 2 * MIN), ["b", "a"]);
        pace.ask("c");
        assert!(pace.take(&list, None, T0 + 30 * MIN).iter().all(|id| id != "c"));
    }

    #[test]
    fn a_429_or_5xx_backs_off_until_a_read_gets_through() {
        let mut pace = Pace::new();
        let list = two(Some(T0 - 20 * MIN));
        let mut now = T0;
        for wait in [2, 4, 8, 16, 30, 30] {
            pace.cycle();
            assert_eq!(pace.take(&list, Some("a"), now), ["a", "b"]);
            pace.started("a", now);
            pace.finished("a", Finish::Throttled, now);
            pace.started("b", now);
            pace.finished("b", Finish::Done, now);
            assert_eq!(pace.waiting_until("a", now), Some(now + wait * MIN));
            pace.cycle();
            pace.ask("a");
            let just_before = now + wait * MIN - 1;
            assert!(!pace.take(&list, Some("a"), just_before).contains(&"a".to_string()));
            now += wait * MIN;
            // `b` was read at `now - wait`; make it due again for the next round.
            pace.started("b", now - 10 * MIN);
        }
        pace.finished("a", Finish::Done, now);
        assert_eq!(pace.waiting_until("a", now), None);
        pace.started("a", now);
        pace.finished("a", Finish::Throttled, now);
        assert_eq!(pace.waiting_until("a", now), Some(now + 2 * MIN), "the count starts again");
    }
}
