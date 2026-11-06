// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The alerts with the advanced part, as the app's extension passes its
//! hooks: alert 2 (an account leaving the rotation), alert 1 under the
//! rotation's policy of the pool, and the
//! app's alerts in the full build's words and names (the core's `alerts`
//! has them without an extension). A fake notification center records
//! every card it shows and every button pressed. Nothing here reads a real
//! account, the Keychain or macOS's notification center.

mod common;

use std::sync::Mutex;

use common::temp_dir;
use quota_station_core::advanced::alerts::{leaving, switched, AlertHooks, Source};
use quota_station_core::advanced::labels::Roles;
use quota_station_core::advanced::pool::{Policy, PoolHooks};
use quota_station_core::alerts::{
    self, Act, Alert, Card, Effect, ExtAnswer, ExtKind, Facts, Kind, Memory, Notifier, Switches,
};
use quota_station_core::app_dir::AppDir;
use quota_station_core::ext::Hooks;
use quota_station_core::labels::Labels;
use quota_station_core::quota::claude::PoolPolicy;
use quota_station_core::quota::Response;
use quota_station_core::rotation::{Expire, Rules};
use quota_station_core::settings::PoolAlert;
use serde_json::{json, Value};
use time::{OffsetDateTime, UtcOffset};

/// 14:20 at UTC+7, a weekday afternoon inside the personal hours.
const T0: i64 = 1_768_548_000;
const MIN: i64 = 60;
const HOUR: i64 = 3600;

fn offset() -> UtcOffset {
    UtcOffset::from_hms(7, 0, 0).unwrap()
}

fn at(t: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(t).unwrap()
}

fn rfc(t: i64) -> String {
    at(t).format(&time::format_description::well_known::Rfc3339).unwrap()
}

/// Claude account `n` by the name the cards give it.
fn name(n: u32) -> String {
    match n {
        1 => "Cá nhân".into(),
        n => format!("Team {}", char::from(b'A' + (n - 2) as u8)),
    }
}

/// The title of alert 1's card.
const DRY: &str = "Hạn mức Claude để tự chuyển sắp cạn";

/// One of the app's Claude accounts as its report gives it.
fn slot(n: u32, left: f64, active: bool) -> Value {
    json!({
        "slot": n, "active": active, "disabled": false, "status": "fresh",
        "ageSeconds": 60, "fetchedAt": "2026-01-16T07:19:00.000Z",
        "effectivePercentRemaining": left, "limitedBy": "five_hour",
        "resetsAt": "2026-01-16T10:29:00.000Z", "scopes": []
    })
}

/// A Claude report with the app's accounts and the rotation Team 2 3 4,
/// Cá nhân 1 from 8 to 23.
fn report(slots: Vec<Value>) -> Response {
    rotating(slots, vec![])
}

/// The same with `expire` entries.
fn rotating(slots: Vec<Value>, expire: Vec<(u32, i64)>) -> Response {
    serde_json::from_value(report_value(slots, expire)).expect("synthetic report parses")
}

fn report_value(slots: Vec<Value>, expire: Vec<(u32, i64)>) -> Value {
    let active = slots.iter().find(|s| s["active"] == true).and_then(|s| s["slot"].as_u64());
    json!({
        "generatedAt": "2026-01-16T07:20:00.000Z",
        "providers": [{
            "provider": "claude",
            "windows": [],
            "state": { "status": "fresh" },
            "claudeAccounts": {
                "activeSlot": active,
                "slots": slots,
                "rotation": {
                    "team": [2, 3, 4], "personal": 1, "personalHours": "8-23",
                    "expire": expire.iter().map(|(s, t)| json!({ "slot": s, "at": rfc(*t) })).collect::<Vec<_>>()
                },
                "pool": { "scope": "all_models", "members": 4, "effectivePercentRemaining": 50.0, "slotsOfRoom": 2.0, "slots": [] }
            }
        }]
    })
}

/// The same report without the rotation: the app then reads the rotation
/// file itself.
fn report_without_rotation(slots: Vec<Value>) -> Response {
    let mut text = report_value(slots, vec![]);
    text["providers"][0]["claudeAccounts"]["rotation"] = Value::Null;
    serde_json::from_value(text).expect("synthetic report parses")
}

/// The advanced part's hooks the alerts reach: its kinds of alert, and its
/// policy of the Claude pool with alert 1's words.
struct PrivateHooks;

impl Hooks for PrivateHooks {
    fn alert_kinds(&self) -> &'static [ExtKind] {
        AlertHooks.alert_kinds()
    }

    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        AlertHooks.alert_sample(name, now, offset, claude)
    }

    fn pool_answers(&self) -> &'static [ExtAnswer] {
        PoolHooks.pool_answers()
    }
}

