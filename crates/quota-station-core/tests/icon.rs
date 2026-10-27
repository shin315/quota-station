// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

mod common;

use common::fixture;
use quota_station_core::icon::{
    icon, render_glyph, render_tray, reset_words, short_name, tray_tip, CodexUse, Context, Icon,
    Tone,
};
use quota_station_core::settings::{Menubar, MenubarWindow};
use quota_station_core::snapshot::{LineKind, Role, SectionStatus};
use quota_station_core::Snapshot;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).unwrap()
}

fn snap(claude: &str, agy: Option<&str>) -> Snapshot {
    let agy = agy.map(fixture);
    Snapshot::build(Some(&fixture(claude)), agy.as_ref())
}

/// Claude in rotation, Antigravity's live account, Codex on its ChatGPT login.
fn full() -> Snapshot {
    snap("claude-pool.json", Some("agy-single.json"))
        .with_codex(Some(&fixture("codex-chatgpt.json")))
}

/// The tightest window, as the title showed before there was a choice.
fn cx(follow: Menubar) -> Context {
    shows(follow, MenubarWindow::Tightest)
}

fn shows(follow: Menubar, window: MenubarWindow) -> Context {
    Context {
        follow,
        window,
        codex: CodexUse::ChatGpt,
        now: at("2026-01-16T07:20:00Z"),
        roles: true,
    }
}

fn named(mut s: Snapshot) -> Snapshot {
    for a in &mut s.claude.accounts {
        a.email = Some(format!("lan.acme{}@example.com", a.number));
    }
    for a in &mut s.agy.accounts {
        a.email = Some("lanpham.ultra@example.com".into());
    }
    s
}

#[test]
fn the_title_is_the_claude_account_in_use_and_its_tightest_window() {
    let i = icon(Some(&named(full())), &cx(Menubar::Claude));
    // Team A: 5 giờ 74 %, tuần 39 %, Fable 87 %: Fable limits first.
    assert_eq!(i.title, "lan.acme2 87%");
    assert_eq!(i.tone, Tone::Ok);
    assert_eq!(
        i.label,
        "Quota Station. Claude: lan.acme2 đã dùng 87% Fable tuần, đặt lại sau 1 ngày 18 giờ. \
         Antigravity: lanpham.ultra đã dùng 6% Gemini tuần, đặt lại sau 4 ngày, Claude/GPT tuần 0%. \
         Codex: ChatGPT đã dùng 41% tuần, đặt lại sau 4 ngày 5 giờ 40 phút"
    );
    assert!(!i.label.contains("tương đương"));
    // The bars fill as the windows are used.
    assert_eq!((i.claude, i.agy), (Some(87.0), Some(6.0)));
}

#[test]
fn the_title_follows_gemini_or_codex_when_picked() {
    let s = named(full());
    assert_eq!(icon(Some(&s), &cx(Menubar::Gemini)).title, "lanpham.ult… 6%");
    assert_eq!(icon(Some(&s), &cx(Menubar::Codex)).title, "ChatGPT 41%");
    // The bars stay Claude's and Antigravity's whatever the title follows.
    assert_eq!(icon(Some(&s), &cx(Menubar::Codex)).claude, Some(87.0));
}

#[test]
fn the_title_shows_the_window_picked_and_the_bars_stay_the_tightest() {
    let s = full();
    // Team A: 5 giờ 74 %, tuần 39 %, Fable 87 %.
    let five = icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::default()));
    assert_eq!(five.title, "Team A 74%");
    assert!(
        five.label.starts_with("Quota Station. Claude: Team A đã dùng 74% 5 giờ, "),
        "{}",
        five.label
    );
    assert_eq!(five.claude, Some(87.0), "the bar fills with the tightest window");
    let week = icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::Week));
    assert_eq!(week.title, "Team A 39%");
    assert!(week.label.contains("Team A đã dùng 39% tuần"), "{}", week.label);
    let fable = icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::FableWeek));
    assert_eq!(fable.title, "Team A 87%");
    assert!(fable.label.contains("Team A đã dùng 87% Fable tuần"), "{}", fable.label);
    assert_eq!(
        icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::Tightest)).title,
        "Team A 87%"
    );
    // Every service in the label names the window picked.
    assert!(week.label.contains("Antigravity: Gemini đã dùng 6% Gemini tuần"), "{}", week.label);
    assert!(five.label.contains("Codex: ChatGPT đã dùng 23% 5 giờ"), "{}", five.label);
    assert!(week.label.contains("Codex: ChatGPT đã dùng 41% tuần"), "{}", week.label);
}

