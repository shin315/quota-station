// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's sockets for an extension (`quota_station::ext`), filled by the
//! test extension in `common` and left empty by `NoExtension`.

mod common;

use std::path::Path;
use std::sync::Arc;

use common::{test_badge, NotKept, TestExtension, TestState};
use quota_station::ext::{
    claude_done, shadowed, shadowed_tabs, AlertInput, Closing, Extension, Hold, Kept, Local,
    ManageError, ModelInput, NoExtension, NoHooks, ReaderPass, View,
};
use quota_station_core::app_dir::AppDir;
use quota_station_core::settings::Settings;
use serde_json::json;
use time::{Duration, OffsetDateTime};

#[test]
fn no_extension_adds_no_command() {
    assert!(NoExtension.command_names().is_empty());
    assert!(shadowed(&[]).is_empty());
}

#[test]
fn an_extension_lists_its_commands_as_the_windows_call_them() {
    let mut expected = vec!["test_extension_echo"];
    if cfg!(unix) {
        expected.push("test_extension_unix");
    }
    expected.push("get_model");
    assert_eq!(TestExtension.command_names(), expected);
}

#[test]
fn a_command_the_app_has_stays_the_apps() {
    assert_eq!(shadowed(&TestExtension.command_names()), vec!["get_model"]);
}

#[test]
fn the_state_an_extension_keeps_is_found_again_by_its_type() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState { machine: Some("Mac A".into()), ..TestState::default() });
    kept.keep(state.clone());
    let found = kept.get::<TestState>().expect("kept");
    assert!(Arc::ptr_eq(&found, &state), "the same state, not a copy");
    *found.count.lock().unwrap() += 1;
    assert_eq!(*state.count.lock().unwrap(), 1);
    assert!(kept.get::<NotKept>().is_none(), "only the types kept");

    // Kept again, the newer one is found.
    let newer = Arc::new(TestState::default());
    kept.keep(newer.clone());
    assert!(Arc::ptr_eq(&kept.get::<TestState>().unwrap(), &newer));
}

#[test]
fn without_an_extension_nothing_is_kept() {
    let kept = Kept::default();
    assert!(kept.get::<TestState>().is_none());
    assert!(kept.get::<NotKept>().is_none());
}

fn pass(kept: &Kept, app_dir: &AppDir, read_due: bool) -> bool {
    let pass = ReaderPass {
        kept,
        home: Path::new("/nonexistent/home"),
        app_dir,
        read_due,
        window_open: false,
    };
    match TestExtension.reader_pass(&pass) {
        Some(work) => {
            work()(kept);
            true
        }
        None => false,
    }
}

#[test]
fn an_extension_asks_the_reader_and_works_at_its_passes() {
    let app_dir = AppDir::new("/nonexistent/app");
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(!TestExtension.reader_asked(&kept));
    *state.asks.lock().unwrap() = true;
    assert!(TestExtension.reader_asked(&kept), "the reader goes now");

    // Alone between reads: the pass takes the ask, the work finishes.
    assert!(pass(&kept, &app_dir, false));
    assert!(!TestExtension.reader_asked(&kept));
    assert_eq!(*state.count.lock().unwrap(), 10);

    // Beside a read.
    assert!(pass(&kept, &app_dir, true));
    assert_eq!(*state.count.lock().unwrap(), 11);
    assert_eq!(*state.passes.lock().unwrap(), vec![(false, false), (true, false)]);
}

#[test]
fn without_an_extension_the_reader_neither_waits_for_it_nor_runs_its_work() {
    let app_dir = AppDir::new("/nonexistent/app");
    let kept = Kept::default();
    assert!(!NoExtension.reader_asked(&kept));
    for read_due in [false, true] {
        let pass = ReaderPass {
            kept: &kept,
            home: Path::new("/nonexistent/home"),
            app_dir: &app_dir,
            read_due,
            window_open: true,
        };
        assert!(NoExtension.reader_pass(&pass).is_none());
    }
    // An extension that keeps nothing yet does no work either.
    assert!(!pass(&kept, &app_dir, true));
}