/// The hooks the app passes: with the advanced part, its kinds of alert and
/// alert 1's words.
fn hooks() -> &'static dyn Hooks {
    &PrivateHooks
}

/// The advanced part's alerts at a pass with no rotation file.
static NO_RULES: Source<'static> = Source { rules: None, last_log: None };

/// The advanced part's policy of the Claude pool at a pass with no rotation
/// file: the report's rotation decides.
static NO_ROTATION_FILE: Policy = Policy { rules: None };

/// The policy of the Claude pool the app's hooks give at each pass: with the
/// advanced part, its own.
fn policy() -> Option<&'static dyn PoolPolicy> {
    Some(&NO_ROTATION_FILE)
}

/// The names the app's hooks give the accounts on the cards: with the
/// advanced part, its own.
fn names() -> Option<&'static dyn Labels> {
    Some(&Roles)
}

fn facts<'a>(now: i64, claude: Option<&'a Response>) -> Facts<'a> {
    Facts {
        now: at(now),
        offset: offset(),
        claude,
        pool_policy: policy(),
        #[cfg(feature = "antigravity-accounts")]
        agy: Vec::new(),
        #[cfg(feature = "antigravity-accounts")]
        overwrite: None,
        ext: Some(&NO_RULES),
        pool: PoolAlert::default(),
        labels: names(),
    }
}

/// Alert 1's check at 14:20 with these thresholds.
fn check_pool(r: &Response, limits: PoolAlert) -> alerts::PoolCheck {
    alerts::pool_check(r, policy(), limits, at(T0), offset()).unwrap()
}

/// The card "Gửi thử" sends, naming the accounts as the cards do.
fn sample(kind: Kind, now: i64) -> alerts::Alert {
    alerts::sample(&kind, now, offset(), names(), hooks()).unwrap()
}

/// Records every card and every button pressed, as macOS would deliver
/// them; `refuse` stands for notifications turned off.
#[derive(Default)]
struct FakeCenter {
    cards: Mutex<Vec<Card>>,
    withdrawn: Mutex<Vec<String>>,
    pressed: Mutex<Vec<(String, String)>>,
    refuse: bool,
}

impl Notifier for FakeCenter {
    fn show(&self, card: &Card) -> Result<(), String> {
        if self.refuse {
            return Err("notifications are off".into());
        }
        self.cards.lock().unwrap().push(card.clone());
        Ok(())
    }

    fn withdraw(&self, id: &str) {
        self.withdrawn.lock().unwrap().push(id.to_string());
    }
}

impl FakeCenter {
    fn titles(&self) -> Vec<String> {
        self.cards.lock().unwrap().iter().map(|c| c.title.clone()).collect()
    }

    fn last(&self) -> Card {
        self.cards.lock().unwrap().last().cloned().expect("a card was shown")
    }

    /// A button on the macOS card, delivered back to the app the way the
    /// real center calls the delegate.
    fn press(&self, memory: &mut Memory, card: &Card, label: &str, now: i64) -> Effect {
        let act = card.actions.iter().find(|a| a.label == label).expect("the card has the button");
        self.pressed.lock().unwrap().push((card.id.clone(), act.id.clone()));
        alerts::answer(memory, &card.id, &act.id, now, offset(), hooks())
            .expect("the answer is taken")
    }
}

/// Runs the rules and shows what they found, as the reader does.
fn run(memory: &mut Memory, f: &Facts, center: &FakeCenter) -> Vec<String> {
    let ids = alerts::evaluate(memory, f, &Switches::default(), hooks());
    alerts::deliver(memory, &ids, center, hooks());
    ids
}

fn labels(actions: &[Act]) -> Vec<&str> {
    actions.iter().map(|a| a.label.as_str()).collect()
}

