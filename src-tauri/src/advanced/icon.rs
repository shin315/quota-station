// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's badge on the menu bar icon: while the pause holds
//! (`SwitcherState::paused`), the paused badge
//! (`quota_station_core::advanced::badge::paused`); otherwise none.

use quota_station_core::advanced::badge;
use quota_station_core::icon::Badge;
use time::OffsetDateTime;

use super::state::AdvancedState;
use crate::ext::Kept;

/// The icon's badge at `now`, from the pause the advanced part last saw.
pub fn badge(kept: &Kept, now: OffsetDateTime) -> Option<Badge> {
    let author = kept.get::<AdvancedState>()?;
    let paused = author.lock().switcher.paused(now);
    paused.then(badge::paused)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use quota_station_core::local_time;
    use quota_station_core::pause;

    use super::*;

    #[test]
    fn the_icon_has_the_paused_badge_while_the_pause_holds() {
        let now = OffsetDateTime::now_utc();
        let mut kept = Kept::default();
        assert_eq!(badge(&kept, now), None, "nothing kept yet");
        let author = Arc::new(AdvancedState::default());
        kept.keep(author.clone());
        assert_eq!(badge(&kept, now), None, "no pause");

        let later = local_time::minute(now + time::Duration::hours(1));
        author.lock().switcher.pause = Some(pause::Flag { until: Some(later), reason: None });
        assert_eq!(badge(&kept, now), Some(badge::paused()));

        let expired = local_time::minute(now - time::Duration::minutes(5));
        author.lock().switcher.pause = Some(pause::Flag { until: Some(expired), reason: None });
        assert_eq!(badge(&kept, now), None, "a pause over");
    }
}
