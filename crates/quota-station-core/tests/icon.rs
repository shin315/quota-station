mod common;

use common::fixture;
use quota_station_core::icon::{format_room, glyph_ascii, icon, render_glyph, Icon, Tone};
use quota_station_core::Snapshot;

fn snap(claude: &str, agy: Option<&str>) -> Snapshot {
    let agy = agy.map(fixture);
    Snapshot::build(Some(&fixture(claude)), agy.as_ref())
}

#[test]
fn title_is_the_rotation_room_in_accounts() {
    let i = icon(Some(&snap("claude-pool-schema5.json", Some("agy-no-cockpit.json"))), false);
    assert_eq!(i.title, "2,3");
    assert_eq!(i.tone, Tone::Ok);
    assert_eq!(
        i.label,
        "Quota Station. Claude: tương đương 2,3 tài khoản trong vòng. Antigravity: còn 94%"
    );
    assert!(i.show_agy);
    assert_eq!(i.agy, Some(94.0));
}

#[test]
fn without_a_rotation_the_title_is_the_active_percent() {
    let i = icon(Some(&snap("claude-keychain-required.json", None)), false);
    assert_eq!(i.title, "27%");
    assert_eq!(i.label, "Quota Station. Claude: tài khoản đang dùng còn 27%");
}

#[test]
fn stale_numbers_warn_and_an_empty_rotation_is_bad() {
    assert_eq!(icon(Some(&snap("claude-stale.json", None)), false).tone, Tone::Warn);
    let ex = icon(Some(&snap("claude-exhausted.json", None)), false);
    assert_eq!(ex.tone, Tone::Bad);
    assert_eq!(ex.title, "0,0");
    assert!(ex.label.ends_with(". Claude đã cạn"));
}

#[test]
fn no_number_shows_a_dash() {
    let unread = icon(None, false);
    assert_eq!((unread.title.as_str(), unread.tone), ("–", Tone::Muted));
    let unavailable = icon(Some(&snap("claude-unavailable.json", None)), false);
    assert_eq!((unavailable.title.as_str(), unavailable.tone), ("–", Tone::Muted));
    // quota-axi failing: never an old number in the menu bar.
    let failing = icon(Some(&snap("claude-pool-schema5.json", None)), true);
    assert_eq!((failing.title.as_str(), failing.tone), ("–", Tone::Muted));
}

#[test]
fn agy_rotation_and_a_mac_without_antigravity() {
    let pool = icon(Some(&snap("claude-pool-schema5.json", Some("agy-pool.json"))), false);
    assert!(pool.label.ends_with(". Antigravity: tương đương 1,2 tài khoản"));
    let air = icon(Some(&snap("claude-pool-schema5.json", Some("agy-not-set-up.json"))), false);
    assert!(!air.show_agy);
    assert_eq!(air.agy, None);
}

#[test]
fn room_rounds_to_one_decimal_with_a_comma() {
    assert_eq!(format_room(2.25), "2,3");
    assert_eq!(format_room(0.04), "0,0");
    assert_eq!(format_room(1.0), "1,0");
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
    let muted = glyph_ascii(&sample(Tone::Muted, true), 1);
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
    let paused = icon(Some(&snap("claude-pool-schema5.json", Some("agy-no-cockpit.json"))), false)
        .with_pause(true);
    assert!(paused.paused);
    assert!(paused.label.ends_with(". Tự chuyển đang tạm dừng"));
    let (_, w, h) = render_glyph(&paused, 2);
    assert_eq!((w, h), (52, 36));
    let rows = glyph_ascii(&paused, 1);
    assert_eq!(rows.lines().nth(5), Some(".#+#######.....+#..##+.##+"));
    assert_eq!(rows.lines().nth(8), Some("....++++++++++.....##+.##+"));

    // An empty rotation or no number keeps its own glyph.
    let empty = icon(Some(&snap("claude-exhausted.json", None)), false).with_pause(true);
    assert!(!empty.paused);
    assert!(!empty.label.contains("tạm dừng"));
    assert!(!icon(None, false).with_pause(true).paused);
    assert!(!icon(Some(&snap("claude-pool-schema5.json", None)), false).with_pause(false).paused);
}
