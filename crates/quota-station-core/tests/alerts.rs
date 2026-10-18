//! The five alerts on synthetic sequences of readings, de-duplication, and
//! the buttons through a fake notification center that records every card
//! it shows and every button pressed. Nothing here reads a real account,
//! the Keychain or macOS's notification center.

mod common;

use std::sync::Mutex;

use common::temp_dir;
use quota_station_core::alerts::{
    self, Act, Card, Effect, Facts, Kind, Memory, Notifier, Overwrite, Switches, Tone,
};
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary, Standing};
use quota_station_core::antigravity::switch::How;
use quota_station_core::antigravity::watch;
use quota_station_core::axi::{self, Response};
use quota_station_core::firstmate;
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

/// One slot of claude-swap's list as quota-axi reports it.
fn slot(n: u32, left: f64, active: bool) -> Value {
    json!({
        "slot": n, "active": active, "disabled": false, "status": "fresh",
        "ageSeconds": 60, "fetchedAt": "2026-01-16T07:19:00.000Z",
        "effectivePercentRemaining": left, "limitedBy": "five_hour",
        "resetsAt": "2026-01-16T10:29:00.000Z", "scopes": []
    })
}

/// A `--provider claude` report with claude-swap's slots and the rotation
/// Team 2 3 4, Cá nhân 1 from 8 to 23, and `expire` entries.
fn report(slots: Vec<Value>, expire: Vec<(u32, i64)>, room: f64) -> Response {
    let active = slots.iter().find(|s| s["active"] == true).and_then(|s| s["slot"].as_u64());
    let text = json!({
        "generatedAt": "2026-01-16T07:20:00.000Z",
        "schemaVersion": 6,
        "providers": [{
            "provider": "claude",
            "windows": [],
            "state": { "status": "fresh" },
            "claudeSwap": { "status": "ok", "activeSlot": active, "slots": slots },
            "claudePool": {
                "status": "pool",
                "rotation": {
                    "team": [2, 3, 4], "personal": 1, "personalHours": "8-23",
                    "expire": expire.iter().map(|(s, t)| json!({ "slot": s, "at": rfc(*t) })).collect::<Vec<_>>()
                },
                "members": [], "excluded": [],
                "scopes": [{ "scope": "all_models", "effectivePercentRemaining": room * 25.0, "slotsOfRoom": room, "slots": [] }]
            }
        }]
    });
    axi::parse(&text.to_string()).expect("synthetic report parses")
}

fn facts<'a>(now: i64, claude: Option<&'a Response>) -> Facts<'a> {
    Facts {
        now: at(now),
        offset: offset(),
        claude,
        rules: None,
        agy: Vec::new(),
        overwrite: None,
        firstmate: None,
    }
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
        alerts::answer(memory, &card.id, &act.id, now, offset()).expect("the answer is taken")
    }
}

