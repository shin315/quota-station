// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The menu bar icon with the advanced part's badge, as the app draws it
//! while the pause holds: what the glyph, the Windows tray icon, the title
//! and the label are for every tone and fill, pinned in
//! `src/advanced/fixtures/icon-badge.txt` (`icon_badge.rs` has the icon
//! without a badge and with a test one).
// The golden file is compared on macOS only.
#![cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]

use std::path::Path;

use quota_station_core::icon::{glyph_ascii, render_glyph, render_tray, tray_tip, Icon, Tone};

/// FNV-1a: a short, stable stand-in for a whole image in the golden files.
fn fnv(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The icon as the golden files have it: its words, its glyph at three
/// scales and its tray icon at four sizes on a dark and a light taskbar,
/// and for an empty or half fill the glyph drawn as text.
fn describe(name: &str, icon: &Icon) -> String {
    let mut out = format!(
        "## {name}\ntitle: {}\ntone: {:?}\nused: {:?}\nlabel: {}\ntip: {:?}\n",
        icon.title,
        icon.tone,
        icon.used,
        icon.label,
        tray_tip(icon)
    );
    for scale in [1, 2, 3] {
        let (rgba, w, h) = render_glyph(icon, scale);
        out.push_str(&format!("glyph@{scale}: {w}x{h} {:016x}\n", fnv(&rgba)));
    }
    for size in [16, 20, 24, 32] {
        for light in [false, true] {
            let rgba = render_tray(icon, size, light);
            let look = if light { " light" } else { "" };
            out.push_str(&format!("tray@{size}{look}: {:016x}\n", fnv(&rgba)));
        }
    }
    if icon.used == Some(45.0) || icon.used.is_none() {
        out.push_str(&glyph_ascii(icon, 2));
    }
    out
}

/// Every tone with no number, an empty, a half and a full fill.
fn icons() -> Vec<(String, Icon)> {
    let mut all = Vec::new();
    for tone in [Tone::Ok, Tone::Warn, Tone::Muted, Tone::Bad] {
        for used in [None, Some(0.0), Some(45.0), Some(100.0)] {
            let icon = Icon {
                title: "lan 45%".into(),
                tone,
                used,
                claude: Some(56.75),
                agy: Some(94.0),
                show_agy: true,
                badge: None,
                label: "Quota Station. Claude: lan đã dùng 45% 5 giờ".into(),
            };
            all.push((format!("{tone:?} {used:?}"), icon));
        }
    }
    all
}

/// Compares `text` with the golden file `path` (from the crate's folder),
/// or rewrites it with `UPDATE_ICON_GOLDEN` set.
fn assert_golden(text: &str, path: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    if std::env::var_os("UPDATE_ICON_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&path).expect("the golden file");
    assert!(text == golden, "the icons differ from {}", path.display());
}

/// With the pause's badge the icon is as pinned (written before the badge
/// was a socket, from the app's own field then).
#[cfg(target_os = "macos")]
#[test]
fn with_the_pause_badge_the_icons_are_the_pinned_ones() {
    use quota_station_core::advanced::badge::paused;
    let mut text = String::new();
    for (name, icon) in icons() {
        text.push_str(&describe(&format!("{name}, badge"), &icon.with_badge(Some(paused()))));
    }
    assert_golden(&text, "src/advanced/fixtures/icon-badge.txt");
}