#[test]
fn a_window_the_service_lacks_falls_back_to_five_hours_then_the_tightest() {
    let s = full();
    // Codex and Gemini have no Fable window: their five hours.
    let codex = icon(Some(&s), &shows(Menubar::Codex, MenubarWindow::FableWeek));
    assert_eq!(codex.title, "ChatGPT 23%");
    assert!(codex.label.contains("Codex: ChatGPT đã dùng 23% 5 giờ"), "{}", codex.label);
    let gemini = icon(Some(&s), &shows(Menubar::Gemini, MenubarWindow::FableWeek));
    let five = icon(Some(&s), &shows(Menubar::Gemini, MenubarWindow::FiveHours));
    assert_eq!(gemini.title, five.title);
    assert!(gemini.label.contains("Gemini 5 giờ"), "{}", gemini.label);
    // A Team seat has no week: its five hours.
    let mut seat = full();
    let team = seat.claude.accounts.iter_mut().find(|a| a.active).unwrap();
    team.windows.retain(|l| l.kind != LineKind::Week);
    assert_eq!(icon(Some(&seat), &shows(Menubar::Claude, MenubarWindow::Week)).title, "Team A 74%");
    // Neither the window picked nor five hours: the tightest.
    let mut fable_only = full();
    let team = fable_only.claude.accounts.iter_mut().find(|a| a.active).unwrap();
    team.windows.retain(|l| l.kind == LineKind::FableWeek);
    assert_eq!(
        icon(Some(&fable_only), &shows(Menubar::Claude, MenubarWindow::Week)).title,
        "Team A 87%"
    );
}

#[test]
fn the_window_picked_without_a_number_shows_no_number() {
    let mut s = full();
    let team = s.claude.accounts.iter_mut().find(|a| a.active).unwrap();
    team.windows.iter_mut().filter(|l| l.kind == LineKind::Session).for_each(|l| l.pending = true);
    // Five hours waits for its number: no number, never another window's.
    let i = icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::FiveHours));
    assert_eq!((i.title.as_str(), i.tone), ("Team A –", Tone::Muted));
    // The week's own number is known and is the one picked.
    assert_eq!(icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::Week)).title, "Team A 39%");
    // The tightest needs every window.
    assert_eq!(icon(Some(&s), &shows(Menubar::Claude, MenubarWindow::Tightest)).title, "Team A –");
}

#[test]
fn a_long_name_is_cut_to_twelve_characters() {
    assert_eq!(short_name("lanpham.ultra@example.com"), "lanpham.ult…");
    assert_eq!(short_name("lan.acme1@example.com"), "lan.acme1");
    assert_eq!(short_name("abcdefghijkl@example.com"), "abcdefghijkl");
    assert_eq!(short_name("cliproxy"), "cliproxy");
}

#[test]
fn hidden_names_give_the_account_its_place() {
    // No email reaches the snapshot while names are hidden.
    let s = full();
    assert_eq!(icon(Some(&s), &cx(Menubar::Claude)).title, "Team A 87%");
    assert_eq!(icon(Some(&s), &cx(Menubar::Gemini)).title, "Gemini 6%");
    let mut personal = full();
    for a in &mut personal.claude.accounts {
        a.active = a.role == Role::Personal;
    }
    assert!(icon(Some(&personal), &cx(Menubar::Claude)).title.starts_with("Cá nhân "));
    let mut agy = snap("claude-pool.json", Some("agy-list.json"));
    agy.agy.accounts.iter_mut().for_each(|a| a.email = None);
    assert!(icon(Some(&agy), &cx(Menubar::Gemini)).title.starts_with("Ultra "));
}