#[test]
fn the_pool_running_dry_and_coming_back_are_two_cards() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    // Team A in use with room: nothing.
    let ok = report(vec![slot(1, 80.0, false), slot(2, 60.0, true), slot(3, 40.0, false)]);
    assert!(run(&mut memory, &facts(T0, Some(&ok)), &center).is_empty());

    // Team A at 8 %, every other account under 15 % but Cá nhân at 12 %.
    let dry = report(vec![
        slot(1, 12.0, false),
        slot(2, 8.0, true),
        slot(3, 4.0, false),
        slot(4, 0.0, false),
    ]);
    let shown = run(&mut memory, &facts(T0 + 5 * MIN, Some(&dry)), &center);
    assert_eq!(shown.len(), 1);
    let card = center.last();
    assert_eq!(card.kind, Kind::Pool);
    assert_eq!(card.title, DRY);
    // The account in use and when another comes back, never a room in accounts.
    assert_eq!(
        card.body,
        format!("{} đã dùng 92%. {} có lại hạn mức lúc 17:29.", name(2), name(1))
    );
    let switch = format!("Sang {} (đã dùng 88%)", name(1));
    assert_eq!(labels(&card.actions), [switch.as_str(), "Bỏ qua"]);

    // Still dry on the next reads, and after a restart: no second card.
    assert!(run(&mut memory, &facts(T0 + 6 * MIN, Some(&dry)), &center).is_empty());
    let mut restarted: Memory =
        serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();
    assert!(run(&mut restarted, &facts(T0 + 7 * MIN, Some(&dry)), &center).is_empty());

    // Team C reset: room again.
    let back = report(vec![
        slot(1, 12.0, false),
        slot(2, 8.0, true),
        slot(3, 4.0, false),
        slot(4, 100.0, false),
    ]);
    run(&mut restarted, &facts(T0 + HOUR, Some(&back)), &center);
    let card = center.last();
    assert_eq!(card.title, "Claude đã có lại hạn mức");
    assert_eq!(card.body, format!("{} có lại hạn mức: đã dùng 0%.", name(4)));
    assert!(card.actions.is_empty());
    assert_eq!(center.titles().len(), 2);

    // Dry again later is a new event.
    run(&mut restarted, &facts(T0 + 3 * HOUR, Some(&dry)), &center);
    assert_eq!(center.titles().len(), 3);
}

#[test]
fn the_account_in_use_is_dry_when_no_other_has_room() {
    let check = |slots: Vec<Value>| check_pool(&report(slots), PoolAlert::default()).dry;
    // 10 % or less in use and nothing else at 15 %: dry.
    assert!(check(vec![slot(2, 10.0, true), slot(3, 14.0, false)]));
    assert!(!check(vec![slot(2, 11.0, true), slot(3, 0.0, false)]));
    assert!(!check(vec![slot(2, 5.0, true), slot(3, 15.0, false)]));
    // One past its reset counts as full.
    let mut pending = slot(3, 0.0, false);
    pending["status"] = json!("reset_pending");
    assert!(!check(vec![slot(2, 5.0, true), pending]));
}

#[test]
fn the_dry_rule_is_the_rotations() {
    let check = |slots: Vec<Value>, expire: Vec<(u32, i64)>, now: i64| {
        let r = rotating(slots, expire);
        alerts::pool_check(&r, policy(), PoolAlert::default(), at(now), offset()).unwrap()
    };
    // An account past its `expire` no longer counts.
    assert!(check(vec![slot(2, 5.0, true), slot(3, 90.0, false)], vec![(3, T0 - MIN)], T0).dry);
    // The personal account counts only during its hours: 03:20 at UTC+7 is outside.
    let night = T0 - 11 * HOUR;
    assert!(check(vec![slot(2, 5.0, true), slot(1, 90.0, false)], vec![], night).dry);
    assert!(!check(vec![slot(2, 5.0, true), slot(1, 90.0, false)], vec![], T0).dry);
    // A disabled account does not count.
    let mut off = slot(3, 90.0, false);
    off["disabled"] = json!(true);
    assert!(check(vec![slot(2, 5.0, true), off], vec![], T0).dry);
}

