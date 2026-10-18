//! The menu bar icon (F02): a monochrome template glyph with one bar per
//! service, and the title beside it with Claude's whole-rotation number.
//!
//! The glyph follows the approved sketch `qs-tray-*.svg` on an 18 pt frame;
//! the bars are drawn from the live percentages instead of the sketch's
//! sample lengths. The number is the tray title, never drawn into the image.

use serde::Serialize;

use crate::snapshot::{Pool, PoolKind, SectionStatus, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Ok,
    Warn,
    /// Claude has no room left in the rotation.
    Bad,
    /// No number can be read.
    Muted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub title: String,
    pub tone: Tone,
    /// Bar fill, 0-100; `None` draws an empty bar.
    pub claude: Option<f64>,
    pub agy: Option<f64>,
    /// Draw the Antigravity bar: false where this Mac has no Antigravity.
    pub show_agy: bool,
    /// Automatic switching is paused: two upright bars after the glyph.
    pub paused: bool,
    /// What VoiceOver reads and the tooltip shows.
    pub label: String,
}

impl Icon {
    /// Marks automatic switching as paused, as `qs-tray-paused.svg` draws
    /// it. An empty rotation or no number keeps its own glyph: that matters
    /// more than the pause.
    pub fn with_pause(mut self, paused: bool) -> Icon {
        if paused && matches!(self.tone, Tone::Ok | Tone::Warn) {
            self.paused = true;
            self.label.push_str(". Tự chuyển đang tạm dừng");
        }
        self
    }
}

/// Room in accounts, one decimal, Vietnamese comma: `2,3`.
pub fn format_room(room: f64) -> String {
    format!("{:.1}", (room * 10.0).round() / 10.0).replace('.', ",")
}

pub fn format_percent(percent: f64) -> String {
    format!("{}%", percent.round() as i64)
}

/// The pool's headline value: room in accounts, or the active percent.
pub fn pool_value(pool: &Pool) -> Option<String> {
    match pool.kind {
        PoolKind::Rotation => pool.room.map(format_room),
        PoolKind::Active => pool.percent.map(format_percent),
        PoolKind::None => None,
    }
}

const NO_NUMBER: &str = "–";

/// The icon for the last snapshot. `failing` is true while quota-axi cannot
/// be read: the icon then says so instead of showing an old number.
pub fn icon(snapshot: Option<&Snapshot>, failing: bool) -> Icon {
    let Some(s) = snapshot.filter(|_| !failing) else {
        return Icon {
            title: NO_NUMBER.to_string(),
            tone: Tone::Muted,
            claude: None,
            agy: None,
            show_agy: snapshot.is_none_or(|s| s.agy.status != SectionStatus::NotSetUp),
            paused: false,
            label: "Quota Station. Claude: chưa đọc được".to_string(),
        };
    };
    let claude = &s.claude.pool;
    let value = pool_value(claude);
    let tone = if value.is_none() {
        Tone::Muted
    } else if claude.exhausted {
        Tone::Bad
    } else if s.claude.stale {
        Tone::Warn
    } else {
        Tone::Ok
    };
    let show_agy = s.agy.status != SectionStatus::NotSetUp;
    let mut label = String::from("Quota Station. Claude: ");
    label.push_str(&match (claude.kind, &value) {
        (PoolKind::Rotation, Some(v)) => format!("tương đương {v} tài khoản trong vòng"),
        (PoolKind::Active, Some(v)) => format!("tài khoản đang dùng còn {v}"),
        _ => "chưa đọc được".to_string(),
    });
    if show_agy {
        if let Some(v) = pool_value(&s.agy.pool) {
            label.push_str(&match s.agy.pool.kind {
                PoolKind::Rotation => format!(". Antigravity: tương đương {v} tài khoản"),
                _ => format!(". Antigravity: còn {v}"),
            });
        }
    }
    match tone {
        Tone::Bad => label.push_str(". Claude đã cạn"),
        Tone::Warn => label.push_str(". Có cảnh báo"),
        _ => {}
    }
    Icon {
        title: value.unwrap_or_else(|| NO_NUMBER.to_string()),
        tone,
        claude: claude.percent,
        agy: if show_agy { s.agy.pool.percent } else { None },
        show_agy,
        paused: false,
        label,
    }
}

/// Frame of the glyph in points: the sketch's 18 pt template frame.
pub const GLYPH_POINTS: u32 = 18;
/// Width of the paused glyph, `qs-tray-paused.svg`: the bars and a pause sign.
pub const PAUSED_POINTS: u32 = 26;

