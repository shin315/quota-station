// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part in the app's state (`quota_station::testing`), as the
//! app's binary runs it: the icon's badge, the model and its golden file,
//! the hold on a close, and the reader's look at the switcher.

use std::path::PathBuf;
use std::sync::Arc;

use quota_station::advanced::state::AdvancedState;
use quota_station::advanced::Advanced;
use quota_station::ext::{Closing, Extension, Kept, NoExtension, Tab};
use quota_station::testing::{OnClose, TestApp};
use quota_station_core::Snapshot;
use time::OffsetDateTime;

/// The app with its advanced part, its settings file nowhere.
fn app() -> TestApp {
    TestApp::new(
        PathBuf::new(),
        PathBuf::from("/nonexistent/settings.json"),
        "Mac Studio",
        &Advanced,
    )
}

/// The advanced part as the app has it once started: its state kept as
/// `Advanced::start` keeps it.
fn keep_author(app: &mut TestApp) -> Arc<AdvancedState> {
    let author = Arc::new(AdvancedState::new(&app.app_dir()));
    app.keep(author.clone());
    author
}

fn with_numbers(app: &mut TestApp) -> (Snapshot, OffsetDateTime) {
    let report = include_str!("../../crates/quota-station-core/tests/fixtures/claude-pool.json");
    app.set_claude(serde_json::from_str(report).ok());
    let now = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
    (app.snapshot(), now)
}

/// The advanced part's badge as the app draws it: while the pause holds
/// the icon has its two bars and says so; Claude used up keeps its own.
#[test]
fn while_the_pause_holds_the_icon_has_the_private_badge() {
    use quota_station_core::advanced::badge;
    use quota_station_core::local_time;
    use quota_station_core::pause;
    let mut app = app();
    let author = keep_author(&mut app);
    let (snapshot, now) = with_numbers(&mut app);
    let plain = app.icon(&snapshot, now);
    assert_eq!(plain.badge, None);
    let later = local_time::minute(now + time::Duration::hours(1));
    author.lock().switcher.pause = Some(pause::Flag { until: Some(later), reason: None });
    let paused = app.icon(&snapshot, now);
    assert_eq!(paused.badge, Some(badge::paused()));
    assert_eq!(paused.label, format!("{}. Tự động chuyển đang tạm dừng", plain.label));
    assert_eq!(paused.title, plain.title);
    assert_eq!(app.icon(&snapshot, now + time::Duration::hours(2)).badge, None, "over");

    let mut used_up = snapshot.clone();
    used_up.claude.pool.exhausted = true;
    assert_eq!(app.icon(&used_up, now).badge, None);
}