#[test]
fn an_account_leaving_the_rotation_is_told_thirty_minutes_before() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let ends = T0 + HOUR;
    let r = rotating(
        vec![slot(1, 80.0, false), slot(2, 60.0, true), slot(3, 90.0, false)],
        vec![(2, ends)],
    );
    // 31 minutes before: not yet.
    assert!(run(&mut memory, &facts(ends - 31 * MIN, Some(&r)), &center).is_empty());
    run(&mut memory, &facts(ends - 30 * MIN, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.kind, leaving());
    assert_eq!(card.title, "Team A: ngừng dùng lúc 15:20");
    assert_eq!(card.body, "Còn 30 phút. Sau đó tự động chuyển sẽ đổi sang tài khoản khác.");
    assert_eq!(labels(&card.actions), ["Ổn", "Chuyển ngay"]);
    // Once only, up to the end; past it, nothing.
    assert!(run(&mut memory, &facts(ends - 10 * MIN, Some(&r)), &center).is_empty());
    assert!(run(&mut memory, &facts(ends + MIN, Some(&r)), &center).is_empty());
    assert_eq!(center.titles().len(), 1);

    // "Chuyển ngay" opens the switch sheet for the account with the most left.
    assert_eq!(
        center.press(&mut memory, &card, "Chuyển ngay", ends - 9 * MIN),
        Effect::OpenClaudeSwitch { slot: 3 }
    );

    // An account not in use leaving: no switch button.
    let r = rotating(
        vec![slot(2, 60.0, true), slot(3, 50.0, false), slot(4, 90.0, false)],
        vec![(4, ends)],
    );
    run(&mut memory, &facts(ends - 20 * MIN, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.title, "Team C: ngừng dùng lúc 15:20");
    assert_eq!(card.body, "Còn 20 phút. Sau đó tự động chuyển không dùng tài khoản này nữa.");
    assert_eq!(labels(&card.actions), ["Ổn"]);
}

#[test]
fn without_the_reports_rotation_the_rotation_file_says_who_leaves() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let ends = T0 + HOUR;
    let r = report_without_rotation(vec![slot(2, 60.0, true), slot(3, 90.0, false)]);
    let rules = Rules { expire: vec![Expire { slot: 2, at: rfc(ends) }], ..common::sample_rules() };
    let policy = Policy { rules: Some(rules.clone()) };
    let source = Source { rules: Some(&rules), last_log: None };
    let mut f = facts(ends - 30 * MIN, Some(&r));
    f.pool_policy = Some(&policy);
    f.ext = Some(&source);
    run(&mut memory, &f, &center);
    let card = center.last();
    assert_eq!(card.kind, leaving());
    // Named as the windows name it without the report's rotation.
    assert_eq!(card.title, "Tài khoản 2: ngừng dùng lúc 15:20");
    assert_eq!(labels(&card.actions), ["Ổn", "Chuyển ngay"]);
}

/// The rotation's log line `text`, written at `t`.
fn log_line(t: i64, text: &str) -> quota_station_core::switcher::LogLine {
    let stamp = quota_station_core::local_time::stamp(at(t)).replace('T', " ");
    quota_station_core::switcher::parse_log_line(&format!("{stamp} {text}"))
}

/// "Tự chuyển khi hết quota" tells of each switch once, with Nâng cao off
/// too; the Team rules' switches and old switches say nothing.
#[test]
fn an_auto_switch_is_told_once() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let r = report_without_rotation(vec![slot(2, 60.0, false), slot(3, 90.0, true)]);
    let line = log_line(T0, "switch to slot 3 (full 5h: slot 2 at 91%, 12% used)");
    let source = Source { rules: None, last_log: Some(&line) };
    let mut f = facts(T0 + 2 * MIN, Some(&r));
    f.ext = Some(&source);
    let switches = Switches::default();
    assert!(switches.on(&switched(), hooks()), "on by default");
    run(&mut memory, &f, &center);
    let card = center.last();
    assert_eq!(card.kind, switched());
    assert_eq!(card.number, 9);
    assert_eq!(card.title, "Đã tự chuyển Claude sang Tài khoản 3");
    assert_eq!(
        card.body,
        "Tài khoản 2 đã dùng 91% hạn mức 5 giờ. Phiên Claude Code đang chạy vẫn chạy tiếp."
    );
    assert_eq!(labels(&card.actions), ["Ổn"]);
    // Once per switch.
    f.now = at(T0 + 5 * MIN);
    assert!(run(&mut memory, &f, &center).is_empty());

    for (t, text) in [
        // A Team rules switch.
        (T0, "switch to slot 3 (20.0%, ends 15:00)"),
        // A failed one.
        (T0, "switch to slot 3 failed: busy: lock held"),
        // Long ago.
        (T0 - 2 * HOUR, "switch to slot 3 (full week: slot 2 at 95%, 1% used)"),
    ] {
        let line = log_line(t, text);
        let source = Source { rules: None, last_log: Some(&line) };
        let mut f = facts(T0 + 2 * MIN, Some(&r));
        f.ext = Some(&source);
        assert!(run(&mut Memory::default(), &f, &center).is_empty(), "{text}");
    }
}

