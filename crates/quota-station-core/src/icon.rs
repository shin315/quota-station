// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The C04 menu bar identity: a circular gauge, rim bead and liquid fill.
//! The title and fill follow the same account and quota window. The title
//! stays outside the 18 pt template image; macOS supplies its tint.

use serde::Serialize;
use time::OffsetDateTime;

use crate::settings::{Menubar, MenubarWindow};
use crate::snapshot::{Account, Line, LineKind, Reading, SectionStatus, Snapshot};
#[cfg(feature = "author-tools")]
use crate::snapshot::{Role, Section};
use crate::time_util::parse_ts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    Ok,
    Warn,
    /// Claude has no room left.
    Bad,
    /// The title has no number.
    Muted,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Icon {
    pub title: String,
    pub tone: Tone,
    /// C04 reservoir level, the share used in the title; `None` leaves it empty.
    pub used: Option<f64>,
    /// Legacy per-service tightest readings, retained in the serialized model.
    pub claude: Option<f64>,
    pub agy: Option<f64>,
    /// Legacy service presence: false where this Mac has no Antigravity.
    pub show_agy: bool,
    /// Automatic switching is paused: two upright bars inside the gauge.
    #[cfg(feature = "author-tools")]
    pub paused: bool,
    /// What VoiceOver reads and the tooltip shows.
    pub label: String,
}

#[cfg(feature = "author-tools")]
impl Icon {
    /// Marks automatic switching as paused, as the C04 template draws
    /// it. An empty rotation or no number keeps its own glyph: that matters
    /// more than the pause.
    pub fn with_pause(mut self, paused: bool) -> Icon {
        if paused && matches!(self.tone, Tone::Ok | Tone::Warn) {
            self.paused = true;
            self.label.push_str(". Tự động chuyển đang tạm dừng");
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
    #[cfg(feature = "author-tools")]
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

#[cfg(feature = "author-tools")]
const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// A Claude account's name, uncut, as the windows write it: its email, else
/// `Tài khoản N`, else "Claude" for the one account read without the app's
/// list.
#[cfg(not(feature = "author-tools"))]
pub fn claude_name(a: &Account) -> String {
    if let Some(email) = &a.email {
        return local_part(email).to_string();
    }
    if a.number == 0 {
        return "Claude".into();
    }
    format!("Tài khoản {}", a.number)
}

/// A Claude account's name, uncut, as the windows write it: its email, else its
/// place (Cá nhân, Team A, Tài khoản N) with `roles`, else `Tài khoản N`, else
/// "Claude" for the one account read without the app's list.
#[cfg(feature = "author-tools")]
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

/// An Antigravity account's name, uncut: its email, else Antigravity N,
/// else "Gemini".
#[cfg(not(feature = "author-tools"))]
pub fn agy_name(a: &Account) -> String {
    match (&a.email, a.number) {
        (Some(email), _) => local_part(email).to_string(),
        (None, 0) => "Gemini".into(),
        (None, n) => format!("Antigravity {n}"),
    }
}

/// An Antigravity account's name, uncut: its email, else Ultra N with
/// `roles` or Antigravity N, else "Gemini".
#[cfg(feature = "author-tools")]
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
            None => format!("{} chưa có dữ liệu", self.name),
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
    #[cfg(feature = "author-tools")]
    let name = claude_name(section, a, cx.roles);
    #[cfg(not(feature = "author-tools"))]
    let name = claude_name(a);
    Now {
        name,
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
    #[cfg(feature = "author-tools")]
    let name = agy_name(a, cx.roles);
    #[cfg(not(feature = "author-tools"))]
    let name = agy_name(a);
    Now {
        name,
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
        used: shown.tight.map(|(_, used)| used),
        claude: claude.bar,
        agy: if show_agy { gemini.bar } else { None },
        show_agy,
        #[cfg(feature = "author-tools")]
        paused: false,
        label,
    }
}

/// Both C04 states share an 18 pt frame: pausing never shifts the title.
pub const GLYPH_POINTS: u32 = 18;
#[cfg(feature = "author-tools")]
pub const PAUSED_POINTS: u32 = GLYPH_POINTS;

/// Renders black RGBA pixels with coverage in alpha, ready for AppKit tinting.
/// Geometry is traced from the approved C04 templates (`assets/menubar`).
pub fn render_glyph(icon: &Icon, scale: u32) -> (Vec<u8>, u32, u32) {
    let size = GLYPH_POINTS * scale;
    (coverage(size, f64::from(scale), |x, y| inside(icon, x, y)), size, size)
}

/// A square of `size` pixels, black with the shape's coverage in alpha;
/// `shape` takes points in units of `1 / scale` pixels.
fn coverage(size: u32, scale: f64, shape: impl Fn(f64, f64) -> bool) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for py in 0..size {
        for px in 0..size {
            // Supersampling keeps the small ring, bead and wave smooth on
            // both 1x and Retina displays, without a bitmap dependency.
            let mut hits = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = (f64::from(px) + (f64::from(sx) + 0.5) / 4.0) / scale;
                    let y = (f64::from(py) + (f64::from(sy) + 0.5) / 4.0) / scale;
                    hits += u32::from(shape(x, y));
                }
            }
            rgba[((py * size + px) * 4 + 3) as usize] = (hits * 255 / 16) as u8;
        }
    }
    rgba
}