/// Runs the rules and shows what they found, as the reader does.
fn run(memory: &mut Memory, f: &Facts, center: &FakeCenter) -> Vec<String> {
    let ids = alerts::evaluate(memory, f, &Switches { firstmate: true, ..Switches::default() });
    alerts::deliver(memory, &ids, center);
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
    let ok =
        report(vec![slot(1, 80.0, false), slot(2, 60.0, true), slot(3, 40.0, false)], vec![], 1.8);
    assert!(run(&mut memory, &facts(T0, Some(&ok)), &center).is_empty());

    // Team A at 8 %, every other account under 15 % but Cá nhân at 12 %.
    let dry = report(
        vec![slot(1, 12.0, false), slot(2, 8.0, true), slot(3, 4.0, false), slot(4, 0.0, false)],
        vec![],
        0.4,
    );
    let shown = run(&mut memory, &facts(T0 + 5 * MIN, Some(&dry)), &center);
    assert_eq!(shown.len(), 1);
    let card = center.last();
    assert_eq!(card.kind, Kind::Pool);
    assert_eq!(card.title, "Hạn mức Claude trong vòng sắp cạn");
    assert_eq!(card.body, "Còn tương đương 0,4 tài khoản. Cá nhân có lại hạn mức lúc 17:29.");
    assert_eq!(labels(&card.actions), ["Sang Cá nhân (còn 12%)", "Thôi"]);

    // Still dry on the next reads, and after a restart: no second card.
    assert!(run(&mut memory, &facts(T0 + 6 * MIN, Some(&dry)), &center).is_empty());
    let mut restarted: Memory =
        serde_json::from_str(&serde_json::to_string(&memory).unwrap()).unwrap();
    assert!(run(&mut restarted, &facts(T0 + 7 * MIN, Some(&dry)), &center).is_empty());

    // Team C reset: room again.
    let back = report(
        vec![slot(1, 12.0, false), slot(2, 8.0, true), slot(3, 4.0, false), slot(4, 100.0, false)],
        vec![],
        1.2,
    );
    run(&mut restarted, &facts(T0 + HOUR, Some(&back)), &center);
    let card = center.last();
    assert_eq!(card.title, "Claude đã có lại hạn mức");
    assert_eq!(card.body, "Còn tương đương 1,2 tài khoản trong vòng.");
    assert!(card.actions.is_empty());
    assert_eq!(center.titles().len(), 2);

    // Dry again later is a new event.
    run(&mut restarted, &facts(T0 + 3 * HOUR, Some(&dry)), &center);
    assert_eq!(center.titles().len(), 3);
}

#[test]
fn the_dry_rule_is_firstmates() {
    let check = |slots: Vec<Value>, expire: Vec<(u32, i64)>, now: i64| {
        let r = report(slots, expire, 0.0);
        alerts::pool_check(&r, None, at(now), offset()).unwrap()
    };
    // 10 % or less in use and nothing else at 15 %: dry.
    assert!(check(vec![slot(2, 10.0, true), slot(3, 14.0, false)], vec![], T0).dry);
    assert!(!check(vec![slot(2, 11.0, true), slot(3, 0.0, false)], vec![], T0).dry);
    assert!(!check(vec![slot(2, 5.0, true), slot(3, 15.0, false)], vec![], T0).dry);
    // An account past its `expire` no longer counts.
    assert!(check(vec![slot(2, 5.0, true), slot(3, 90.0, false)], vec![(3, T0 - MIN)], T0).dry);
    // The personal account counts only during its hours: 03:20 at UTC+7 is outside.
    let night = T0 - 11 * HOUR;
    assert!(check(vec![slot(2, 5.0, true), slot(1, 90.0, false)], vec![], night).dry);
    assert!(!check(vec![slot(2, 5.0, true), slot(1, 90.0, false)], vec![], T0).dry);
    // A disabled account does not count; one past its reset counts as full.
    let mut off = slot(3, 90.0, false);
    off["disabled"] = json!(true);
    assert!(check(vec![slot(2, 5.0, true), off], vec![], T0).dry);
    let mut pending = slot(3, 0.0, false);
    pending["status"] = json!("reset_pending");
    assert!(!check(vec![slot(2, 5.0, true), pending], vec![], T0).dry);
}