fn model_input<'a>(kept: &'a Kept, settings: &'a Settings, app_dir: &'a AppDir) -> ModelInput<'a> {
    ModelInput {
        kept,
        settings,
        home: Path::new("/nonexistent/home"),
        app_dir,
        claude_accounts: &[],
    }
}

#[test]
fn an_extension_draws_its_part_of_the_model_from_what_the_app_hands_it() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings { machine_name: Some("Mac A".into()), ..Settings::default() };
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    *state.count.lock().unwrap() = 3;
    let part = TestExtension.model(&model_input(&kept, &settings, &app_dir));
    assert_eq!(
        serde_json::Value::Object(part),
        json!({ "testExtension": { "machine": "Mac A", "accounts": [], "count": 3 } })
    );
    assert_eq!(
        serde_json::Value::Object(TestExtension.capabilities()),
        json!({ "testExtension": true })
    );

    // Before it keeps its state, it draws what it has.
    let empty = Kept::default();
    let part = TestExtension.model(&model_input(&empty, &settings, &app_dir));
    assert_eq!(part["testExtension"]["count"], serde_json::Value::Null);
}

#[test]
fn without_an_extension_the_model_and_the_capabilities_gain_nothing() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings::default();
    let mut kept = Kept::default();
    kept.keep(Arc::new(TestState::default()));
    assert!(NoExtension.model(&model_input(&kept, &settings, &app_dir)).is_empty());
    assert!(NoExtension.capabilities().is_empty());
}

#[test]
fn an_extension_adds_its_keys_to_the_apps_views_from_what_the_app_hands_it() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings { machine_name: Some("Mac A".into()), ..Settings::default() };
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    *state.count.lock().unwrap() = 4;
    let input = model_input(&kept, &settings, &app_dir);
    let keys = |view| serde_json::Value::Object(TestExtension.view_keys(view, &input));
    assert_eq!(keys(View::ClaudeAccounts), json!({ "testCount": 4 }));
    assert_eq!(keys(View::App), json!({ "testMachine": "Mac A" }));

    // Before it keeps its state, it adds what it has.
    let empty = Kept::default();
    let input = model_input(&empty, &settings, &app_dir);
    let keys = TestExtension.view_keys(View::ClaudeAccounts, &input);
    assert_eq!(serde_json::Value::Object(keys), json!({ "testCount": null }));
}

#[test]
fn without_an_extension_the_apps_views_gain_no_key() {
    let app_dir = AppDir::new("/nonexistent/app");
    let settings = Settings::default();
    let mut kept = Kept::default();
    kept.keep(Arc::new(TestState::default()));
    for view in [View::ClaudeAccounts, View::App] {
        assert!(NoExtension.view_keys(view, &model_input(&kept, &settings, &app_dir)).is_empty());
    }
}

#[test]
fn an_extension_names_its_tabs_and_a_tab_the_app_has_stays_the_apps() {
    assert_eq!(TestExtension.tabs(), ["testExtension", "settings"]);
    assert_eq!(shadowed_tabs(TestExtension.tabs()), ["settings"]);
}

#[test]
fn without_an_extension_the_app_has_only_its_own_tabs() {
    assert!(NoExtension.tabs().is_empty());
    assert!(shadowed_tabs(&[]).is_empty());
}

#[test]
fn an_extension_holds_the_close_while_its_work_is_not_kept() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert_eq!(TestExtension.hold_close(&kept, Closing::MainWindow), Hold::Close);
    assert_eq!(TestExtension.hold_close(&kept, Closing::App), Hold::Close);

    *state.unkept.lock().unwrap() = true;
    assert_eq!(TestExtension.hold_close(&kept, Closing::MainWindow), Hold::Ask { tab: None });
    assert_eq!(
        TestExtension.hold_close(&kept, Closing::App),
        Hold::Ask { tab: Some("testExtension") }
    );
    assert_eq!(
        *state.closes.lock().unwrap(),
        [Closing::MainWindow, Closing::App, Closing::MainWindow, Closing::App]
    );

    // Before it keeps its state, it holds nothing.
    assert_eq!(TestExtension.hold_close(&Kept::default(), Closing::App), Hold::Close);
}

