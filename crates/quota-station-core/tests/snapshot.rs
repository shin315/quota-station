// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

mod common;

use common::fixture;
use quota_station_core::quota::Response;
use quota_station_core::snapshot::{
    Keychain, LineKind, PoolKind, Reading, SectionStatus, Snapshot, Source, LIST_SOURCE,
};

/// The snapshot of these reports as this build's app shows it.
fn shown(claude: Option<&Response>, agy: Option<&Response>) -> Snapshot {
    #[allow(unused_mut)]
    let mut s = Snapshot::build(claude, agy);
    s
}

fn claude(name: &str) -> Snapshot {
    shown(Some(&fixture(name)), None)
}

fn agy(name: &str) -> Snapshot {
    shown(None, Some(&fixture(name)))
}

#[test]
fn the_pool_is_the_active_accounts_own_numbers() {
    let s = claude("claude-pool.json");
    let pool = &s.claude.pool;
    assert_eq!(pool.kind, PoolKind::Active);
    assert_eq!(pool.percent, Some(26.0));
    assert_eq!(pool.line, Some(LineKind::Session));
    assert_eq!(pool.next_refill_at.as_deref(), Some("2026-01-16T10:29:00.000Z"));
    assert!(!pool.exhausted);
    assert_eq!(pool.reason, None);
}

#[test]
fn the_active_slot_reads_the_live_row_and_others_read_the_list() {
    let s = claude("claude-pool.json");
    let active = s.claude.active().expect("active slot");
    assert_eq!(active.number, 2);
    assert_eq!(active.source, Source::Live);
    assert_eq!(active.reading, Reading::Fresh);
    // Live windows: five_hour 26, seven_day 61, model:fable 13.
    assert_eq!(active.lines[0].kind, LineKind::Session);
    assert_eq!(active.lines[0].percent, Some(26.0));
    assert_eq!(active.lines[0].resets_at.as_deref(), Some("2026-01-16T10:29:00.000Z"));
    assert_eq!(active.lines[1].kind, LineKind::FableWeek);
    assert_eq!(active.lines[1].percent, Some(13.0));

    let personal = &s.claude.accounts[0];
    assert_eq!(personal.source, Source::List);
    // Bound by both account windows: named by the week, which resets last.
    assert_eq!(personal.lines[0].kind, LineKind::Week);
    assert_eq!(personal.lines[0].percent, Some(100.0));
    assert_eq!(personal.lines[1].percent, Some(96.0));

    // Fable limited by its own window: the window's number. Limited by the
    // five-hour window instead: only a lower bound, with no Fable reset.
    assert!(!personal.lines[1].at_least);
    let team_c = &s.claude.accounts[3];
    assert_eq!(team_c.lines[1].percent, Some(98.0));
    assert!(team_c.lines[1].at_least);
    assert_eq!(team_c.lines[1].resets_at, None);

    let unstarted = &s.claude.accounts[2];
    assert_eq!(unstarted.lines[0].kind, LineKind::Session);
    assert_eq!(unstarted.lines[0].resets_at, None);
}

#[test]
fn keychain_not_granted_asks_for_it_and_shows_no_number() {
    // A Mac whose accounts the app does not keep reads Claude Code's own
    // login, which the Keychain holds back until the owner grants it.
    let s = claude("claude-keychain-required.json");
    assert_eq!(s.keychain, Keychain::Required);
    assert_eq!(s.claude.status, SectionStatus::Single);
    assert!(s.claude.accounts.is_empty());
    assert_eq!(s.claude.reason.as_deref(), Some("keychain_prompt_required"));
    assert_eq!(s.claude.pool.kind, PoolKind::None);
}

#[test]
fn the_one_login_read_keeps_its_last_numbers_marked_stale() {
    let report = serde_json::from_value(serde_json::json!({
        "generatedAt": "2026-01-16T07:20:00.000Z",
        "providers": [{
            "provider": "claude",
            "windows": [
                { "id": "five_hour", "kind": "session", "percentUsed": 30, "percentRemaining": 70, "resetsAt": "2026-01-16T09:00:00.000Z" },
                { "id": "seven_day", "kind": "weekly", "percentUsed": 12, "percentRemaining": 88, "resetsAt": "2026-01-19T14:00:00.000Z" }
            ],
            "state": { "status": "stale", "refreshedAt": "2026-01-16T06:50:00.000Z", "error": "Claude quota unavailable" }
        }]
    }))
    .unwrap();
    let s = Snapshot::build(Some(&report), None);
    assert_eq!(s.claude.status, SectionStatus::Single);
    let a = s.claude.active().unwrap();
    assert_eq!((a.reading, a.lines[0].percent), (Reading::Stale, Some(70.0)));
    assert_eq!(a.fetched_at.as_deref(), Some("2026-01-16T06:50:00.000Z"));
    assert!(s.claude.stale);
    assert_eq!(s.claude.reason, None);
}

#[test]
fn a_stale_active_account_falls_back_to_its_list_numbers() {
    let s = claude("claude-stale.json");
    let active = s.claude.active().unwrap();
    assert_eq!(active.source, Source::List);
    assert_eq!(active.lines[0].percent, Some(27.0));
    let pool = &s.claude.pool;
    assert_eq!(pool.kind, PoolKind::Active);
    assert_eq!(pool.percent, Some(27.0));
    assert_eq!(pool.line, Some(LineKind::Session));
}