#[test]
fn an_account_leaving_the_rotation_is_told_thirty_minutes_before() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let ends = T0 + HOUR;
    let r = report(
        vec![slot(1, 80.0, false), slot(2, 60.0, true), slot(3, 90.0, false)],
        vec![(2, ends)],
        2.0,
    );
    // 31 minutes before: not yet.
    assert!(run(&mut memory, &facts(ends - 31 * MIN, Some(&r)), &center).is_empty());
    run(&mut memory, &facts(ends - 30 * MIN, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.kind, Kind::Leaving);
    assert_eq!(card.title, "Team A rời vòng lúc 15:20");
    assert_eq!(card.body, "Còn 30 phút. Sau đó switcher chuyển sang tài khoản khác trong vòng.");
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
    let r = report(
        vec![slot(2, 60.0, true), slot(3, 50.0, false), slot(4, 90.0, false)],
        vec![(4, ends)],
        2.0,
    );
    run(&mut memory, &facts(ends - 20 * MIN, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.title, "Team C rời vòng lúc 15:20");
    assert_eq!(card.body, "Còn 20 phút. Sau đó tài khoản này không còn trong vòng.");
    assert_eq!(labels(&card.actions), ["Ổn"]);
}

#[test]
fn a_claude_login_ending_within_three_days_or_refused_asks_for_a_new_login() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let mut b = slot(3, 80.0, false);
    b["loginExpiresAt"] = json!(rfc(T0 + 3 * 86400 + HOUR));
    let r = report(vec![slot(2, 60.0, true), b.clone()], vec![], 2.0);
    // Three days and an hour: not yet.
    assert!(run(&mut memory, &facts(T0, Some(&r)), &center).is_empty());
    run(&mut memory, &facts(T0 + 2 * HOUR, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.kind, Kind::Login);
    assert_eq!(card.title, "Team B cần đăng nhập lại");
    assert_eq!(
        card.body,
        "Đăng nhập hết hạn sau 2 ngày (Thứ Hai 19/01). Đăng nhập lại bằng claude-swap trong Terminal."
    );
    assert_eq!(labels(&card.actions), ["Nhắc ngày mai", "Đã biết"]);

    // "Nhắc ngày mai": nothing until 09:00 tomorrow, then once more.
    assert_eq!(center.press(&mut memory, &card, "Nhắc ngày mai", T0 + 2 * HOUR), Effect::None);
    let answer = memory.record(&card.id).unwrap().answer.clone().unwrap();
    assert_eq!(answer.result.unwrap().title, "Sẽ nhắc lại lúc 09:00 ngày mai");
    assert!(run(&mut memory, &facts(T0 + 4 * HOUR, Some(&r)), &center).is_empty());
    let nine = alerts::remind_at(T0 + 2 * HOUR, offset());
    assert_eq!(alerts::clock(nine, offset()), "09:00");
    assert_eq!(run(&mut memory, &facts(nine, Some(&r)), &center).len(), 1);
    let again = center.last();
    assert_ne!(again.id, card.id, "a reminder is a new card");
    assert!(run(&mut memory, &facts(nine + HOUR, Some(&r)), &center).is_empty());
    // "Đã biết" on the reminder: no more.
    center.press(&mut memory, &again, "Đã biết", nine + MIN);
    assert!(run(&mut memory, &facts(nine + 2 * 86400, Some(&r)), &center).is_empty());

    // A refresh token Anthropic refused: the same card, for the error.
    let mut c = slot(4, 0.0, false);
    c["error"] = json!("invalid_grant");
    let mut d = slot(1, 50.0, false);
    d["error"] = json!("http-429");
    let r = report(vec![slot(2, 60.0, true), slot(3, 80.0, false), c, d], vec![], 2.0);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    let card = center.last();
    assert_eq!(card.title, "Team C cần đăng nhập lại");
    assert!(card.body.contains("(invalid_grant)"), "{}", card.body);
    // A passing 429 is not a login to renew.
    assert!(!center.titles().iter().any(|t| t.starts_with("Cá nhân")));
}

#[test]
fn a_refused_login_that_was_mended_can_come_back_later() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let mut c = slot(4, 0.0, false);
    c["error"] = json!("invalid_grant");
    let broken = report(vec![slot(2, 60.0, true), c], vec![], 2.0);
    let mended = report(vec![slot(2, 60.0, true), slot(4, 90.0, false)], vec![], 2.0);
    run(&mut memory, &facts(T0, Some(&broken)), &center);
    run(&mut memory, &facts(T0 + MIN, Some(&broken)), &center);
    assert_eq!(center.titles().len(), 1);
    run(&mut memory, &facts(T0 + HOUR, Some(&mended)), &center);
    run(&mut memory, &facts(T0 + 30 * 86400, Some(&broken)), &center);
    assert_eq!(center.titles().len(), 2);
}

