// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

mod common;

use common::{fixture, fixture_text};
use quota_station_core::detail::{self, AgyDetail};
use quota_station_core::snapshot::{Line, LineKind, Snapshot};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn parse_ts(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

fn windows(lines: &[Line]) -> Vec<(LineKind, Option<f64>, bool)> {
    lines.iter().map(|l| (l.kind, l.percent, l.pending)).collect()
}

fn reading(name: &str) -> serde_json::Value {
    serde_json::from_str(&fixture_text(name)).unwrap()
}

#[test]
fn each_claude_account_gets_its_windows_email_and_organization() {
    let report = fixture("claude-pool-schema5.json");
    let now = parse_ts(&report.generated_at);
    let mut s = Snapshot::build(Some(&report), None);
    let list = common::claude_details("cswap-list-pool.json");
    detail::apply_claude(&mut s.claude, &list, now);
    let a = &s.claude.accounts;

    // The personal account's five hours have not started: whole, no reset.
    assert_eq!(
        windows(&a[0].windows),
        [
            (LineKind::Session, Some(100.0), false),
            (LineKind::Week, Some(100.0), false),
            (LineKind::FableWeek, Some(96.0), false),
        ]
    );
    assert_eq!(a[0].windows[0].resets_at, None);
    assert_eq!(a[0].email.as_deref(), Some("lanpham.dev@example.com"));
    // The account in use keeps quota-axi's live reading of its windows.
    assert_eq!(
        windows(&a[1].windows),
        [
            (LineKind::Session, Some(26.0), false),
            (LineKind::Week, Some(61.0), false),
            (LineKind::FableWeek, Some(13.0), false),
        ]
    );
    assert_eq!(a[1].org.as_deref(), Some("Acme Studio"));
    // A Team seat claude-swap gives no week has no weekly window.
    assert_eq!(
        windows(&a[2].windows),
        [(LineKind::Session, Some(100.0), false), (LineKind::FableWeek, Some(50.0), false)]
    );
    assert_eq!(a[3].windows[0].percent, Some(98.0));
    assert_eq!(a[3].windows[0].resets_at.as_deref(), Some("2026-01-16T10:09:00+00:00"));

    detail::hide_names(&mut s.claude);
    assert!(s.claude.accounts.iter().all(|a| a.email.is_none() && a.org.is_none()));
}

#[test]
fn a_claude_window_past_its_reset_has_no_number() {
    let list = common::claude_details("cswap-list-pool.json");
    let later = parse_ts("2026-01-16T10:30:00Z");
    let lines = detail::claude_windows(&list[3], later);
    assert_eq!(lines[0].kind, LineKind::Session);
    assert_eq!((lines[0].percent, lines[0].pending), (None, true));
    assert_eq!((lines[1].percent, lines[1].pending), (Some(98.0), false));
}

#[test]
fn an_account_the_app_has_no_reading_of_keeps_only_quota_axis_lines() {
    let report = fixture("claude-pool-schema5.json");
    let mut s = Snapshot::build(Some(&report), None);
    detail::apply_claude(&mut s.claude, &[], parse_ts(&report.generated_at));
    let a = &s.claude.accounts;
    assert!(a[0].windows.is_empty() && a[0].email.is_none());
    assert!(!a[0].lines.is_empty());
}

#[test]
fn each_antigravity_account_gets_its_four_windows_from_the_apps_reading() {
    let report = fixture("agy-quota-station.json");
    let now = parse_ts(&report.generated_at);
    let mut s = Snapshot::build(None, Some(&report));
    assert_eq!(s.agy.source.as_deref(), Some("quota-station"));
    let details = [
        AgyDetail {
            number: 1,
            email: Some("lanpham.ultra@example.com".into()),
            payload: Some(reading("agy-reading-1.json")),
        },
        AgyDetail { number: 2, email: Some("lan.ultra2@example.com".into()), payload: None },
    ];
    detail::apply_agy(&mut s.agy, &details, now);
    let a = &s.agy.accounts;
    assert_eq!(
        windows(&a[0].windows),
        [
            (LineKind::GeminiSession, Some(100.0), false),
            (LineKind::GeminiWeek, Some(63.0), false),
            (LineKind::ClaudeGptSession, Some(90.0), false),
            (LineKind::ClaudeGptWeek, Some(82.0), false),
        ]
    );
    assert_eq!(a[0].email.as_deref(), Some("lanpham.ultra@example.com"));
    // The account in use, with no reading of its own, keeps the live row's.
    assert_eq!(a[1].email.as_deref(), Some("lan.ultra2@example.com"));
    assert_eq!(
        windows(&a[1].windows),
        [
            (LineKind::GeminiSession, Some(98.0), false),
            (LineKind::GeminiWeek, Some(94.0), false),
            (LineKind::ClaudeGptSession, Some(100.0), false),
            (LineKind::ClaudeGptWeek, Some(100.0), false),
        ]
    );
}

#[test]
fn antigravity_readings_are_not_used_for_cockpit_tools_accounts() {
    let report = fixture("agy-cockpit.json");
    let mut s = Snapshot::build(None, Some(&report));
    let details = [AgyDetail {
        number: 1,
        email: Some("lanpham.ultra@example.com".into()),
        payload: Some(reading("agy-reading-1.json")),
    }];
    detail::apply_agy(&mut s.agy, &details, parse_ts(&report.generated_at));
    assert!(s.agy.accounts[0].windows.is_empty());
    assert!(s.agy.accounts.iter().all(|a| a.email.is_none()));
}

#[test]
fn a_bucket_without_a_fraction_is_used_up() {
    let mut payload = reading("agy-reading-2.json");
    let bucket = &mut payload["quota_summary"]["groups"][0]["buckets"][1];
    bucket.as_object_mut().unwrap().remove("remainingFraction");
    let lines = detail::agy_windows(&payload, parse_ts("2026-01-16T07:20:00Z"));
    assert_eq!(lines[0].kind, LineKind::GeminiSession);
    assert_eq!(lines[0].percent, Some(0.0));
    assert_eq!(detail::agy_windows(&serde_json::json!({}), None), []);
}
