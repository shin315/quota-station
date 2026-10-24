// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The menu bar icon (F02): a monochrome template glyph with one bar per
//! service, and the title beside it: the account in use and its share used
//! for the service and the window the owner picked (Claude, Gemini or Codex;
//! 5 giờ, tuần, Fable tuần or the tightest), the way claude-swap's own menu
//! bar named the account in use.
//!
//! The glyph follows the approved sketch `qs-tray-*.svg` on an 18 pt frame;
//! each bar fills as its account's tightest window is used. The title is
//! the tray title, never drawn into the image.

use serde::Serialize;
use time::OffsetDateTime;

use crate::settings::{Menubar, MenubarWindow};
use crate::snapshot::{Account, Line, LineKind, Reading, Role, Section, SectionStatus, Snapshot};
use crate::time_util::parse_ts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Ok,
    Warn,
    /// Claude has no room left in the rotation.
    Bad,
    /// The title has no number.
    Muted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub title: String,
    pub tone: Tone,
    /// Bar fill, the share used 0-100; `None` draws an empty bar.
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

/// How Codex reaches its models, from its `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexUse {
    /// No Codex folder on this Mac.
    Missing,
    /// The ChatGPT login: the numbers are that account's.
    ChatGpt,
    /// A provider of its own (`cliproxy`), which reports no quota.
    Provider(String),
}

/// What the icon needs beside the snapshot.
#[derive(Debug, Clone)]
pub struct Context {
    /// The service the title follows, from settings.
    pub follow: Menubar,
    /// The window the title shows, from settings.
    pub window: MenubarWindow,
    pub codex: CodexUse,
    pub now: OffsetDateTime,
    /// Accounts without a name shown go by their place in the rotation and
    /// as Ultra N (Cài đặt › Nâng cao on); otherwise by number.
    pub roles: bool,
}

/// The longest name the title keeps, in characters.
pub const NAME_MAX: usize = 12;
const NO_NUMBER: &str = "–";

/// The part of an email before `@`, cut to [`NAME_MAX`] characters with `…`.
pub fn short_name(name: &str) -> String {
    let local = local_part(name);
    if local.chars().count() > NAME_MAX {
        local.chars().take(NAME_MAX - 1).chain(['…']).collect()
    } else {
        local.to_string()
    }
}

/// The part of an email before `@`: the menu bar has no room for the domain.
fn local_part(name: &str) -> &str {
    name.split('@').next().unwrap_or(name)
}

const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A Claude account's name, uncut, as the windows write it: its email, else its
/// place (Cá nhân, Team A, Tài khoản N) with `roles`, else `Tài khoản N`, else
/// "Claude" for the one account read without the app's list.
pub fn claude_name(section: &Section, a: &Account, roles: bool) -> String {
    if let Some(email) = &a.email {
        return local_part(email).to_string();
    }
    if a.number == 0 {
        return "Claude".into();
    }
    if !roles {
        return format!("Tài khoản {}", a.number);
    }
    match a.role {
        Role::Personal => "Cá nhân".into(),
        Role::Team => {
            let mut team: Vec<u32> = section
                .accounts
                .iter()
                .filter(|x| x.role == Role::Team)
                .map(|x| x.number)
                .collect();
            team.sort_unstable();
            let place = team.iter().position(|n| *n == a.number).unwrap_or(0);
            match LETTERS.get(place) {
                Some(l) => format!("Team {}", *l as char),
                None => format!("Team {}", a.number),
            }
        }
        Role::Unknown => format!("Tài khoản {}", a.number),
    }
}

/// An Antigravity account's name, uncut: its email, else Ultra N with
/// `roles` or Antigravity N, else "Gemini".
pub fn agy_name(a: &Account, roles: bool) -> String {
    match (&a.email, a.number) {
        (Some(email), _) => local_part(email).to_string(),
        (None, 0) => "Gemini".into(),
        (None, n) if roles => format!("Ultra {n}"),
        (None, n) => format!("Antigravity {n}"),
    }
}

/// One window's share used, when it has a number now.
fn used(line: &Line, now: OffsetDateTime) -> Option<f64> {
    let passed = line.resets_at.as_deref().and_then(parse_ts).is_some_and(|at| at <= now);
    if line.pending || passed {
        return None;
    }
    line.percent.map(|left| (100.0 - left).round().clamp(0.0, 100.0))
}

fn longer(kind: LineKind) -> bool {
    matches!(
        kind,
        LineKind::Week | LineKind::FableWeek | LineKind::GeminiWeek | LineKind::ClaudeGptWeek
    )
}

/// The window used the most and its share; a tie names the longer window,
/// which recovers later. `None` when any window has no number now: the
/// title never shows a number that may be the wrong one.
fn tightest<'a>(lines: &[&'a Line], now: OffsetDateTime) -> Option<(&'a Line, f64)> {
    let mut best: Option<(&Line, f64)> = None;
    for line in lines {
        let u = used(line, now)?;
        let wins = match best {
            None => true,
            Some((b, bu)) => u > bu || (u == bu && longer(line.kind) && !longer(b.kind)),
        };
        if wins {
            best = Some((line, u));
        }
    }
    best
}