/// The Windows tray icon, `size` pixels square: Windows shows no title
/// beside a tray icon, so the icon draws the title's number itself, in
/// white on a dark taskbar and black on a light one. Claude used up, no
/// number, and a full window (three digits do not fit) keep the menu
/// bar's glyph; the tooltip ([`tray_tip`]) says the rest.
pub fn render_tray(icon: &Icon, size: u32, light_taskbar: bool) -> Vec<u8> {
    let number = icon
        .used
        .filter(|u| u.is_finite() && icon.tone != Tone::Bad)
        .map(|u| u.clamp(0.0, 100.0) as u32)
        .filter(|n| *n < 100);
    let mut rgba = match number {
        Some(n) => {
            let digits: Vec<u8> =
                if n < 10 { vec![n as u8] } else { vec![(n / 10) as u8, (n % 10) as u8] };
            // The digits' box is 5 by 9 units with a unit between them,
            // 78% of the icon tall, centred.
            let unit = f64::from(size) * 0.78 / 9.0;
            let count = digits.len() as f64;
            let width = count * 5.0 + (count - 1.0);
            let left = (f64::from(size) / unit - width) / 2.0;
            let top = (f64::from(size) / unit - 9.0) / 2.0;
            coverage(size, unit, |x, y| {
                let x = (x - left) / 6.0;
                let slot = x.floor();
                (0.0..count).contains(&slot)
                    && digit(digits[slot as usize], (x - slot) * 6.0, y - top) <= STROKE
            })
        }
        None => {
            coverage(size, f64::from(size) / f64::from(GLYPH_POINTS), |x, y| inside(icon, x, y))
        }
    };
    let ink = if light_taskbar { 0 } else { 255 };
    for pixel in rgba.chunks_mut(4) {
        pixel[..3].fill(ink);
    }
    rgba
}

/// Half the digits' stroke, in units of their 5 by 9 box.
const STROKE: f64 = 0.675;

/// How far `(x, y)` lies from the centre line of `d`, in its 5 by 9 box.
fn digit(d: u8, x: f64, y: f64) -> f64 {
    let (l, r, t, b) = (0.7, 4.3, 0.7, 8.3);
    let line = |points: &[(f64, f64)]| {
        points.windows(2).map(|w| seg_dist(x, y, w[0], w[1])).fold(f64::INFINITY, f64::min)
    };
    let ring = |cx: f64, cy: f64, radius: f64| ((x - cx).hypot(y - cy) - radius).abs();
    match d {
        0 => round_rect(x, y, l, t, r - l, b - t, 1.8).abs(),
        1 => line(&[(1.3, 2.1), (3.0, t), (3.0, b)]),
        2 => arc(x, y, (2.5, 2.5), 1.8, 180.0, 380.0).min(line(&[(4.19, 3.12), (l, b), (r, b)])),
        3 => arc(x, y, (2.5, 2.35), 1.65, 200.0, 450.0).min(arc(
            x,
            y,
            (2.5, 6.25),
            2.05,
            270.0,
            520.0,
        )),
        4 => line(&[(3.5, b), (3.5, t), (l, 5.9), (r, 5.9)]),
        5 => line(&[(r, t), (1.1, t), (0.9, 4.2)]).min(arc(x, y, (2.45, 6.05), 2.25, 235.0, 520.0)),
        6 => ring(2.5, 6.25, 2.05).min(line(&[
            (0.45, 6.1),
            (0.75, 3.7),
            (1.5, 1.85),
            (2.6, 0.9),
            (3.9, t),
        ])),
        7 => line(&[(l, t), (r, t), (1.8, b)]),
        8 => ring(2.5, 2.35, 1.65).min(ring(2.5, 6.45, 1.85)),
        // A 6 turned upside down.
        _ => digit(6, 5.0 - x, 9.0 - y),
    }
}