#[test]
fn a_claude_login_refused_asks_for_a_new_login() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    // A refresh token Anthropic refused.
    let mut c = slot(4, 0.0, false);
    c["error"] = json!("invalid_grant");
    let mut d = slot(1, 50.0, false);
    d["error"] = json!("http-429");
    let r = report(vec![slot(2, 60.0, true), slot(3, 80.0, false), c, d]);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.kind, Kind::Login);
    assert_eq!(card.title, format!("{} cần đăng nhập lại", name(4)));
    assert!(card.body.contains("(invalid_grant)"), "{}", card.body);
    assert_eq!(labels(&card.actions), ["Nhắc ngày mai", "Đã biết"]);
    // A passing 429 is not a login to renew.
    assert!(!center.titles().iter().any(|t| t.starts_with(&name(1))));

    // "Nhắc ngày mai": nothing until 09:00 tomorrow, then once more.
    assert_eq!(center.press(&mut memory, &card, "Nhắc ngày mai", T0 + MIN), Effect::None);
    let answer = memory.record(&card.id).unwrap().answer.clone().unwrap();
    assert_eq!(answer.result.unwrap().title, "Sẽ nhắc lại lúc 09:00 ngày mai");
    assert!(run(&mut memory, &facts(T0 + 2 * HOUR, Some(&r)), &center).is_empty());
    let nine = alerts::remind_at(T0 + MIN, offset());
    assert_eq!(alerts::clock(nine, offset()), "09:00");
    assert_eq!(run(&mut memory, &facts(nine, Some(&r)), &center).len(), 1);
    let again = center.last();
    assert_ne!(again.id, card.id, "a reminder is a new card");
    assert!(run(&mut memory, &facts(nine + HOUR, Some(&r)), &center).is_empty());
    // "Đã biết" on the reminder: no more.
    center.press(&mut memory, &again, "Đã biết", nine + MIN);
    assert!(run(&mut memory, &facts(nine + 2 * 86400, Some(&r)), &center).is_empty());

    // Any other error that asks for a new login goes on the card as a code,
    // never as the error's own words.
    let mut e = slot(3, 0.0, false);
    e["error"] = json!("Relogin required for owner@example.com: token revoked");
    let r = report(vec![slot(2, 60.0, true), e]);
    run(&mut memory, &facts(T0 + MIN, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.title, format!("{} cần đăng nhập lại", name(3)));
    assert!(card.body.contains("(relogin)"), "{}", card.body);
    assert!(!format!("{card:?}").contains("example.com"), "{card:?}");
    assert_eq!(alerts::relogin_code(" INVALID_GRANT "), Some("invalid_grant"));
    assert_eq!(alerts::relogin_code("http-429"), None);
}

#[test]
fn a_refused_login_that_was_mended_can_come_back_later() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let mut c = slot(4, 0.0, false);
    c["error"] = json!("invalid_grant");
    let broken = report(vec![slot(2, 60.0, true), c]);
    let mended = report(vec![slot(2, 60.0, true), slot(4, 90.0, false)]);
    run(&mut memory, &facts(T0, Some(&broken)), &center);
    run(&mut memory, &facts(T0 + MIN, Some(&broken)), &center);
    assert_eq!(center.titles().len(), 1);
    run(&mut memory, &facts(T0 + HOUR, Some(&mended)), &center);
    run(&mut memory, &facts(T0 + 30 * 86400, Some(&broken)), &center);
    assert_eq!(center.titles().len(), 2);
}

#[test]
fn kinds_turned_off_send_nothing_and_cards_macos_refused_stay_in_the_app() {
    let center = FakeCenter { refuse: true, ..FakeCenter::default() };
    let mut memory = Memory::default();
    let ends = T0 + 20 * MIN;
    let r = rotating(vec![slot(2, 60.0, true), slot(3, 90.0, false)], vec![(2, ends)]);
    let mut off = Switches::default();
    off.set(&leaving(), false, hooks());
    assert!(alerts::evaluate(&mut memory, &facts(T0, Some(&r)), &off, hooks()).is_empty());
    // Turned on while it still holds: it comes.
    let ids =
        alerts::evaluate(&mut memory, &facts(T0 + MIN, Some(&r)), &Switches::default(), hooks());
    alerts::deliver(&mut memory, &ids, &center, hooks());
    assert_eq!(ids.len(), 1);
    let record = memory.record(&ids[0]).unwrap();
    assert!(!record.notified, "macOS refused the card; the app keeps it");
    assert_eq!(memory.unanswered(), 1);
}

// Its cards are alert 2's.
#[test]
fn a_card_takes_one_answer_whichever_card_was_pressed() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let ends = T0 + 30 * MIN;
    let r = rotating(vec![slot(2, 60.0, true), slot(3, 90.0, false)], vec![(2, ends)]);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    let card = center.last();
    // In the app first: the macOS card is withdrawn, it has nothing left to ask.
    assert_eq!(
        alerts::answer_in_app(
            &mut memory,
            &card.id,
            "dismiss",
            T0 + MIN,
            offset(),
            &center,
            hooks()
        ),
        Ok(Effect::None)
    );
    assert_eq!(*center.withdrawn.lock().unwrap(), vec![card.id.clone()]);
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Đã ghi nhận");
    // Then the macOS card, pressed before it went: refused, nothing else withdrawn.
    assert_eq!(
        alerts::answer(&mut memory, &card.id, "switch:3", T0 + 2 * MIN, offset(), hooks()),
        Err(alerts::AnswerError::Answered)
    );
    assert_eq!(
        alerts::answer_in_app(&mut memory, "nope", "dismiss", T0, offset(), &center, hooks()),
        Err(alerts::AnswerError::Unknown)
    );
    assert_eq!(center.withdrawn.lock().unwrap().len(), 1);
    assert_eq!(memory.unanswered(), 0);

    // A button on the macOS card leaves that card to macOS.
    let r = rotating(vec![slot(2, 60.0, true), slot(4, 90.0, false)], vec![(4, ends)]);
    run(&mut memory, &facts(T0 + 5 * MIN, Some(&r)), &center);
    let other = center.last();
    center.press(&mut memory, &other, "Ổn", T0 + 6 * MIN);
    assert_eq!(center.withdrawn.lock().unwrap().len(), 1);
}