#[test]
fn no_number_shows_the_name_and_a_dash() {
    let unread = icon(None, &cx(Menubar::Claude));
    assert_eq!((unread.title.as_str(), unread.tone), ("Claude –", Tone::Muted));
    let unavailable = icon(Some(&snap("claude-unavailable.json", None)), &cx(Menubar::Claude));
    assert_eq!(unavailable.tone, Tone::Muted);
    assert!(unavailable.title.ends_with(" –"), "{}", unavailable.title);
    // A window past its reset waits for a new reading.
    let later = Context { now: at("2026-01-18T02:00:00Z"), ..cx(Menubar::Claude) };
    assert_eq!(icon(Some(&full()), &later).title, "Team A –");
    // Codex not read yet.
    let unread_codex = snap("claude-pool.json", None);
    assert_eq!(icon(Some(&unread_codex), &cx(Menubar::Codex)).title, "ChatGPT –");
    let gone = snap("claude-pool.json", None).with_codex(Some(&fixture("codex-unavailable.json")));
    assert_eq!(gone.codex.status, SectionStatus::Unavailable);
    assert_eq!(icon(Some(&gone), &cx(Menubar::Codex)).title, "ChatGPT –");
}

#[test]
fn codex_through_its_own_provider_shows_the_provider_without_a_number() {
    let mut c = cx(Menubar::Codex);
    c.codex = CodexUse::Provider("cliproxy".into());
    let i = icon(Some(&full()), &c);
    assert_eq!((i.title.as_str(), i.tone), ("cliproxy", Tone::Ok));
    assert!(i.label.ends_with(". Codex: đi qua cliproxy"));
}

#[test]
fn a_service_this_mac_lacks_falls_back_to_claude() {
    let air = snap("claude-pool.json", Some("agy-not-set-up.json"));
    let i = icon(Some(&air), &cx(Menubar::Gemini));
    assert_eq!(i.title, "Team A 87%");
    assert!(!i.show_agy);
    assert_eq!(i.agy, None);
    assert!(!i.label.contains("Antigravity"));
    let mut no_codex = cx(Menubar::Codex);
    no_codex.codex = CodexUse::Missing;
    let i = icon(Some(&full()), &no_codex);
    assert_eq!(i.title, "Team A 87%");
    assert!(!i.label.contains("Codex"));
    let no_login =
        snap("claude-pool.json", None).with_codex(Some(&fixture("codex-not-set-up.json")));
    assert_eq!(icon(Some(&no_login), &cx(Menubar::Codex)).title, "Team A 87%");
}

#[test]
fn codex_reads_its_own_login_when_the_report_lists_several() {
    let s = full();
    assert_eq!(s.codex.status, SectionStatus::Ok);
    let a = s.codex.active().unwrap();
    assert_eq!(a.windows.len(), 2, "five hours and a week, not the code review windows");
    assert_eq!(a.windows[0].percent, Some(77.0));
}

#[test]
fn a_tie_names_the_longer_window_and_a_team_seat_has_no_week() {
    let mut s = full();
    let team = s.claude.accounts.iter_mut().find(|a| a.active).unwrap();
    // A Team seat: five hours and Fable only, both 40 % used.
    team.windows.retain(|l| l.kind != quota_station_core::snapshot::LineKind::Week);
    for l in &mut team.windows {
        l.percent = Some(60.0);
    }
    let i = icon(Some(&s), &cx(Menubar::Claude));
    assert_eq!(i.title, "Team A 40%");
    assert!(i.label.contains("đã dùng 40% Fable tuần"), "{}", i.label);
}

#[test]
fn stale_numbers_warn_and_an_empty_rotation_is_bad() {
    assert_eq!(icon(Some(&snap("claude-stale.json", None)), &cx(Menubar::Claude)).tone, Tone::Warn);
    let ex = icon(Some(&snap("claude-exhausted.json", None)), &cx(Menubar::Claude));
    assert_eq!(ex.tone, Tone::Bad);
    assert!(ex.label.ends_with(". Claude đã cạn"));
}

#[test]
fn reset_words_read_like_the_popover() {
    assert_eq!(reset_words(9), "đặt lại sau 9 phút");
    assert_eq!(reset_words(189), "đặt lại sau 3 giờ 09 phút");
    assert_eq!(reset_words(2 * 1440 + 60), "đặt lại sau 2 ngày 1 giờ");
    assert_eq!(reset_words(0), "đặt lại sau 1 phút");
}

fn sample(tone: Tone, show_agy: bool) -> Icon {
    Icon {
        title: String::new(),
        tone,
        used: Some(45.0),
        claude: Some(56.75),
        agy: Some(94.0),
        show_agy,
        paused: false,
        label: String::new(),
    }
}