/// Renders the glyph as a template image: black, with coverage in alpha.
/// Returns RGBA rows, `18 * scale` pixels high and as wide, or 26 points
/// wide while paused, with the width and height.
pub fn render_glyph(icon: &Icon, scale: u32) -> (Vec<u8>, u32, u32) {
    let height = GLYPH_POINTS * scale;
    let width = if icon.paused { PAUSED_POINTS } else { GLYPH_POINTS } * scale;
    let s = f64::from(scale);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for py in 0..height {
        for px in 0..width {
            // Sample at the pixel centre, in points.
            let x = (f64::from(px) + 0.5) / s;
            let y = (f64::from(py) + 0.5) / s;
            let a = coverage(icon, x, y, s);
            let i = ((py * width + px) * 4) as usize;
            rgba[i + 3] = (a * 255.0).round() as u8;
        }
    }
    (rgba, width, height)
}

/// Bar geometry from `qs-tray-base.svg`: an outlined capsule and a fill
/// inset by 1.1 pt. A full fill is 12.5 pt long.
const BAR_X: f64 = 1.65;
const BAR_W: f64 = 14.7;
const BAR_H: f64 = 4.2;
const BAR_STROKE: f64 = 1.3;
const FILL_INSET: f64 = 1.1;
const FILL_H: f64 = 2.0;
const FILL_MAX: f64 = BAR_W - 2.0 * FILL_INSET;

fn coverage(icon: &Icon, x: f64, y: f64, scale: f64) -> f64 {
    let aa = |d: f64| (0.5 - d * scale).clamp(0.0, 1.0);
    if icon.tone == Tone::Bad {
        // qs-tray-empty.svg: a warning triangle.
        let (a, b, c) = ((9.0, 2.2), (16.6, 15.6), (1.4, 15.6));
        let edge = seg_dist(x, y, a, b).min(seg_dist(x, y, b, c)).min(seg_dist(x, y, c, a));
        let mark = seg_dist(x, y, (9.0, 6.6), (9.0, 10.6)) - 0.8;
        let dot = ((x - 9.0).powi(2) + (y - 13.0).powi(2)).sqrt() - 1.0;
        return aa(edge - 0.75).max(aa(mark)).max(aa(dot));
    }
    let tops: &[(f64, Option<f64>)] = if icon.show_agy {
        &[(3.4, icon.claude), (10.4, icon.agy)]
    } else {
        // qs-tray-air.svg: Claude only, centred.
        &[(6.9, icon.claude)]
    };
    let mut cover: f64 = 0.0;
    if icon.paused {
        // qs-tray-paused.svg: two 2.2 × 9 pt bars right of the glyph.
        for x0 in [19.2, 23.1] {
            cover = cover.max(aa(round_rect(x, y, x0, 4.5, 2.2, 9.0, 1.0)));
        }
    }
    for &(top, percent) in tops {
        let ring = round_rect(x, y, BAR_X, top, BAR_W, BAR_H, BAR_H / 2.0).abs() - BAR_STROKE / 2.0;
        cover = cover.max(aa(ring));
        let percent = if icon.tone == Tone::Muted { None } else { percent };
        if let Some(p) = percent.filter(|p| *p > 0.0) {
            let w = (FILL_MAX * p.min(100.0) / 100.0).max(FILL_H);
            let fill =
                round_rect(x, y, BAR_X + FILL_INSET, top + FILL_INSET, w, FILL_H, FILL_H / 2.0);
            cover = cover.max(aa(fill));
        }
    }
    cover
}

/// Signed distance to a rounded rectangle, negative inside.
fn round_rect(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64, r: f64) -> f64 {
    let qx = (px - (x + w / 2.0)).abs() - (w / 2.0 - r);
    let qy = (py - (y + h / 2.0)).abs() - (h / 2.0 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}

fn seg_dist(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((px - a.0) * dx + (py - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt()
}

/// The glyph as text, one character per pixel (`#` solid, `+` edge), for
/// snapshot tests and debugging.
pub fn glyph_ascii(icon: &Icon, scale: u32) -> String {
    let (rgba, w, h) = render_glyph(icon, scale);
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            let a = rgba[((y * w + x) * 4 + 3) as usize];
            out.push(match a {
                0..=63 => '.',
                64..=191 => '+',
                _ => '#',
            });
        }
        out.push('\n');
    }
    out
}