#[test]
fn a_test_card_shows_the_real_buttons_and_changes_nothing() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    for kind in Kind::all(hooks()) {
        let alert = sample(kind, T0);
        assert!(alert.test);
        assert!(alert.body.starts_with("Thử:"), "{}", alert.body);
        assert!(!alert.body.contains("tương đương"), "{}", alert.body);
        let id = alerts::keep_test(&mut memory, alert);
        alerts::deliver(&mut memory, std::slice::from_ref(&id), &center, hooks());
    }
    assert_eq!(center.titles().len(), Kind::all(hooks()).len());
    let cards = center.cards.lock().unwrap().clone();
    let overwrite = cards.into_iter().find(|c| c.kind == Kind::Overwrite).unwrap();
    assert_eq!(labels(&overwrite.actions), ["Thoát app rồi chuyển lại", "Chuyển lại", "Bỏ qua"]);
    assert_eq!(center.press(&mut memory, &overwrite, "Chuyển lại", T0), Effect::None);
    let said = memory.record(&overwrite.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Thông báo thử");
    assert_eq!(memory.unanswered(), 0, "test cards never wait for an answer");
    assert_eq!(center.pressed.lock().unwrap().len(), 1);
}

#[test]
fn the_bell_counts_the_cards_since_its_panel_was_opened() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    for kind in [Kind::Pool, Kind::Login] {
        let id = alerts::keep_test(&mut memory, sample(kind, T0));
        alerts::deliver(&mut memory, std::slice::from_ref(&id), &center, hooks());
    }
    // Test cards are notifications like any other: they count until read.
    assert_eq!(memory.unread(), 2);
    assert!(memory.mark_read());
    assert_eq!(memory.unread(), 0);
    assert!(!memory.mark_read(), "nothing left to mark");
    // A card sent after the panel was opened counts again.
    let id = alerts::keep_test(&mut memory, sample(Kind::Overwrite, T0 + MIN));
    assert_eq!(memory.unread(), 1);
    assert!(!memory.record(&id).unwrap().read);
}

#[test]
fn cards_kept_before_the_bell_count_as_read() {
    let app = AppDir::new(temp_dir("alerts-read"));
    let mut memory = Memory::default();
    alerts::keep_test(&mut memory, sample(Kind::Pool, T0));
    let mut kept = serde_json::to_value(&memory).unwrap();
    // The file as the app wrote it before: no `read` on its cards.
    kept["recent"][0].as_object_mut().unwrap().remove("read");
    std::fs::create_dir_all(alerts::path(&app).parent().unwrap()).unwrap();
    std::fs::write(alerts::path(&app), kept.to_string()).unwrap();
    let loaded = alerts::load(&app, hooks());
    assert_eq!(loaded.recent.len(), 1);
    assert_eq!(loaded.unread(), 0);
}

#[test]
fn memory_survives_the_file_and_an_unreadable_file_starts_afresh() {
    let app = AppDir::new(temp_dir("alerts-memory"));
    assert_eq!(alerts::load(&app, hooks()), Memory::default());
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let r = report(vec![slot(2, 5.0, true)]);
    assert_eq!(run(&mut memory, &facts(T0, Some(&r)), &center).len(), 1);
    alerts::save(&app, &memory).unwrap();
    assert_eq!(alerts::load(&app, hooks()), memory);
    std::fs::write(alerts::path(&app), "{ not json").unwrap();
    assert_eq!(alerts::load(&app, hooks()), Memory::default());
}