#[test]
fn without_an_extension_the_main_window_closes_and_the_app_quits_at_once() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    *state.unkept.lock().unwrap() = true;
    kept.keep(state.clone());
    for closing in [Closing::MainWindow, Closing::App] {
        assert_eq!(NoExtension.hold_close(&kept, closing), Hold::Close);
    }
    assert!(state.closes.lock().unwrap().is_empty(), "never asked");
    assert_eq!(Hold::default(), Hold::Close);
}

#[test]
fn an_extension_follows_a_manual_switch_and_asks_the_reader_for_a_pass() {
    let mut kept = Kept::default();
    let home = Path::new("/home/of/the/owner");
    // Before it keeps its state, nothing follows and the reader stays.
    assert!(!TestExtension.after_switch(&kept, home));

    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(TestExtension.after_switch(&kept, home));
    assert_eq!(*state.switches.lock().unwrap(), [home.to_path_buf()]);
    assert!(TestExtension.reader_asked(&kept), "what it asks of the reader once woken");
}

#[test]
fn an_extension_follows_a_change_of_the_claude_accounts_and_asks_the_reader() {
    let mut kept = Kept::default();
    // Before it keeps its state, nothing follows and the reader stays.
    assert!(!TestExtension.after_accounts_change(&kept));

    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(TestExtension.after_accounts_change(&kept));
    assert_eq!(*state.accounts_changes.lock().unwrap(), 1);
    assert!(TestExtension.reader_asked(&kept), "what it asks of the reader once woken");
}

#[test]
fn without_an_extension_nothing_follows_a_change_of_the_claude_accounts() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(!NoExtension.after_accounts_change(&kept));
    assert_eq!(*state.accounts_changes.lock().unwrap(), 0, "never asked");
    assert!(!*state.asks.lock().unwrap());
}

#[test]
fn without_an_extension_nothing_follows_a_manual_switch() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert!(!NoExtension.after_switch(&kept, Path::new("/home/of/the/owner")));
    assert!(state.switches.lock().unwrap().is_empty(), "never asked");
    assert!(!*state.asks.lock().unwrap());
}

#[test]
fn an_extension_badges_the_icon_from_what_it_keeps() {
    let now = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
    let mut kept = Kept::default();
    assert_eq!(TestExtension.icon_badge(&kept, now), None, "nothing kept yet");
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    assert_eq!(TestExtension.icon_badge(&kept, now), None);
    *state.badge_until.lock().unwrap() = Some(now + Duration::hours(1));
    assert_eq!(TestExtension.icon_badge(&kept, now), Some(test_badge()));
    assert_eq!(TestExtension.icon_badge(&kept, now + Duration::hours(2)), None, "over");
}

#[test]
fn without_an_extension_the_icon_has_no_badge() {
    let now = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    *state.badge_until.lock().unwrap() = Some(now + Duration::hours(1));
    kept.keep(state);
    assert_eq!(NoExtension.icon_badge(&kept, now), None);
}

fn words(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test]
fn an_extension_answers_its_own_subcommand_from_a_terminal() {
    let hooks = quota_station_core::ext::none();
    assert_eq!(TestExtension.subcommand(&words(&["test-extension"]), hooks), Some(5));
    assert_eq!(TestExtension.subcommand(&words(&["test-extension", "a", "b"]), hooks), Some(7));
    for args in [words(&[]), words(&["nonsense"]), words(&["--test-extension"])] {
        assert_eq!(TestExtension.subcommand(&args, hooks), None, "{args:?}");
    }
}

#[test]
fn without_an_extension_no_subcommand_is_its_own() {
    let hooks = quota_station_core::ext::none();
    for args in [words(&[]), words(&["test-extension"]), words(&["agents", "read"])] {
        assert_eq!(NoExtension.subcommand(&args, hooks), None, "{args:?}");
    }
}

