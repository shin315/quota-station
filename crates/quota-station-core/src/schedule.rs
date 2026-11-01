// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! When to read the quota again.
//!
//! Every 60 seconds while the popover is open, every 300 seconds (the
//! `quota-axi --tui` default) while it is hidden, and at once when the
//! popover opens, when the Mac wakes, or when the user asks. The freshness
//! shown is the age of each reading, never the time of the app's last call.

use std::time::{Duration, Instant};

pub const VISIBLE_EVERY: Duration = Duration::from_secs(60);
pub const HIDDEN_EVERY: Duration = Duration::from_secs(300);
/// Opening the popover reads again only when the last read is older than
/// this, so quick open-close-open does not queue reads.
pub const OPEN_REREAD_AFTER: Duration = Duration::from_secs(10);
/// Wall time running this far ahead of the monotonic clock means the Mac slept.
pub const WAKE_SLACK: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default)]
pub struct Schedule {
    visible: bool,
    last_read: Option<Instant>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn interval(&self) -> Duration {
        if self.visible {
            VISIBLE_EVERY
        } else {
            HIDDEN_EVERY
        }
    }

    /// Records that the popover opened or closed. Returns true when opening
    /// it should read at once.
    pub fn set_visible(&mut self, visible: bool, now: Instant) -> bool {
        let opened = visible && !self.visible;
        self.visible = visible;
        opened
            && self
                .last_read
                .is_none_or(|at| now.saturating_duration_since(at) >= OPEN_REREAD_AFTER)
    }

    pub fn mark_read(&mut self, at: Instant) {
        self.last_read = Some(at);
    }

    /// How long until the next timed read; zero when one is due.
    pub fn due_in(&self, now: Instant) -> Duration {
        match self.last_read {
            None => Duration::ZERO,
            Some(at) => self.interval().saturating_sub(now.saturating_duration_since(at)),
        }
    }
}

/// The monotonic clock stops while the Mac sleeps and the wall clock does
/// not, so a gap between the two over one wait means the Mac just woke.
pub fn slept(monotonic: Duration, wall: Duration) -> bool {
    wall > monotonic + WAKE_SLACK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_read_is_due_at_once() {
        let s = Schedule::new();
        assert_eq!(s.due_in(Instant::now()), Duration::ZERO);
    }

    #[test]
    fn hidden_waits_five_minutes_and_visible_one() {
        let t0 = Instant::now();
        let mut s = Schedule::new();
        s.mark_read(t0);
        assert_eq!(s.due_in(t0 + Duration::from_secs(100)), Duration::from_secs(200));
        s.set_visible(true, t0 + Duration::from_secs(5));
        assert_eq!(s.due_in(t0 + Duration::from_secs(20)), Duration::from_secs(40));
        assert_eq!(s.due_in(t0 + Duration::from_secs(61)), Duration::ZERO);
    }

    #[test]
    fn opening_reads_now_unless_just_read() {
        let t0 = Instant::now();
        let mut s = Schedule::new();
        s.mark_read(t0);
        assert!(!s.set_visible(true, t0 + Duration::from_secs(3)));
        s.set_visible(false, t0 + Duration::from_secs(4));
        assert!(s.set_visible(true, t0 + Duration::from_secs(30)));
        // Already open: no second read.
        assert!(!s.set_visible(true, t0 + Duration::from_secs(90)));
    }

    #[test]
    fn a_wall_clock_jump_means_the_mac_slept() {
        assert!(slept(Duration::from_secs(60), Duration::from_secs(3600)));
        assert!(!slept(Duration::from_secs(60), Duration::from_secs(61)));
    }
}