fn app_with_accounts(n: usize) -> AppDir {
    let app = AppDir::new(temp_dir("alerts-agy"));
    let mut idx = AccountIndex::new();
    for i in 1..=n {
        let email = format!("ultra{i}@example.com");
        idx.accounts.push(AccountSummary {
            id: index::account_id(&email),
            email,
            name: None,
            created_at: T0,
            last_used: T0,
        });
    }
    index::save(&app, &idx).unwrap();
    app
}

#[test]
fn antigravity_accounts_that_need_a_login_or_a_verification_are_told() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let app = app_with_accounts(3);
    let ids: Vec<String> = index::load(&app).unwrap().accounts.into_iter().map(|a| a.id).collect();
    index::set_standing(&app, &ids[0], Some(Standing::NeedsLogin { since: T0 })).unwrap();
    index::set_standing(
        &app,
        &ids[2],
        Some(Standing::NeedsVerification {
            since: T0,
            url: Some("https://example.com/verify".into()),
        }),
    )
    .unwrap();
    let mut f = facts(T0, None);
    f.agy = alerts::agy_logins(&app);
    assert_eq!(run(&mut memory, &f, &center).len(), 2);
    let cards = center.cards.lock().unwrap().clone();
    assert_eq!(cards[0].title, "Ultra 1 cần đăng nhập lại");
    assert_eq!(labels(&cards[0].actions), ["Đăng nhập lại", "Đã biết"]);
    assert_eq!(cards[1].title, "Ultra 3 cần xác minh");
    assert_eq!(labels(&cards[1].actions), ["Mở trang xác minh", "Đã biết"]);
    assert_eq!(center.press(&mut memory, &cards[0], "Đăng nhập lại", T0 + MIN), Effect::OpenAgyTab);
    assert_eq!(
        center.press(&mut memory, &cards[1], "Mở trang xác minh", T0 + MIN),
        Effect::OpenVerification { number: 3 }
    );
    // The same standing on the next reads: nothing new.
    let mut f = facts(T0 + 5 * MIN, None);
    f.agy = alerts::agy_logins(&app);
    assert!(run(&mut memory, &f, &center).is_empty());
    // Asked again later (a new `since`): a new card.
    index::set_standing(&app, &ids[0], Some(Standing::NeedsLogin { since: T0 + 86400 })).unwrap();
    let mut f = facts(T0 + 86400, None);
    f.agy = alerts::agy_logins(&app);
    assert_eq!(run(&mut memory, &f, &center).len(), 1);
}

