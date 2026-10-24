// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

mod common;

use common::fixture;
use quota_station_core::icon::{
    glyph_ascii, icon, render_glyph, reset_words, short_name, CodexUse, Context, Failing, Icon,
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
    snap("claude-pool-schema5.json", Some("agy-no-cockpit.json"))
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
        failing: Failing::default(),
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
    let mut agy = snap("claude-pool-schema5.json", Some("agy-cockpit.json"));
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
    // quota-axi failing: the name stays, never an old number.
    let mut failing = cx(Menubar::Claude);
    failing.failing.claude = true;
    let i = icon(Some(&full()), &failing);
    assert_eq!((i.title.as_str(), i.tone), ("Team A –", Tone::Muted));
    assert!(i.label.starts_with("Quota Station. Claude: không đọc được số."));
    assert_eq!(i.claude, None);
    // A window past its reset waits for a new reading.
    let later = Context { now: at("2026-01-18T02:00:00Z"), ..cx(Menubar::Claude) };
    assert_eq!(icon(Some(&full()), &later).title, "Team A –");
    // Codex failing or not read yet.
    let mut codex = cx(Menubar::Codex);
    codex.failing.codex = true;
    assert_eq!(icon(Some(&full()), &codex).title, "ChatGPT –");
    let unread_codex = snap("claude-pool-schema5.json", None);
    assert_eq!(icon(Some(&unread_codex), &cx(Menubar::Codex)).title, "ChatGPT –");
    let gone =
        snap("claude-pool-schema5.json", None).with_codex(Some(&fixture("codex-unavailable.json")));
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
    let air = snap("claude-pool-schema5.json", Some("agy-not-set-up.json"));
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
        snap("claude-pool-schema5.json", None).with_codex(Some(&fixture("codex-not-set-up.json")));
    assert_eq!(icon(Some(&no_login), &cx(Menubar::Codex)).title, "Team A 87%");
}

#[test]
fn codex_reads_its_own_login_when_quota_axi_lists_several() {
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
fn glyph_snapshot_two_bars() {
    let expected = "\
..................
..................
....++++++++++....
..##############..
.#++++++++.....+#.
.#+#######.....+#.
.#++++++++.....+#.
..##############..
....++++++++++....
....++++++++++....
..##############..
.#++++++++++++.+#.
.#+###########++#.
.#++++++++++++.+#.
..##############..
....++++++++++....
..................
..................
";
    assert_eq!(glyph_ascii(&sample(Tone::Ok, true), 1), expected);
}

#[test]
fn glyph_snapshot_claude_only_and_empty() {
    let air = glyph_ascii(&sample(Tone::Ok, false), 1);
    assert_eq!(air.lines().nth(9), Some(".#+######+.....+#."));
    assert_eq!(air.lines().filter(|l| l.contains('#')).count(), 4);
    // A bar without a number is an empty capsule.
    let muted = glyph_ascii(&Icon { claude: None, agy: None, ..sample(Tone::Muted, true) }, 1);
    assert_eq!(muted.lines().nth(5), Some(".#+............+#."));
}

#[test]
fn glyph_snapshot_exhausted_triangle() {
    let expected = "\
..................
........++........
........##........
.......+##+.......
.......#++#.......
......+#..#+......
.....+#+##+#+.....
.....##.##.##.....
....+#+.##.+#+....
....#+..##..+#....
...+#...##...#+...
...#+...++...+#...
..+#....##....#+..
.+#+....##....+#+.
.##............##.
+################+
.++++++++++++++++.
..................
";
    assert_eq!(glyph_ascii(&sample(Tone::Bad, true), 1), expected);
}

#[test]
fn a_paused_switcher_adds_a_pause_sign_and_says_so() {
    let paused = icon(Some(&full()), &cx(Menubar::Claude)).with_pause(true);
    assert!(paused.paused);
    assert!(paused.label.ends_with(". Tự chuyển đang tạm dừng"));
    let (_, w, h) = render_glyph(&paused, 2);
    assert_eq!((w, h), (52, 36));
    let rows = glyph_ascii(&paused, 1);
    // Team A's Fable at 87 % used: the Claude bar nearly full.
    assert_eq!(rows.lines().nth(5), Some(".#+##########+.+#..##+.##+"));
    assert_eq!(rows.lines().nth(8), Some("....++++++++++.....##+.##+"));

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