#[test]
fn glyph_is_an_18pt_template_image() {
    let (rgba, w, h) = render_glyph(&sample(Tone::Ok, true), 2);
    assert_eq!((w, h), (36, 36));
    // Template image: black only, shape in alpha.
    assert!(rgba.chunks(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0));
    assert!(rgba.chunks(4).any(|p| p[3] == 255));
}

#[test]
fn reservoir_tracks_the_selected_service_and_window() {
    let s = full();
    for (service, window, expected) in [
        (Menubar::Claude, MenubarWindow::FiveHours, 74.0),
        (Menubar::Claude, MenubarWindow::Week, 39.0),
        (Menubar::Gemini, MenubarWindow::Week, 6.0),
        (Menubar::Codex, MenubarWindow::Week, 41.0),
    ] {
        let i = icon(Some(&s), &shows(service, window));
        assert_eq!(i.used, Some(expected));
    }
    assert_eq!(icon(None, &cx(Menubar::Claude)).used, None);
    let provider = Context { codex: CodexUse::Provider("local".into()), ..cx(Menubar::Codex) };
    assert_eq!(icon(Some(&s), &provider).used, None);
}

#[test]
fn reservoir_fills_monotonically_without_changing_the_rim() {
    let mut last = 0;
    for used in [None, Some(0.0), Some(1.0), Some(45.0), Some(90.0), Some(100.0)] {
        let (rgba, w, _) = render_glyph(&Icon { used, ..sample(Tone::Ok, true) }, 4);
        let alpha: u32 = rgba.chunks(4).map(|p| u32::from(p[3])).sum();
        assert!(alpha >= last);
        last = alpha;
        // Clear channel between the reservoir and lower rim.
        assert_eq!(rgba[((59 * w + 9 * 4) * 4 + 3) as usize], 0);
    }
    let empty = render_glyph(&Icon { used: None, ..sample(Tone::Muted, true) }, 2);
    let zero = render_glyph(&Icon { used: Some(0.0), ..sample(Tone::Ok, true) }, 2);
    assert_eq!(empty, zero);
}

/// Columns of the tray icon with any ink, as `#` and `.`.
fn inked_columns(rgba: &[u8], size: u32) -> String {
    (0..size)
        .map(|x| {
            if (0..size).any(|y| rgba[((y * size + x) * 4 + 3) as usize] > 128) {
                '#'
            } else {
                '.'
            }
        })
        .collect()
}

#[test]
fn the_windows_tray_draws_the_number_in_the_taskbars_ink() {
    for size in [16, 20, 24, 32] {
        let two = render_tray(&Icon { used: Some(38.9), ..sample(Tone::Ok, true) }, size, false);
        assert_eq!(two.len(), (size * size * 4) as usize);
        // Two digits, side by side with a gap between them.
        let columns = inked_columns(&two, size);
        let runs = columns.split('.').filter(|r| !r.is_empty()).count();
        assert!(runs >= 2, "{size}: {columns}");
        // White on a dark taskbar, black on a light one.
        assert!(two.chunks(4).all(|p| p[..3] == [255, 255, 255]));
        let light = render_tray(&Icon { used: Some(38.0), ..sample(Tone::Ok, true) }, size, true);
        assert!(light.chunks(4).all(|p| p[..3] == [0, 0, 0]));
        assert_eq!(
            two.chunks(4).map(|p| p[3]).collect::<Vec<_>>(),
            light.chunks(4).map(|p| p[3]).collect::<Vec<_>>(),
            "the number is the title's, cut the same way"
        );
    }
    // One digit stands in the middle.
    let one = render_tray(&Icon { used: Some(7.0), ..sample(Tone::Ok, true) }, 16, false);
    let columns = inked_columns(&one, 16);
    assert_eq!(columns.split('.').filter(|r| !r.is_empty()).count(), 1, "{columns}");
    let first = columns.find('#').unwrap();
    let last = columns.rfind('#').unwrap();
    assert!(first >= 4 && last <= 11, "{columns}");
    // Each number draws differently.
    let drawn: Vec<Vec<u8>> = (0..100)
        .map(|n| {
            render_tray(&Icon { used: Some(f64::from(n)), ..sample(Tone::Ok, true) }, 16, false)
        })
        .collect();
    for (a, x) in drawn.iter().enumerate() {
        for (b, y) in drawn.iter().enumerate().skip(a + 1) {
            assert_ne!(x, y, "{a} and {b}");
        }
    }
}