/// An account's windows, or its effective lines when no list has them.
fn lines_of(a: &Account) -> &[Line] {
    if a.windows.is_empty() {
        &a.lines
    } else {
        &a.windows
    }
}

fn pick<'a>(a: &'a Account, kinds: &[LineKind]) -> Vec<&'a Line> {
    lines_of(a).iter().filter(|l| kinds.contains(&l.kind)).collect()
}

const FIVE_HOURS: &[LineKind] =
    &[LineKind::Session, LineKind::GeminiSession, LineKind::ClaudeGptSession];
const WEEK: &[LineKind] = &[LineKind::Week, LineKind::GeminiWeek, LineKind::ClaudeGptWeek];

/// The window the owner picked and its share used: the five-hour window when
/// the account has no window of that kind, and the tightest without either.
/// `None` when that window has no number now.
fn chosen<'a>(
    lines: &[&'a Line],
    window: MenubarWindow,
    now: OffsetDateTime,
) -> Option<(&'a Line, f64)> {
    for w in [window, MenubarWindow::FiveHours] {
        let kinds = match w {
            MenubarWindow::FiveHours => FIVE_HOURS,
            MenubarWindow::Week => WEEK,
            MenubarWindow::FableWeek => &[LineKind::FableWeek],
            MenubarWindow::Tightest => break,
        };
        let found: Vec<&Line> = lines.iter().copied().filter(|l| kinds.contains(&l.kind)).collect();
        if !found.is_empty() {
            return tightest(&found, now);
        }
    }
    tightest(lines, now)
}

const GEMINI: &[LineKind] = &[
    LineKind::GeminiSession,
    LineKind::GeminiWeek,
    LineKind::Gemini,
    LineKind::Session,
    LineKind::Week,
];
const CLAUDE_GPT: &[LineKind] =
    &[LineKind::ClaudeGptSession, LineKind::ClaudeGptWeek, LineKind::ClaudeGpt];
const CLAUDE: &[LineKind] = &[LineKind::Session, LineKind::Week, LineKind::FableWeek];

/// A window's name after a number, as the popover writes it.
fn window_words(kind: LineKind, gemini: bool) -> &'static str {
    match (kind, gemini) {
        (LineKind::Session, false) => "5 giờ",
        (LineKind::Week, false) => "tuần",
        (LineKind::Session | LineKind::GeminiSession, true) => "Gemini 5 giờ",
        (LineKind::Week | LineKind::GeminiWeek, true) => "Gemini tuần",
        (LineKind::FableWeek, _) => "Fable tuần",
        (LineKind::Gemini, _) => "Gemini",
        (LineKind::ClaudeGpt, _) => "Claude/GPT",
        (LineKind::ClaudeGptSession, _) => "Claude/GPT 5 giờ",
        (LineKind::ClaudeGptWeek, _) => "Claude/GPT tuần",
        (LineKind::GeminiSession, false) => "5 giờ",
        (LineKind::GeminiWeek, false) => "tuần",
    }
}

/// `đặt lại sau 3 giờ 09 phút`, for screen readers.
pub fn reset_words(minutes: i64) -> String {
    let m = minutes.max(1);
    let (d, h, mi) = (m / 1440, (m % 1440) / 60, m % 60);
    let mut parts = Vec::new();
    if d > 0 {
        parts.push(format!("{d} ngày"));
    }
    if h > 0 {
        parts.push(format!("{h} giờ"));
    }
    if mi > 0 {
        parts.push(if h > 0 { format!("{mi:02} phút") } else { format!("{mi} phút") });
    }
    format!("đặt lại sau {}", parts.join(" "))
}

/// What one service shows: whose account, and the window picked.
struct Now<'a> {
    /// Uncut: the title cuts it, the label reads it whole.
    name: String,
    /// The window the title shows.
    tight: Option<(&'a Line, f64)>,
    /// The tightest window's share used: the bar fills with it.
    bar: Option<f64>,
    /// The number is old: the last reading is stale.
    stale: bool,
    /// Codex goes through a provider that reports no quota: no number is
    /// the normal state, not a missing reading.
    no_quota: bool,
}

impl Now<'_> {
    fn title(&self) -> String {
        match self.tight {
            Some((_, u)) => format!("{} {}%", short_name(&self.name), u as i64),
            None if self.no_quota => short_name(&self.name),
            None => format!("{} {NO_NUMBER}", short_name(&self.name)),
        }
    }

    fn words(&self, gemini: bool, now: OffsetDateTime) -> String {
        match self.tight {
            Some((line, u)) => {
                let mut w = format!(
                    "{} đã dùng {}% {}",
                    self.name,
                    u as i64,
                    window_words(line.kind, gemini)
                );
                if let Some(at) = line.resets_at.as_deref().and_then(parse_ts) {
                    w.push_str(", ");
                    w.push_str(&reset_words((at - now).whole_minutes()));
                }
                w
            }
            None => format!("{} chưa có số", self.name),
        }
    }
}