/// Distance to the arc of `radius` round `centre` from `from` to `to`
/// degrees, clockwise on screen (y grows downwards).
fn arc(x: f64, y: f64, centre: (f64, f64), radius: f64, from: f64, to: f64) -> f64 {
    let (dx, dy) = (x - centre.0, y - centre.1);
    let angle = dy.atan2(dx).to_degrees();
    let within = (0..3).any(|k| {
        let a = angle + 360.0 * f64::from(k);
        (from..=to).contains(&a)
    });
    if within {
        return (dx.hypot(dy) - radius).abs();
    }
    let end = |deg: f64| {
        let (sin, cos) = deg.to_radians().sin_cos();
        (x - centre.0 - radius * cos).hypot(y - centre.1 - radius * sin)
    };
    end(from).min(end(to))
}

/// The Windows tray's tooltip: the title on the first line (the icon has
/// only its number), then what the label reads, cut to the 127 characters
/// a tray tooltip holds.
pub fn tray_tip(icon: &Icon) -> String {
    const MAX: usize = 127;
    let rest = icon.label.strip_prefix("Quota Station. ").unwrap_or(&icon.label);
    let full = format!("{}\n{rest}", icon.title);
    if full.encode_utf16().count() <= MAX {
        return full;
    }
    let mut out = String::new();
    let mut units = 0;
    for c in full.chars() {
        if units + c.len_utf16() > MAX - 1 {
            break;
        }
        units += c.len_utf16();
        out.push(c);
    }
    out.push('…');
    out
}

fn inside(icon: &Icon, x: f64, y: f64) -> bool {
    if icon.tone == Tone::Bad {
        // Preserve the existing exhausted warning.
        let (a, b, c) = ((9.0, 2.2), (16.6, 15.6), (1.4, 15.6));
        let edge = seg_dist(x, y, a, b).min(seg_dist(x, y, b, c)).min(seg_dist(x, y, c, a));
        return edge <= 0.75
            || seg_dist(x, y, (9.0, 6.6), (9.0, 10.6)) <= 0.8
            || (x - 9.0).hypot(y - 13.0) <= 1.0;
    }
    let (dx, dy) = (x - 9.0, y - 9.0);
    let radius = dx.hypot(dy);
    // Almost-closed ring: rounded ends at -90 and 254 degrees.
    let angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);
    let ring = ((radius - 6.85).abs() <= 0.72 && !(254.0..270.0).contains(&angle))
        || (x - 9.0).hypot(y - 2.15) <= 0.72
        || (x - 7.112).hypot(y - 2.415) <= 0.72;
    let bead_distance = (x - 14.05).hypot(y - 3.8);
    if bead_distance <= 1.42 || (ring && bead_distance >= 1.87) {
        return true;
    }
    // Pause bars have a transparent halo so they survive a high reservoir.
    #[cfg(feature = "author-tools")]
    if icon.paused {
        let pause = [7.05, 9.55].map(|left| round_rect(x, y, left, 4.8, 1.4, 3.7, 0.7));
        if pause.iter().any(|d| *d <= 0.0) {
            return true;
        }
        if pause.iter().any(|d| *d <= 0.55) {
            return false;
        }
    }
    let Some(used) = icon.used.filter(|p| p.is_finite() && *p > 0.0) else {
        return false;
    };
    // Linear liquid height. At 45% the wave matches the approved master;
    // the exact percentage stays in the title, as for the former bar glyph.
    let p = used.clamp(0.0, 100.0) / 100.0;
    let wave = 9.0
        + 5.6 * (1.0 - 2.0 * p)
        + 0.55 * (dx * std::f64::consts::PI / 5.6).sin() * (4.0 * p * (1.0 - p));
    radius <= 5.6 && y >= wave
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