/// A settings file with Nâng cao on: the windows get it, with the rules
/// editor and, on a Mac, the update from source. Windows has no other Mac
/// to copy the rules to.
#[test]
fn the_model_carries_the_advanced_part() {
    let dir = std::env::temp_dir().join(format!("qs-state-author-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("app/settings.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, r#"{ "advanced": true, "sourceDir": "/srv/x" }"#).unwrap();
    let mut app = TestApp::new(dir.clone(), file, "Mac A", &Advanced);
    keep_author(&mut app);
    let json = serde_json::to_value(app.model()).unwrap();
    assert_eq!(json["capabilities"]["authorTools"], true);
    assert_eq!(json["advanced"]["on"], true);
    assert!(json.get("rotation").is_some());
    assert_eq!(json["app"].get("sourceStatus").is_some(), cfg!(unix));
    if cfg!(windows) {
        assert_eq!(json["rotation"]["peers"], serde_json::json!([]));
        assert_eq!(json["rotation"]["ssh"], false);
    }
    let own = quota_station_core::advanced::settings::Own::of(app.settings());
    assert!(own.advanced, "the file keeps it");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The advanced part's hold as the app asks it: unsaved rules hold the
/// main window's close, and a quit asks on the rules' tab.
#[test]
fn with_unsaved_rules_the_app_asks_first_and_a_quit_opens_their_tab() {
    let mut app = app();
    let author = keep_author(&mut app);
    assert_eq!(app.on_close(Closing::MainWindow), OnClose::Close);
    assert_eq!(app.on_close(Closing::App), OnClose::Close);
    author.lock().rules_dirty = true;
    assert_eq!(app.on_close(Closing::MainWindow), OnClose::Ask(None));
    assert!(!author.lock().quit_pending);
    assert_eq!(app.on_close(Closing::App), OnClose::Ask(Some(Tab::Ext("rules".into()))));
    assert!(author.lock().quit_pending);
}

/// No key of the advanced part's views is one of the app's own, which
/// would come twice in the JSON the windows get.
#[test]
fn the_private_views_take_no_key_of_the_apps() {
    let plain = TestApp::new(
        PathBuf::new(),
        PathBuf::from("/nonexistent/settings.json"),
        "Mac Studio",
        &NoExtension,
    );
    let apps = serde_json::to_value(plain.model()).unwrap();
    let mut with = app();
    keep_author(&mut with);
    let model = with.model();
    assert_eq!(
        model.ext.keys().map(String::as_str).collect::<Vec<_>>(),
        ["advanced", "rotation", "switcher"]
    );
    for key in model.ext.keys() {
        assert!(apps.get(key).is_none(), "{key} is the app's");
    }
    for key in model.capabilities.ext.keys() {
        assert!(apps["capabilities"].get(key).is_none(), "{key} is the app's");
    }
}

/// A look the advanced part asks for (`AdvancedState::request_check`)
/// reaches the reader through the state it keeps in the app: the reader
/// reads when the extension says so ([`Extension::reader_asked`]).
#[test]
fn a_look_at_the_switcher_asked_through_the_kept_state_wakes_the_reader() {
    let mut kept = Kept::default();
    let author = Arc::new(AdvancedState::default());
    kept.keep(author.clone());
    author.lock().switcher_now = false;
    assert!(!Advanced.reader_asked(&kept));
    author.lock().switcher_now = true;
    assert!(Advanced.reader_asked(&kept));
}

/// The advanced part's keys in the settings file of the golden model's
/// state.
#[cfg(target_os = "macos")]
fn advanced_settings(_home: &std::path::Path, settings: &mut serde_json::Value) {
    settings["advanced"] = true.into();
    settings["sourceDir"] = "/srv/quota-station".into();
    settings["peers"] = serde_json::json!([
        { "name": "Mac B", "alias": "mac-b" },
        { "name": "Mac C", "alias": "mac-c" },
        { "name": "Mac D", "alias": "mac-d" },
    ]);
}

/// The advanced part's state in the golden model's state.
#[cfg(target_os = "macos")]
fn fill_advanced(app: &mut TestApp) {
    use quota_station::advanced::state::DryRunRecord;
    use quota_station_core::pause;
    use quota_station_core::peers::{PeerRecord, PeerState};
    use quota_station_core::rotation::{self, Rules, Stamp};
    use quota_station_core::switcher::{self, DryRun, LogLine};
    let file = rotation::path(app.home());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let stamp = Stamp { at: "2026-01-16T14:30:05+07:00".into(), machine: "Mac A".into() };
    let text = rotation::render(
        &Rules { team: vec![1, 2], personal: Some(3), ..Rules::starter(&[]) },
        &stamp,
        None,
    );
    std::fs::write(&file, &text).unwrap();
    let author = keep_author(app);
    let mut own = author.lock();
    let at = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
    own.switcher.pause = Some(pause::Flag { until: None, reason: Some("lý do".into()) });
    let line = "paused indefinitely: lý do";
    own.switcher.dry_run = Some(DryRunRecord {
        run: DryRun { decision: switcher::parse_decision(line), raw: line.into() },
        at,
        flag_held: true,
    });
    own.switcher.last_log = Some(LogLine {
        at: Some("2026-01-16T07:00:00Z".into()),
        event: switcher::LogEvent::Switch { slot: 2 },
        raw: "2026-01-16 14:00:00 switch to slot 2 (team)".into(),
    });
    own.switcher.service = Some(true);
    own.switcher.legacy = Some("com.cswap.auto".into());
    own.rotation_saving = true;
    let newer = Stamp { at: "2026-01-16T14:39:05+07:00".into(), machine: "Mac B".into() };
    let mut theirs = Rules { team: vec![1, 2], personal: Some(3), ..Rules::starter(&[]) };
    theirs.full = 88.0;
    own.sync.peers.insert(
        "mac-b".into(),
        PeerRecord {
            state: PeerState::Newer { their: newer.clone() },
            against: rotation::version(&text),
            their_text: Some(rotation::render(&theirs, &newer, None)),
        },
    );
    own.sync.peers.insert(
        "mac-c".into(),
        PeerRecord {
            state: PeerState::Synced { at: "2026-01-16T07:00:00Z".into() },
            against: "2026-01-16T13:00:00+07:00".into(),
            their_text: None,
        },
    );
    own.pushing.insert("mac-c".into());
}

/// The model JSON of the app with its advanced part is the same, key for
/// key and value for value, as its golden file
/// (`tests/fixtures/model-advanced*.json`), drawn as the app crate draws
/// its own ([`quota_station::testing::fixed_model_json`]) with the advanced
/// part's settings keys, its keys on the accounts in the rotation or held
/// out of it, and its state. Only on a Mac: the files are a Mac's model
/// (Windows draws its own platform and paths). `UPDATE_MODEL_GOLDEN=1`
/// rewrites the file after a change the windows are meant to see.
#[cfg(target_os = "macos")]
#[test]
fn the_model_json_is_the_golden_one_of_the_advanced_part() {
    let dir = std::env::temp_dir().join(format!("qs-model-golden-advanced-{}", std::process::id()));
    let json = quota_station::testing::fixed_model_json(
        &dir,
        &Advanced,
        advanced_settings,
        |slot| quota_station_core::advanced::held::shown(slot != 3),
        fill_advanced,
    );
    let agy = if cfg!(feature = "antigravity-accounts") { "-antigravity-accounts" } else { "" };
    let golden = format!("tests/fixtures/model-advanced{agy}.json");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(&golden);
    if std::env::var_os("UPDATE_MODEL_GOLDEN").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap() + "\n").unwrap();
        return;
    }
    let text = std::fs::read_to_string(&path).expect("the golden file");
    let expected: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json, expected, "the model JSON differs from {golden}");
}