fn readable(a: &Account) -> bool {
    !matches!(a.reading, Reading::ResetPending | Reading::NoData)
}

fn claude_now<'a>(s: &'a Snapshot, cx: &Context) -> Now<'a> {
    let section = &s.claude;
    let Some(a) = section.active() else {
        return Now {
            name: "Claude".into(),
            tight: None,
            bar: None,
            stale: false,
            no_quota: false,
        };
    };
    let lines = if readable(a) { pick(a, CLAUDE) } else { Vec::new() };
    Now {
        name: claude_name(section, a, cx.roles),
        tight: chosen(&lines, cx.window, cx.now),
        bar: tightest(&lines, cx.now).map(|(_, u)| u),
        stale: section.stale,
        no_quota: false,
    }
}

fn agy_now<'a>(s: &'a Snapshot, cx: &Context, kinds: &[LineKind]) -> Now<'a> {
    let section = &s.agy;
    let Some(a) = section.active() else {
        return Now {
            name: "Gemini".into(),
            tight: None,
            bar: None,
            stale: false,
            no_quota: false,
        };
    };
    let lines = if readable(a) { pick(a, kinds) } else { Vec::new() };
    Now {
        name: agy_name(a, cx.roles),
        tight: chosen(&lines, cx.window, cx.now),
        bar: tightest(&lines, cx.now).map(|(_, u)| u),
        stale: section.stale,
        no_quota: false,
    }
}

fn codex_now<'a>(s: &'a Snapshot, cx: &Context) -> Now<'a> {
    if let CodexUse::Provider(name) = &cx.codex {
        return Now { name: name.clone(), tight: None, bar: None, stale: false, no_quota: true };
    }
    let section = &s.codex;
    let tight = section
        .active()
        .filter(|a| readable(a))
        .and_then(|a| chosen(&pick(a, &[LineKind::Session, LineKind::Week]), cx.window, cx.now));
    let name = if section.status == SectionStatus::Unread && cx.codex == CodexUse::Missing {
        "Codex"
    } else {
        "ChatGPT"
    };
    Now { name: name.into(), tight, bar: None, stale: section.stale, no_quota: false }
}

/// The service the title follows: the one picked, or Claude when this Mac
/// has no Antigravity, or no Codex reachable through a login or a provider.
pub fn followed(s: &Snapshot, follow: Menubar, codex: &CodexUse) -> Menubar {
    match follow {
        Menubar::Gemini if s.agy.status == SectionStatus::NotSetUp => Menubar::Claude,
        Menubar::Codex
            if *codex == CodexUse::Missing
                || (*codex == CodexUse::ChatGpt && s.codex.status == SectionStatus::NotSetUp) =>
        {
            Menubar::Claude
        }
        other => other,
    }
}

/// The icon for the last snapshot, `None` before the first read.
pub fn icon(snapshot: Option<&Snapshot>, cx: &Context) -> Icon {
    let unread;
    let s = match snapshot {
        Some(s) => s,
        None => {
            unread = Snapshot::build(None, None);
            &unread
        }
    };
    let claude = claude_now(s, cx);
    let gemini = agy_now(s, cx, GEMINI);
    let claude_gpt = agy_now(s, cx, CLAUDE_GPT);
    let codex = codex_now(s, cx);
    let show_agy = s.agy.status != SectionStatus::NotSetUp;
    let follow = followed(s, cx.follow, &cx.codex);
    let shown = match follow {
        Menubar::Claude => &claude,
        Menubar::Gemini => &gemini,
        Menubar::Codex => &codex,
    };
    let exhausted = snapshot.is_some() && s.claude.pool.exhausted;
    let tone = if exhausted {
        Tone::Bad
    } else if shown.tight.is_none() && !shown.no_quota {
        Tone::Muted
    } else if shown.stale {
        Tone::Warn
    } else {
        Tone::Ok
    };

    let mut label = String::from("Quota Station. Claude: ");
    label.push_str(&claude.words(false, cx.now));
    if show_agy && s.agy.status != SectionStatus::Unread {
        label.push_str(". Antigravity: ");
        label.push_str(&gemini.words(true, cx.now));
        if let Some((line, u)) = claude_gpt.tight {
            label.push_str(&format!(", {} {}%", window_words(line.kind, false), u as i64));
        }
    }
    match &cx.codex {
        CodexUse::Missing => {}
        CodexUse::Provider(name) => label.push_str(&format!(". Codex: đi qua {name}")),
        CodexUse::ChatGpt if s.codex.status == SectionStatus::NotSetUp => {}
        CodexUse::ChatGpt => {
            label.push_str(". Codex: ");
            label.push_str(&codex.words(false, cx.now));
        }
    }
    match tone {
        Tone::Bad => label.push_str(". Claude đã cạn"),
        Tone::Warn => label.push_str(". Có cảnh báo"),
        _ => {}
    }
    Icon {
        title: shown.title(),
        tone,
        claude: claude.bar,
        agy: if show_agy { gemini.bar } else { None },
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
