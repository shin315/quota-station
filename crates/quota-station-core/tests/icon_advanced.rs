// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The menu bar icon with the advanced part: the accounts named by their
//! place in the rotation and as Ultra N, the whole rotation used up, and the
//! pause's badge (`icon.rs` has the icon without an extension).

mod common;

use common::fixture;
use quota_station_core::advanced::labels::Roles;
use quota_station_core::advanced::snapshot::{role_in, rotation_of, Role};
use quota_station_core::icon::{icon, render_glyph, CodexUse, Context, Icon, Tone};
use quota_station_core::settings::{Menubar, MenubarWindow};
use quota_station_core::Snapshot;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// What the account in use goes by in the title without its email: its
/// place in the rotation.
const A: &str = "Team A";

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).unwrap()
}

/// The snapshot of these reports as this build's app shows it.
fn snap(claude: &str, agy: Option<&str>) -> Snapshot {
    let (claude, agy) = (fixture(claude), agy.map(fixture));
    let mut s = Snapshot::build(Some(&claude), agy.as_ref());
    // With the advanced part's keys, and Claude's pool as the rotation's room.
    quota_station_core::advanced::snapshot::decorate(&mut s, Some(&claude));
    s
}

/// Claude's accounts, Antigravity's live account, Codex on its ChatGPT login.
fn full() -> Snapshot {
    snap("claude-pool.json", Some("agy-single.json"))
        .with_codex(Some(&fixture("codex-chatgpt.json")))
}

/// The tightest window, as the title showed before there was a choice.
fn cx(follow: Menubar) -> Context<'static> {
    shows(follow, MenubarWindow::Tightest)
}

fn shows(follow: Menubar, window: MenubarWindow) -> Context<'static> {
    Context {
        follow,
        window,
        codex: CodexUse::ChatGpt,
        now: at("2026-01-16T07:20:00Z"),
        labels: Some(&Roles),
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
fn hidden_names_give_the_account_its_place() {
    // No email reaches the snapshot while names are hidden.
    let s = full();
    assert_eq!(icon(Some(&s), &cx(Menubar::Claude)).title, format!("{A} 87%"));
    assert_eq!(icon(Some(&s), &cx(Menubar::Gemini)).title, "Gemini 6%");
    let mut personal = full();
    let rotation = rotation_of(&personal.claude);
    for a in &mut personal.claude.accounts {
        a.active = role_in(rotation.as_ref(), a.number) == Role::Personal;
    }
    assert!(icon(Some(&personal), &cx(Menubar::Claude)).title.starts_with("Cá nhân "));
    let mut agy = snap("claude-pool.json", Some("agy-list.json"));
    agy.agy.accounts.iter_mut().for_each(|a| a.email = None);
    assert!(icon(Some(&agy), &cx(Menubar::Gemini)).title.starts_with("Ultra "));
}

#[test]
fn stale_numbers_warn_and_claude_used_up_is_bad() {
    assert_eq!(icon(Some(&snap("claude-stale.json", None)), &cx(Menubar::Claude)).tone, Tone::Warn);
    // The whole rotation used up.
    {
        let ex = icon(Some(&snap("claude-exhausted.json", None)), &cx(Menubar::Claude));
        assert_eq!(ex.tone, Tone::Bad);
        assert!(ex.label.ends_with(". Claude đã cạn"));
    }
}

fn sample(tone: Tone, show_agy: bool) -> Icon {
    Icon {
        title: String::new(),
        tone,
        used: Some(45.0),
        claude: Some(56.75),
        agy: Some(94.0),
        show_agy,
        badge: None,
        label: String::new(),
    }
}

#[test]
fn a_paused_switcher_adds_a_pause_sign_and_says_so() {
    use quota_station_core::advanced::badge::paused as pause_badge;
    let paused = icon(Some(&full()), &cx(Menubar::Claude)).with_badge(Some(pause_badge()));
    assert_eq!(paused.badge, Some(pause_badge()));
    assert!(paused.label.ends_with(". Tự động chuyển đang tạm dừng"));
    let (_, w, h) = render_glyph(&paused, 2);
    assert_eq!((w, h), (36, 36));
    for used in [0.0, 45.0, 100.0] {
        let normal = Icon { used: Some(used), ..sample(Tone::Ok, true) };
        let paused = Icon { badge: Some(pause_badge()), ..normal.clone() };
        let (rgba, w, _) = render_glyph(&paused, 4);
        let alpha = |x: u32, y: u32| rgba[((y * w + x) * 4 + 3) as usize];
        assert_eq!(alpha(30, 26), 255, "left pause bar remains visible");
        assert_eq!(alpha(40, 26), 255, "right pause bar remains visible");
        assert_eq!(alpha(35, 26), 0, "pause bars stay separate even at full quota");
        assert_ne!(render_glyph(&normal, 4).0, rgba);
    }

    // An empty rotation or no number keeps its own glyph.
    let empty = icon(Some(&snap("claude-exhausted.json", None)), &cx(Menubar::Claude))
        .with_badge(Some(pause_badge()));
    assert_eq!(empty.badge, None);
    assert!(!empty.label.contains("tạm dừng"));
    assert_eq!(icon(None, &cx(Menubar::Claude)).with_badge(Some(pause_badge())).badge, None);
    assert_eq!(icon(Some(&full()), &cx(Menubar::Claude)).with_badge(None).badge, None);
}

#[test]
fn without_nang_cao_an_account_without_a_name_goes_by_number() {
    let s = full();
    let plain = |follow| Context { labels: None, ..cx(follow) };
    assert_eq!(icon(Some(&s), &cx(Menubar::Claude)).title, format!("{A} 87%"));
    assert_eq!(icon(Some(&s), &plain(Menubar::Claude)).title, "Tài khoản 2 87%");
    assert!(!icon(Some(&s), &plain(Menubar::Gemini)).title.contains("Ultra"));
    // A name shown wins either way.
    assert_eq!(icon(Some(&named(full())), &plain(Menubar::Claude)).title, "lan.acme2 87%");
}