#[test]
fn without_nang_cao_the_cards_go_by_number_and_every_account_counts() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let plain = |now, r| Facts { labels: None, ..facts(now, r) };
    // No rotation anywhere: account 5, in no rotation, still has room.
    let room = report_without_rotation(vec![slot(1, 5.0, true), slot(5, 50.0, false)]);
    assert!(run(&mut memory, &plain(T0, Some(&room)), &center).is_empty());
    let dry = report_without_rotation(vec![slot(1, 5.0, true), slot(5, 12.0, false)]);
    run(&mut memory, &plain(T0 + MIN, Some(&dry)), &center);
    let card = center.last();
    assert_eq!(card.title, "Hạn mức Claude sắp cạn");
    assert_eq!(card.body, "Tài khoản 1 đã dùng 95%. Tài khoản 1 có lại hạn mức lúc 17:29.");
    assert_eq!(labels(&card.actions), ["Sang Tài khoản 5 (đã dùng 88%)", "Bỏ qua"]);
    // "Gửi thử" speaks the same way.
    let sample = alerts::sample(&Kind::Pool, T0, offset(), None, hooks()).unwrap();
    assert_eq!(sample.title, "Hạn mức Claude sắp cạn");
    assert!(
        sample.body.contains("Tài khoản 1") && !sample.body.contains("Team"),
        "{}",
        sample.body
    );
    let overwrite = alerts::sample(&Kind::Overwrite, T0, offset(), None, hooks()).unwrap();
    assert!(overwrite.body.contains("Antigravity 1") && !overwrite.body.contains("Ultra"));
}

#[test]
fn alert_one_uses_the_thresholds_from_settings() {
    let r = report(vec![slot(2, 18.0, true), slot(3, 25.0, false)]);
    let check = |limits| check_pool(&r, limits).dry;
    assert!(!check(PoolAlert::default()), "18 % left is not dry at 10 %");
    assert!(check(PoolAlert { dry_at_most: 20, room_at_least: 30 }));
    assert!(!check(PoolAlert { dry_at_most: 20, room_at_least: 25 }), "25 % elsewhere is room");
}

/// "Gửi thử" for `kind` with `hooks`, naming the accounts by number.
fn sample_with(kind: &Kind, now: i64, hooks: &dyn Hooks) -> Option<alerts::Alert> {
    alerts::sample(kind, now, offset(), None, hooks)
}

/// A pool gone dry and a refused login, the app's alerts 1 and 3.
fn dry_and_refused() -> Response {
    let mut refused = slot(4, 0.0, false);
    refused["error"] = json!("invalid_grant");
    report(vec![slot(2, 5.0, true), slot(3, 4.0, false), refused])
}

