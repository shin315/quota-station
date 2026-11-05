// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The alerts on synthetic sequences of readings, de-duplication, and
//! the buttons through a fake notification center that records every card
//! it shows and every button pressed. Nothing here reads a real account,
//! the Keychain or macOS's notification center.

mod common;

use std::sync::Mutex;

use common::temp_dir;
use quota_station_core::alerts::{
    self, Act, Card, Effect, Facts, Kind, Memory, Notifier, Switches,
};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::alerts::{Overwrite, Tone};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary, Standing};
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::switch::How;
#[cfg(feature = "antigravity-accounts")]
use quota_station_core::antigravity::watch;
use quota_station_core::app_dir::AppDir;
use quota_station_core::ext::Hooks;
use quota_station_core::labels::Labels;
use quota_station_core::quota::claude::PoolPolicy;
use quota_station_core::quota::Response;
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

/// Claude account `n` by the name the cards give it.
fn name(n: u32) -> String {
    format!("Tài khoản {n}")
}

const DRY: &str = "Hạn mức Claude sắp cạn";

/// One of the app's Claude accounts as its report gives it.
fn slot(n: u32, left: f64, active: bool) -> Value {
    json!({
        "slot": n, "active": active, "disabled": false, "status": "fresh",
        "ageSeconds": 60, "fetchedAt": "2026-01-16T07:19:00.000Z",
        "effectivePercentRemaining": left, "limitedBy": "five_hour",
        "resetsAt": "2026-01-16T10:29:00.000Z", "scopes": []
    })
}

/// A Claude report with the app's accounts.
fn report(slots: Vec<Value>) -> Response {
    serde_json::from_value(report_value(slots)).expect("synthetic report parses")
}

fn report_value(slots: Vec<Value>) -> Value {
    let active = slots.iter().find(|s| s["active"] == true).and_then(|s| s["slot"].as_u64());
    json!({
        "generatedAt": "2026-01-16T07:20:00.000Z",
        "providers": [{
            "provider": "claude",
            "windows": [],
            "state": { "status": "fresh" },
            "claudeAccounts": { "activeSlot": active, "slots": slots }
        }]
    })
}

/// The hooks the app passes without an extension: none.
fn hooks() -> &'static dyn Hooks {
    quota_station_core::ext::none()
}

/// The policy of the Claude pool the app's hooks give at each pass: with the
/// advanced part, its own; without it, none.
fn policy() -> Option<&'static dyn PoolPolicy> {
    None
}

/// The names the app's hooks give the accounts on the cards: with the
/// advanced part, its own; without it, none.
fn names() -> Option<&'static dyn Labels> {
    None
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
        ext: None,
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
    let switches = Switches::default();
    let ids = alerts::evaluate(memory, f, &switches, hooks());
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

/// Antigravity account `n` by the name the cards give it.
#[cfg(feature = "antigravity-accounts")]
fn agy(n: u32) -> String {
    format!("Antigravity {n}")
}

#[cfg(feature = "antigravity-accounts")]
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

#[cfg(feature = "antigravity-accounts")]
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
    assert_eq!(cards[0].title, format!("{} cần đăng nhập lại", agy(1)));
    assert_eq!(labels(&cards[0].actions), ["Đăng nhập lại", "Đã biết"]);
    assert_eq!(cards[1].title, format!("{} cần xác minh", agy(3)));
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

#[cfg(feature = "antigravity-accounts")]
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
        format!(
            "Lúc 16:20 Antigravity.app đổi về {}. Bạn đã chuyển sang {} lúc 14:20.",
            agy(1),
            agy(2)
        )
    );
    assert_eq!(labels(&card.actions), ["Thoát app rồi chuyển lại", "Chuyển lại", "Bỏ qua"]);
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
    let said = alerts::agy_result(&outcome, names());
    alerts::settle(&mut memory, &card.id, said);
    let said = memory.record(&card.id).unwrap().answer.clone().unwrap().result.unwrap();
    let title = format!("Đã chuyển lại sang {}", agy(2));
    assert_eq!((said.tone, said.title.as_str()), (Tone::Ok, title.as_str()));

    // The app not running: one "Chuyển lại", the plain switch.
    let mut quiet = f.overwrite.clone().unwrap();
    quiet.running = Some(false);
    quiet.at += 1;
    let mut f2 = facts(T0 + 3 * HOUR, None);
    f2.overwrite = Some(quiet.clone());
    run(&mut memory, &f2, &center);
    assert_eq!(labels(&center.last().actions), ["Chuyển lại", "Bỏ qua"]);
    assert_eq!(
        center.press(&mut memory, &center.last(), "Chuyển lại", T0 + 3 * HOUR),
        Effect::SwitchAgy { number: 2, how: How::Plain }
    );

    // After two overwrites the app stops offering to switch back; "Bỏ qua" stops the watch.
    quiet.overwrites = 3;
    quiet.at += 1;
    let mut f3 = facts(T0 + 4 * HOUR, None);
    f3.overwrite = Some(quiet);
    run(&mut memory, &f3, &center);
    let card = center.last();
    assert!(
        card.body.ends_with("Đây là lần thứ 3. Ứng dụng ngừng đề nghị chuyển lại."),
        "{}",
        card.body
    );
    assert_eq!(labels(&card.actions), ["Bỏ qua"]);
    assert_eq!(center.press(&mut memory, &card, "Bỏ qua", T0 + 4 * HOUR), Effect::DismissOverwrite);
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

/// Without an extension, no kind is added: the app's own kinds and nothing
/// else, a source handed over anyway sends nothing, there is no sample of a
/// kind it does not have, and a file with a card of such a kind starts
/// afresh, as a build without that kind always did.
#[test]
fn without_an_extension_no_kind_is_added_and_its_cards_never_go_out() {
    use common::ext::{test_kind, TestHooks, TestSource};
    let none = quota_station_core::ext::none();
    assert_eq!(Kind::all(none), Kind::ALL);
    assert!(!test_kind().known(none));
    assert_eq!(test_kind().number(none), 0);
    assert!(!Switches::default().on(&test_kind(), none));

    let r = dry_and_refused();
    let source = TestSource { events: vec![3] };
    let mut with = facts(T0, Some(&r));
    with.ext = Some(&source);
    let mut without = facts(T0, Some(&r));
    without.ext = None;
    let (mut a, mut b) = (Memory::default(), Memory::default());
    let ids = alerts::evaluate(&mut a, &with, &Switches::default(), none);
    assert_eq!(ids, alerts::evaluate(&mut b, &without, &Switches::default(), none));
    assert_eq!(a.recent, b.recent);
    assert!(sample_with(&test_kind(), T0, none).is_none());

    // A file with a card of the extension's kind.
    let app = AppDir::new(temp_dir("alerts-no-ext"));
    let mut memory = Memory::default();
    let mut on = Switches::default();
    on.set(&test_kind(), true, &TestHooks);
    alerts::evaluate(&mut memory, &with, &on, &TestHooks);
    alerts::save(&app, &memory).unwrap();
    assert_eq!(alerts::load(&app, &TestHooks), memory);
    assert_eq!(alerts::load(&app, none), Memory::default());
}