#[test]
fn an_extension_reads_for_its_alerts_and_has_what_it_read_back() {
    let mut kept = Kept::default();
    let state = Arc::new(TestState::default());
    kept.keep(state.clone());
    let settings = Settings { machine_name: Some("Mac A".into()), ..Settings::default() };
    let app_dir = AppDir::new("/nonexistent/app");
    let input =
        AlertInput { settings: &settings, home: Path::new("/nonexistent"), app_dir: &app_dir };
    let read = TestExtension.alert_read(&input).expect("it reads for its alerts");
    let _source = read.source();
    assert!(state.alert_reads.lock().unwrap().is_empty(), "not had back before the rules ran");
    read.done(&kept);
    assert_eq!(*state.alert_reads.lock().unwrap(), [Some("Mac A".to_string())]);
}

#[test]
fn without_an_extension_nothing_is_read_for_the_alerts() {
    let settings = Settings::default();
    let app_dir = AppDir::new("/nonexistent/app");
    let input =
        AlertInput { settings: &settings, home: Path::new("/nonexistent"), app_dir: &app_dir };
    assert!(NoExtension.alert_read(&input).is_none());
}

#[test]
fn an_extensions_action_on_a_claude_account_ends_as_the_apps_own_do() {
    assert_eq!(
        serde_json::to_value(claude_done("testAction", 3, Ok(()))).unwrap(),
        json!({
            "action": "testAction", "ok": true, "error": null, "detail": null,
            "slot": 3, "renewed": false, "imported": null,
        })
    );
    assert_eq!(
        serde_json::to_value(claude_done(
            "testAction",
            3,
            Err(ManageError::Login("the browser was closed".into()))
        ))
        .unwrap(),
        json!({
            "action": "testAction", "ok": false, "error": "login",
            "detail": "the browser was closed", "slot": null, "renewed": false, "imported": null,
        })
    );
}

#[test]
fn an_extension_reads_the_commit_the_app_was_built_from() {
    assert_eq!(quota_station::ext::COMMIT, option_env!("QS_COMMIT"));
}

#[test]
fn without_a_window_the_agents_setup_of_a_home_reads_as_the_app_would() {
    let home = std::env::temp_dir().join(format!("qs-ext-local-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let app_dir = quota_station_core::claude::app_dir(&home);
    std::fs::create_dir_all(&app_dir).unwrap();
    let codex_home = home.join("own-codex");
    let settings = json!({ "machineName": "Test machine", "codexHome": codex_home });
    std::fs::write(app_dir.join("settings.json"), settings.to_string()).unwrap();

    let local = Local::open(home.clone(), &NoHooks);
    assert_eq!(local.home(), home);
    assert_eq!(local.app_dir().path(), app_dir);
    assert_eq!(local.machine(), "Test machine");
    assert_eq!(local.app_version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(local.codex_home(), codex_home);
    let places = local.mcp_places();
    assert_eq!(places.home, home);
    assert_eq!(places.codex_config, codex_home.join("config.toml"));
    let skills = local.skill_places();
    assert_eq!(skills.home, home);
    assert_eq!(local.restore_tools().codex_home, codex_home);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn an_extension_adds_its_part_to_the_apps_diagnostics() {
    assert_eq!(TestExtension.build_name(), Some("test-extension"));
    let mut settings = Settings::default();
    assert_eq!(TestExtension.diagnostics(&settings), ["Test machine unnamed"]);
    settings.machine_name = Some("Mac A".into());
    assert_eq!(TestExtension.diagnostics(&settings), ["Test machine named"]);
    assert_eq!(TestExtension.hidden_keys(), ["testExtension"]);

    let dir = std::env::temp_dir().join(format!("qs-ext-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let app_dir = AppDir::new(&dir);
    assert_eq!(TestExtension.self_check("testCheck", &app_dir), Some(json!({ "file": false })));
    std::fs::write(dir.join("test-check"), "").unwrap();
    assert_eq!(TestExtension.self_check("testCheck", &app_dir), Some(json!({ "file": true })));
    assert_eq!(TestExtension.self_check("system", &app_dir), None, "not its own");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_an_extension_the_apps_diagnostics_are_its_own() {
    let app_dir = AppDir::new("/nonexistent/app");
    assert_eq!(NoExtension.build_name(), None);
    assert!(NoExtension.diagnostics(&Settings::default()).is_empty());
    assert_eq!(NoExtension.self_check("testCheck", &app_dir), None);
    assert!(NoExtension.hidden_keys().is_empty());
}