#[test]
fn the_windows_tray_keeps_the_glyph_without_a_two_digit_number() {
    let glyph = |icon: &Icon| {
        let (rgba, _, _) = render_glyph(icon, 2);
        rgba.chunks(4).map(|p| p[3]).collect::<Vec<_>>()
    };
    for icon in [
        Icon { used: None, ..sample(Tone::Muted, true) },
        Icon { used: Some(100.0), ..sample(Tone::Ok, true) },
        sample(Tone::Bad, true),
    ] {
        // The glyph at 36 pixels, as the menu bar draws it at 2x.
        let tray = render_tray(&icon, 36, true);
        assert_eq!(tray.chunks(4).map(|p| p[3]).collect::<Vec<_>>(), glyph(&icon));
    }
}

#[test]
fn the_windows_tray_tooltip_leads_with_the_title_and_fits_the_tray() {
    let i = icon(Some(&full()), &cx(Menubar::Claude));
    let tip = tray_tip(&i);
    let (first, rest) = tip.split_once('\n').unwrap();
    assert_eq!(first, i.title);
    assert!(rest.starts_with("Claude: "), "{rest}");
    assert!(tip.encode_utf16().count() <= 127);

    let long = Icon {
        title: "lan.acme1 38%".into(),
        label: format!("Quota Station. Claude: {}", "đã dùng 38% 5 giờ, ".repeat(20)),
        ..sample(Tone::Ok, true)
    };
    let tip = tray_tip(&long);
    assert_eq!(tip.encode_utf16().count(), 127);
    assert!(tip.starts_with("lan.acme1 38%\nClaude: đã dùng"));
    assert!(tip.ends_with('…'));
}

#[test]
fn exhausted_rotation_keeps_the_warning_instead_of_a_quota_gauge() {
    assert_ne!(render_glyph(&sample(Tone::Bad, true), 2), render_glyph(&sample(Tone::Ok, true), 2));
}

#[test]
fn a_paused_switcher_adds_a_pause_sign_and_says_so() {
    let paused = icon(Some(&full()), &cx(Menubar::Claude)).with_pause(true);
    assert!(paused.paused);
    assert!(paused.label.ends_with(". Tự chuyển đang tạm dừng"));
    let (_, w, h) = render_glyph(&paused, 2);
    assert_eq!((w, h), (36, 36));
    for used in [0.0, 45.0, 100.0] {
        let normal = Icon { used: Some(used), ..sample(Tone::Ok, true) };
        let paused = Icon { paused: true, ..normal.clone() };
        let (rgba, w, _) = render_glyph(&paused, 4);
        let alpha = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize];
        assert_eq!(alpha(30, 26), 255, "left pause bar remains visible");
        assert_eq!(alpha(40, 26), 255, "right pause bar remains visible");
        assert_eq!(alpha(35, 26), 0, "pause bars stay separate even at full quota");
        assert_ne!(render_glyph(&normal, 4).0, rgba);
    }

    // An empty rotation or no number keeps its own glyph.
    let empty =
        icon(Some(&snap("claude-exhausted.json", None)), &cx(Menubar::Claude)).with_pause(true);
    assert!(!empty.paused);
    assert!(!empty.label.contains("tạm dừng"));
    assert!(!icon(None, &cx(Menubar::Claude)).with_pause(true).paused);
    assert!(!icon(Some(&full()), &cx(Menubar::Claude)).with_pause(false).paused);
}

#[test]
fn without_nang_cao_an_account_without_a_name_goes_by_number() {
    let s = full();
    let plain = |follow| Context { roles: false, ..cx(follow) };
    assert_eq!(icon(Some(&s), &cx(Menubar::Claude)).title, "Team A 87%");
    assert_eq!(icon(Some(&s), &plain(Menubar::Claude)).title, "Tài khoản 2 87%");
    assert!(!icon(Some(&s), &plain(Menubar::Gemini)).title.contains("Ultra"));
    // A name shown wins either way.
    assert_eq!(icon(Some(&named(full())), &plain(Menubar::Claude)).title, "lan.acme2 87%");
}