/// An extension's kind (`testKind`, number 2, off by default) is one more
/// kind beside the app's: its cards come from what the extension read,
/// among the app's in the order of the numbers, once each, only while
/// switched on; a kind its hooks do not have never goes out; its buttons
/// do what they do on any card, in its own words where it has some; it
/// keeps its own memory in the alerts' file; and "Gửi thử" sends its sample.
#[test]
fn an_extensions_kind_alerts_among_the_apps_and_is_switched_and_answered_like_them() {
    use common::ext::{test_kind, TestHooks, TestSource};
    let hooks: &dyn Hooks = &TestHooks;
    assert_eq!(Kind::all(hooks), [Kind::Pool, test_kind(), Kind::Login, Kind::Overwrite]);
    assert_eq!(test_kind().number(hooks), 2);
    assert!(test_kind().known(hooks) && !Kind::named("testStranger").known(hooks));
    assert_eq!(Kind::named("testKind"), test_kind());
    assert_eq!(serde_json::to_value(test_kind()).unwrap(), json!("testKind"));
    assert_eq!(serde_json::from_value::<Kind>(json!("login")).unwrap(), Kind::Login);

    let r = dry_and_refused();
    let source = TestSource { events: vec![3] };
    let mut f = facts(T0, Some(&r));
    f.ext = Some(&source);
    // Off by default: the app's two cards only, as without it.
    let mut memory = Memory::default();
    let ids = alerts::evaluate(&mut memory, &f, &Switches::default(), hooks);
    assert_eq!(ids.len(), 2, "{ids:?}");
    assert_eq!(memory.ext.get("testPasses"), Some(&json!(1)), "the extension ran all the same");

    // Switched on: its card comes between the app's 1 and 3, once.
    let mut on = Switches::default();
    on.set(&test_kind(), true, hooks);
    let mut memory = Memory::default();
    let ids = alerts::evaluate(&mut memory, &f, &on, hooks);
    let kinds: Vec<Kind> =
        ids.iter().map(|id| memory.record(id).unwrap().alert.kind.clone()).collect();
    assert_eq!(kinds, [Kind::Pool, test_kind(), Kind::Login]);
    let center = FakeCenter::default();
    alerts::deliver(&mut memory, &ids, &center, hooks);
    let card = center.cards.lock().unwrap()[1].clone();
    assert_eq!((card.kind.clone(), card.number), (test_kind(), 2));
    assert_eq!(card.title, format!("Test event for {}", name(3)));
    // After a restart, nothing again; its memory went with the file.
    let app = AppDir::new(temp_dir("alerts-ext"));
    alerts::save(&app, &memory).unwrap();
    let mut memory = alerts::load(&app, hooks);
    assert_eq!(memory.ext.get("testPasses"), Some(&json!(1)));
    let mut f = facts(T0 + MIN, Some(&r));
    f.ext = Some(&source);
    assert!(alerts::evaluate(&mut memory, &f, &on, hooks).is_empty());
    assert_eq!(memory.ext.get("testPasses"), Some(&json!(2)));

    // Its switch button opens the switch sheet, in the app's words; its
    // "dismiss" says its own.
    let go = alerts::answer(&mut memory, &card.id, "switch:3", T0 + MIN, offset(), hooks);
    assert_eq!(go, Ok(Effect::OpenClaudeSwitch { slot: 3 }));
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Đã mở hộp thoại chuyển tài khoản");
    let mut f = facts(T0 + HOUR, Some(&r));
    let other = TestSource { events: vec![2] };
    f.ext = Some(&other);
    let ids = alerts::evaluate(&mut memory, &f, &on, hooks);
    assert_eq!(ids.len(), 1);
    assert_eq!(
        alerts::answer(&mut memory, &ids[0], "dismiss", T0 + HOUR, offset(), hooks),
        Ok(Effect::None)
    );
    let said = memory.record(&ids[0]).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(
        (said.title.as_str(), said.body.as_str()),
        ("Test noted", "Not again for this test event.")
    );

    // "Gửi thử": its sample, keyed and marked by the app; none for a kind
    // its hooks do not have.
    let sample = sample_with(&test_kind(), T0, hooks).expect("its sample");
    assert_eq!(
        (sample.key.as_str(), sample.kind.clone(), sample.test),
        ("test:2:1768548000", test_kind(), true)
    );
    assert_eq!(sample.title, "Test sample for Tài khoản 1 at 14:20");
    assert!(sample_with(&Kind::named("testStranger"), T0, hooks).is_none());
    let id = alerts::keep_test(&mut memory, sample);
    assert_eq!(alerts::answer(&mut memory, &id, "dismiss", T0, offset(), hooks), Ok(Effect::None));
    let said = memory.record(&id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Thông báo thử", "a test card keeps the app's words");
}

/// Alert 2 keeps its number, name (its cards' `kind` and its switch's key),
/// default and words: on by default, it says its own words for "Ổn". It
/// comes among the app's in the order of the numbers, and "Gửi thử" sends
/// it.
#[test]
fn alert_two_keeps_its_number_switch_default_and_words() {
    use quota_station_core::alerts::{ExtAnswer, ExtKind};
    assert_eq!(
        AlertHooks.alert_kinds(),
        [
            ExtKind {
                number: 2,
                name: "leaving",
                on: true,
                answers: &[ExtAnswer {
                    action: "dismiss",
                    title: "Đã ghi nhận",
                    body: "Không nhắc lại cho lần ngừng dùng này.",
                }],
            },
            ExtKind {
                number: 9,
                name: "switched",
                on: true,
                answers: &[ExtAnswer { action: "dismiss", title: "Đã ghi nhận", body: "" }],
            }
        ]
    );
    assert_eq!(
        Kind::all(hooks()),
        [Kind::Pool, leaving(), Kind::Login, Kind::Overwrite, switched()]
    );
    assert_eq!(leaving().number(hooks()), 2);
    assert_eq!(serde_json::to_value(leaving()).unwrap(), json!("leaving"));
    assert!(Switches::default().on(&leaving(), hooks()));

    // "Ổn" on alert 2.
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let r = rotating(vec![slot(2, 60.0, true), slot(3, 90.0, false)], vec![(2, T0 + 30 * MIN)]);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.number, 2);
    center.press(&mut memory, &card, "Ổn", T0 + MIN);
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(
        (said.tone, said.title.as_str(), said.body.as_str()),
        (alerts::Tone::Info, "Đã ghi nhận", "Không nhắc lại cho lần ngừng dùng này.")
    );

    // "Gửi thử", as the build before sent it.
    let two = sample(leaving(), T0);
    assert_eq!(
        (two.key.as_str(), two.title.as_str()),
        ("test:2:1768548000", "Team A: ngừng dùng lúc 14:50")
    );
    assert_eq!(labels(&two.actions), ["Ổn", "Chuyển ngay"]);
}