#[test]
fn an_overwrite_within_six_hours_offers_to_switch_back() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let app = app_with_accounts(2);
    let ids: Vec<String> = index::load(&app).unwrap().accounts.into_iter().map(|a| a.id).collect();
    // Switched from Ultra 1 to Ultra 2 at 14:20; Ultra 2 seen; Ultra 1 back at 16:20.
    watch::begin(&app, &ids[0], &ids[1], T0).unwrap();
    watch::observe(&app, Some(&ids[1]), T0 + 5);
    let mut f = facts(T0 + HOUR, None);
    f.overwrite = alerts::overwrite(&app, Some(true));
    assert!(f.overwrite.is_none(), "no overwrite yet");
    watch::observe(&app, Some(&ids[0]), T0 + 2 * HOUR);
    let mut f = facts(T0 + 2 * HOUR, None);
    f.overwrite = alerts::overwrite(&app, Some(true));
    assert_eq!(
        f.overwrite,
        Some(Overwrite {
            to_id: ids[1].clone(),
            to: 2,
            from: 1,
            switched_at: T0,
            at: T0 + 2 * HOUR,
            overwrites: 1,
            running: Some(true)
        })
    );
    run(&mut memory, &f, &center);
    let card = center.last();
    assert_eq!(card.kind, Kind::Overwrite);
    assert_eq!(card.title, "Antigravity đổi về tài khoản cũ");
    assert_eq!(
        card.body,
        "Lúc 16:20 Antigravity.app đổi về Ultra 1. Bạn đã chuyển sang Ultra 2 lúc 14:20."
    );
    assert_eq!(labels(&card.actions), ["Thoát app rồi chuyển lại", "Chuyển lại", "Thôi"]);
    // Once for this overwrite.
    assert!(run(&mut memory, &f, &center).is_empty());

    // "Chuyển lại" switches back, leaving the app running; the card waits for the result.
    let effect = center.press(&mut memory, &card, "Chuyển lại", T0 + 2 * HOUR + MIN);
    assert_eq!(effect, Effect::SwitchAgy { number: 2, how: How::KeepRunning });
    assert!(memory.record(&card.id).unwrap().answer.as_ref().unwrap().result.is_none());
    let outcome = quota_station_core::antigravity::switch::Outcome::Switched {
        number: 2,
        how: How::KeepRunning,
        basis: Some("reading".into()),
        reopened: None,
        saved: true,
    };
    alerts::settle(&mut memory, &card.id, alerts::agy_result(&outcome));
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!((said.tone, said.title.as_str()), (Tone::Ok, "Đã chuyển lại sang Ultra 2"));

    // The app not running: one "Chuyển lại", the plain switch.
    let mut quiet = f.overwrite.clone().unwrap();
    quiet.running = Some(false);
    quiet.at += 1;
    let mut f2 = facts(T0 + 3 * HOUR, None);
    f2.overwrite = Some(quiet.clone());
    run(&mut memory, &f2, &center);
    assert_eq!(labels(&center.last().actions), ["Chuyển lại", "Thôi"]);
    assert_eq!(
        center.press(&mut memory, &center.last(), "Chuyển lại", T0 + 3 * HOUR),
        Effect::SwitchAgy { number: 2, how: How::Plain }
    );

    // After two overwrites the app stops offering to switch back; "Thôi" stops the watch.
    quiet.overwrites = 3;
    quiet.at += 1;
    let mut f3 = facts(T0 + 4 * HOUR, None);
    f3.overwrite = Some(quiet);
    run(&mut memory, &f3, &center);
    let card = center.last();
    assert!(card.body.ends_with("Đây là lần thứ 3. App thôi đề nghị chuyển lại."), "{}", card.body);
    assert_eq!(labels(&card.actions), ["Thôi"]);
    assert_eq!(center.press(&mut memory, &card, "Thôi", T0 + 4 * HOUR), Effect::DismissOverwrite);
}

#[test]
fn firstmate_holding_claude_work_is_told_once_per_hold() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let reading = |text: &str, written: i64| firstmate::Reading {
        line: firstmate::parse(text),
        modified_at: written,
    };
    let dry = reading(
        "fork=ok auto=off cs=ok active=2 pool=dry detail=s2=8%,s3=4%,next-reset=2026-01-16T10:29:00.000Z",
        T0,
    );
    let ok = reading("fork=ok auto=off cs=ok active=2 pool=ok detail=s2=60%,next-reset=-", T0);
    let mut f = facts(T0 + MIN, None);
    f.firstmate = Some(&ok);
    assert!(run(&mut memory, &f, &center).is_empty());
    f.firstmate = Some(&dry);
    run(&mut memory, &f, &center);
    let card = center.last();
    assert_eq!(card.kind, Kind::Firstmate);
    assert_eq!(card.title, "Firstmate đang chờ hạn mức Claude");
    assert_eq!(
        card.body,
        "Hạn mức trong vòng dưới ngưỡng: Firstmate giữ việc Claude mới đến khi có lại. Có lại lúc 17:29."
    );
    assert!(card.actions.is_empty());
    let mut f = facts(T0 + 2 * MIN, None);
    f.firstmate = Some(&dry);
    assert!(run(&mut memory, &f, &center).is_empty());
    // A state file Firstmate stopped writing is not acted on.
    let old = reading("fork=ok auto=off cs=ok active=2 pool=dry detail=-", T0 - HOUR);
    let mut f = facts(T0 + 3 * MIN, None);
    f.firstmate = Some(&old);
    assert!(run(&mut memory, &f, &center).is_empty());
}

