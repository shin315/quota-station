// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's hold on the main window's close and on `Thoát Quota
//! Station` ([`Extension::hold_close`]): while the rules editor has changes
//! not saved, closing the main window asks first, and `Thoát` brings the
//! main window forward on the rules' tab and asks there. The page answers
//! through the commands `rules_dirty`, `take_quit` and `close_main`.
//!
//! [`Extension::hold_close`]: crate::ext::Extension::hold_close

use crate::advanced::state::AdvancedState;
use crate::advanced::tabs;
use crate::ext::{Closing, Hold, Kept};

/// Whether `closing` waits for the owner: only while the rules editor has
/// changes not saved (`rules_dirty`). A held `Thoát` is kept until the page
/// takes it (`take_quit`), whether it was listening already or opens now.
/// Called under the app's lock; takes the advanced part's own after it.
pub fn hold(kept: &Kept, closing: Closing) -> Hold {
    let Some(own) = kept.get::<AdvancedState>() else { return Hold::Close };
    let mut fields = own.lock();
    if !fields.rules_dirty {
        return Hold::Close;
    }
    match closing {
        Closing::MainWindow => Hold::Ask { tab: None },
        Closing::App => {
            fields.quit_pending = true;
            Hold::Ask { tab: Some(tabs::RULES) }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// What the advanced part keeps once started, with the rules editor's
    /// changes saved or not.
    fn kept(dirty: bool) -> (Kept, Arc<AdvancedState>) {
        let own = Arc::new(AdvancedState::default());
        own.lock().rules_dirty = dirty;
        let mut kept = Kept::default();
        kept.keep(own.clone());
        (kept, own)
    }

    #[test]
    fn with_the_rules_saved_the_window_closes_and_the_app_quits() {
        let (kept, own) = kept(false);
        assert_eq!(hold(&kept, Closing::MainWindow), Hold::Close);
        assert_eq!(hold(&kept, Closing::App), Hold::Close);
        assert!(!own.lock().quit_pending);
    }

    #[test]
    fn with_unsaved_rules_closing_the_main_window_asks_first() {
        let (kept, own) = kept(true);
        assert_eq!(hold(&kept, Closing::MainWindow), Hold::Ask { tab: None });
        assert!(!own.lock().quit_pending, "no quit to take");
        assert!(own.lock().rules_dirty, "still unsaved");
    }

    #[test]
    fn with_unsaved_rules_quit_asks_on_the_rules_tab_and_waits_for_the_page() {
        let (kept, own) = kept(true);
        assert_eq!(hold(&kept, Closing::App), Hold::Ask { tab: Some("rules") });
        assert!(own.lock().quit_pending, "kept until the page takes it");
        assert!(own.lock().rules_dirty);
    }

    /// Before `Advanced::start` keeps its state there is nothing unsaved.
    #[test]
    fn before_the_private_part_starts_nothing_is_held() {
        let kept = Kept::default();
        assert_eq!(hold(&kept, Closing::MainWindow), Hold::Close);
        assert_eq!(hold(&kept, Closing::App), Hold::Close);
    }
}