#[test]
fn stale_readings_mark_the_section_stale() {
    let s = claude("claude-stale.json");
    assert!(s.claude.stale);
    assert!(s.claude.accounts.iter().all(|a| a.reading == Reading::Stale));
    assert_eq!(s.claude.newest_reading_at.as_deref(), Some("2026-01-16T06:39:00.000Z"));
    // Numbers are kept, not dropped.
    assert_eq!(s.claude.pool.percent, Some(27.0));
}

#[test]
fn reset_pending_states_no_number() {
    let s = claude("claude-reset-pending.json");
    let slot = &s.claude.accounts[3];
    assert_eq!(slot.reading, Reading::ResetPending);
    assert_eq!(slot.lines[0].percent, None);
    assert!(slot.lines[0].pending);
    assert!(slot.lines[1].pending);
    assert!(!s.claude.stale);
}

#[test]
fn accounts_without_a_name_show_the_active_slot_only() {
    let s = claude("claude-unnamed.json");
    assert_eq!(s.claude.pool.kind, PoolKind::Active);
    assert_eq!(s.claude.pool.percent, Some(26.0));
    assert_eq!(s.claude.pool.reason, None);
}

#[test]
fn an_unread_login_shows_no_number_and_says_why() {
    let s = claude("claude-unavailable.json");
    assert_eq!(s.claude.status, SectionStatus::Single);
    assert!(s.claude.accounts.is_empty());
    assert_eq!(s.claude.reason.as_deref(), Some("network_unreachable"));
    assert_eq!(s.claude.pool.kind, PoolKind::None);
    assert_eq!(s.keychain, Keychain::Unknown);
}

#[test]
fn the_list_shows_every_antigravity_account() {
    let s = agy("agy-list.json");
    let a = &s.agy;
    assert_eq!(a.status, SectionStatus::Ok);
    assert_eq!(a.source.as_deref(), Some(LIST_SOURCE));
    assert_eq!(a.accounts.len(), 2);
    assert_eq!(a.accounts[0].reading, Reading::Stale);
    assert_eq!(a.accounts[0].lines[0].kind, LineKind::Gemini);
    assert_eq!(a.accounts[0].lines[0].percent, Some(63.0));
    assert_eq!(a.accounts[0].lines[1].kind, LineKind::ClaudeGpt);
    let active = a.active().unwrap();
    assert_eq!(active.number, 2);
    assert!(active.lines[1].pending);
    assert!(!a.stale);
    // The signed-in account's own Gemini number.
    assert_eq!(a.pool.kind, PoolKind::Active);
    assert_eq!(a.pool.percent, Some(94.0));
    assert_eq!(a.pool.line, Some(LineKind::Week));
}

#[test]
fn a_mac_without_the_list_sees_only_the_signed_in_account() {
    let s = agy("agy-single.json");
    let a = &s.agy;
    assert_eq!(a.status, SectionStatus::Single);
    assert_eq!(a.accounts.len(), 1);
    let only = &a.accounts[0];
    assert!(only.active);
    assert_eq!(only.source, Source::Live);
    assert_eq!(only.lines[0].kind, LineKind::Session);
    assert_eq!(only.lines[0].percent, Some(98.0));
    assert_eq!(only.lines[1].kind, LineKind::Week);
    assert_eq!(only.lines[1].percent, Some(94.0));
    assert_eq!(only.fetched_at.as_deref(), Some("2026-01-16T07:16:00.000Z"));
    assert_eq!(a.pool.percent, Some(94.0));
}

#[test]
fn an_unreadable_list_keeps_the_signed_in_account() {
    let s = agy("agy-list-unavailable.json");
    assert_eq!(s.agy.status, SectionStatus::Unavailable);
    assert_eq!(s.agy.reason.as_deref(), Some("index_unreadable"));
    assert_eq!(s.agy.accounts.len(), 1);
}

#[test]
fn a_mac_without_antigravity_has_no_section() {
    let s = agy("agy-not-set-up.json");
    assert_eq!(s.agy.status, SectionStatus::NotSetUp);
    assert!(s.agy.accounts.is_empty());
    assert_eq!(s.agy.pool.kind, PoolKind::None);
}

#[test]
fn nothing_read_yet_is_unread() {
    let s = Snapshot::build(None, None);
    assert_eq!(s.claude.status, SectionStatus::Unread);
    assert_eq!(s.agy.status, SectionStatus::Unread);
    assert_eq!(s.claude.generated_at, None);
    assert_eq!(s.agy.generated_at, None);
}

#[test]
fn each_section_keeps_the_time_of_its_own_report() {
    let claude = fixture("claude-pool.json");
    let mut agy = fixture("agy-list.json");
    agy.generated_at = "2026-01-16T06:50:00.000Z".into();
    let s = Snapshot::build(Some(&claude), Some(&agy));
    assert_eq!(s.claude.generated_at.as_deref(), Some("2026-01-16T07:20:00.000Z"));
    assert_eq!(s.agy.generated_at.as_deref(), Some("2026-01-16T06:50:00.000Z"));
}

#[test]
fn snapshot_serializes_in_camel_case_for_the_ui() {
    let s = shown(Some(&fixture("claude-pool.json")), Some(&fixture("agy-single.json")));
    let v: serde_json::Value = serde_json::to_value(&s).unwrap();
    assert_eq!(v["claude"]["pool"]["kind"], "active");
    assert_eq!(v["claude"]["accounts"][1]["lines"][1]["kind"], "fableWeek");
    assert_eq!(v["agy"]["status"], "single");
    assert_eq!(v["keychain"], "granted");
    assert_eq!(v["claude"]["generatedAt"], "2026-01-16T07:20:00.000Z");
    assert_eq!(v["agy"]["generatedAt"], "2026-01-16T07:20:00.000Z");
}
