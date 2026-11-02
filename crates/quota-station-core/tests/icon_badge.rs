// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The menu bar icon with an extension's badge, and without one: what the
//! glyph, the Windows tray icon, the title and the label are for every tone
//! and fill without a badge, pinned in `tests/fixtures/icon-glyphs.txt`,
//! and a test badge drawn as the app draws any.
// The golden files are compared on macOS only.
#![cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]

use std::path::Path;

use quota_station_core::icon::{
    glyph_ascii, render_glyph, render_tray, tray_tip, Badge, Icon, Shape, Tone,
};

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

/// Without a badge the icon is the app's own, as pinned (written before
/// the badge was a socket). Pinned on macOS, whose maths drew the golden
/// file; the drawing is the same code on Windows.
#[cfg(target_os = "macos")]
#[test]
fn without_a_badge_the_icons_are_the_pinned_ones() {
    let mut text = String::new();
    for (name, icon) in icons() {
        assert_eq!(icon.clone().with_badge(None), icon);
        text.push_str(&describe(&name, &icon));
    }
    assert_golden(&text, "tests/fixtures/icon-glyphs.txt");
}

/// A test badge: one square in the middle of the gauge.
fn square(words: &str) -> Badge {
    Badge {
        shapes: vec![Shape { x: 7.5, y: 7.5, width: 3.0, height: 3.0, radius: 0.0 }],
        halo: 0.5,
        words: words.into(),
    }
}

/// The alpha of the glyph at 4x at point `(x, y)`.
fn alpha(icon: &Icon, x: f64, y: f64) -> u8 {
    let (rgba, w, _) = render_glyph(icon, 4);
    let (px, py) = ((x * 4.0) as u32, (y * 4.0) as u32);
    rgba[((py * w + px) * 4 + 3) as usize]
}

#[test]
fn an_extensions_badge_is_drawn_inside_the_gauge_and_read_after_the_label() {
    for (name, icon) in icons() {
        let badged = icon.clone().with_badge(Some(square("Huy hiệu thử")));
        if !matches!(icon.tone, Tone::Ok | Tone::Warn) {
            // Claude used up and no number keep their own glyph and words.
            assert_eq!(badged, icon, "{name}");
            continue;
        }
        assert_eq!(badged.badge, Some(square("Huy hiệu thử")), "{name}");
        assert_eq!(badged.label, format!("{}. Huy hiệu thử", icon.label), "{name}");
        assert_eq!((badged.title.as_str(), badged.used), (icon.title.as_str(), icon.used));
        assert_eq!(alpha(&badged, 9.0, 9.0), 255, "{name}: the square is solid");
        assert_ne!(render_glyph(&badged, 4), render_glyph(&icon, 4), "{name}");
        if icon.used == Some(100.0) {
            // Its halo stays clear even in a full gauge.
            assert_eq!(alpha(&icon, 7.2, 9.0), 255, "{name}");
            assert_eq!(alpha(&badged, 7.2, 9.0), 0, "{name}: the halo");
            // The Windows tray draws the glyph for a full window, with it.
            assert_ne!(render_tray(&badged, 32, false), render_tray(&icon, 32, false));
        } else if icon.used.is_some() {
            // The Windows tray draws the number instead of the glyph.
            assert_eq!(render_tray(&badged, 32, false), render_tray(&icon, 32, false));
        } else {
            assert_ne!(render_tray(&badged, 32, false), render_tray(&icon, 32, false));
        }
    }

    // Without words the label is the app's.
    let (_, icon) = icons().swap_remove(2);
    let quiet = icon.clone().with_badge(Some(square("")));
    assert_eq!(quiet.label, icon.label);
    assert!(quiet.badge.is_some());
}