#[test]
fn kinds_turned_off_send_nothing_and_cards_macos_refused_stay_in_the_app() {
    let center = FakeCenter { refuse: true, ..FakeCenter::default() };
    let mut memory = Memory::default();
    let ends = T0 + 20 * MIN;
    let r = report(vec![slot(2, 60.0, true), slot(3, 90.0, false)], vec![(2, ends)], 2.0);
    let off = Switches { leaving: false, ..Switches::default() };
    assert!(alerts::evaluate(&mut memory, &facts(T0, Some(&r)), &off).is_empty());
    // Turned on while it still holds: it comes.
    let ids = alerts::evaluate(&mut memory, &facts(T0 + MIN, Some(&r)), &Switches::default());
    alerts::deliver(&mut memory, &ids, &center);
    assert_eq!(ids.len(), 1);
    let record = memory.record(&ids[0]).unwrap();
    assert!(!record.notified, "macOS refused the card; the app keeps it");
    assert_eq!(memory.unanswered(), 1);
}

#[test]
fn a_card_takes_one_answer_whichever_card_was_pressed() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let ends = T0 + 30 * MIN;
    let r = report(vec![slot(2, 60.0, true), slot(3, 90.0, false)], vec![(2, ends)], 2.0);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    let card = center.last();
    // In the app first, then the macOS card.
    assert_eq!(
        alerts::answer(&mut memory, &card.id, "dismiss", T0 + MIN, offset()),
        Ok(Effect::None)
    );
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Đã ghi nhận");
    assert_eq!(
        alerts::answer(&mut memory, &card.id, "switch:3", T0 + 2 * MIN, offset()),
        Err(alerts::AnswerError::Answered)
    );
    assert_eq!(
        alerts::answer(&mut memory, "nope", "dismiss", T0, offset()),
        Err(alerts::AnswerError::Unknown)
    );
    assert_eq!(memory.unanswered(), 0);
}

#[test]
fn a_test_card_shows_the_real_buttons_and_changes_nothing() {
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    for kind in Kind::ALL {
        let alert = alerts::sample(kind, T0, offset());
        assert!(alert.test);
        assert!(alert.body.starts_with("Thử:"), "{}", alert.body);
        let id = alerts::keep_test(&mut memory, alert);
        alerts::deliver(&mut memory, std::slice::from_ref(&id), &center);
    }
    assert_eq!(center.titles().len(), 5);
    let overwrite = center.cards.lock().unwrap()[3].clone();
    assert_eq!(labels(&overwrite.actions), ["Thoát app rồi chuyển lại", "Chuyển lại", "Thôi"]);
    assert_eq!(center.press(&mut memory, &overwrite, "Chuyển lại", T0), Effect::None);
    let said = memory.record(&overwrite.id).unwrap().answer.clone().unwrap().result.unwrap();
    assert_eq!(said.title, "Thông báo thử");
    assert_eq!(memory.unanswered(), 0, "test cards never wait for an answer");
    assert_eq!(center.pressed.lock().unwrap().len(), 1);
}

#[test]
fn memory_survives_the_file_and_an_unreadable_file_starts_afresh() {
    let app = AppDir::new(temp_dir("alerts-memory"));
    assert_eq!(alerts::load(&app), Memory::default());
    let center = FakeCenter::default();
    let mut memory = Memory::default();
    let r = report(vec![slot(2, 60.0, true)], vec![(2, T0 + 10 * MIN)], 1.0);
    run(&mut memory, &facts(T0, Some(&r)), &center);
    alerts::save(&app, &memory).unwrap();
    assert_eq!(alerts::load(&app), memory);
    std::fs::write(alerts::path(&app), "{ not json").unwrap();
    assert_eq!(alerts::load(&app), Memory::default());
}
