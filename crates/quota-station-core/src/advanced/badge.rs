// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's badge on the menu bar icon while automatic switching
//! is paused, as the C04 template draws it: two upright bars inside the
//! gauge, with a clear halo so they survive a high reservoir, and the label
//! saying so. The app draws it as any extension's badge
//! ([`crate::icon::Icon::with_badge`]); the advanced part's extension
//! hands it over while the pause holds.

use crate::icon::{Badge, Shape, GLYPH_POINTS};

/// The paused icon's frame, in points: the same as the gauge's, so
/// pausing never shifts the title.
pub const PAUSED_POINTS: u32 = GLYPH_POINTS;

/// What the label reads after the app's words while paused.
pub const PAUSED_WORDS: &str = "Tự động chuyển đang tạm dừng";

/// The badge while automatic switching is paused.
pub fn paused() -> Badge {
    Badge {
        shapes: [7.05, 9.55]
            .map(|left| Shape { x: left, y: 4.8, width: 1.4, height: 3.7, radius: 0.7 })
            .to_vec(),
        halo: 0.55,
        words: PAUSED_WORDS.into(),
    }
}
