// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's state for tests outside this crate (the `test-support`
//! feature), made as [`crate::run`] makes it but without a window: an
//! extension's tests draw the model, the icon and the answer to a close
//! through it, and the model JSON of a build with an extension is drawn here
//! as this crate's own golden test draws it.

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use quota_station_core::app_dir::AppDir;
use quota_station_core::claude::state::AccountOut;
use quota_station_core::ext::Fields;
use quota_station_core::quota::Response;
use quota_station_core::settings::Settings;
use quota_station_core::{Icon, Snapshot};
use serde_json::Value;
use time::OffsetDateTime;

use crate::ext::{Closing, Extension, Model};
use crate::state::Inner;
/// What the app does when the owner closes the main window or quits.
pub use crate::state::OnClose;

/// The app's state, with `extension` in it as the binary passes it to
/// [`crate::run`].
pub struct TestApp {
    inner: Inner,
}

impl TestApp {
    /// The state of an app whose home is `home` and whose settings are read
    /// from `settings_file` with `extension`'s hooks, on a machine macOS
    /// names `machine`.
    pub fn new<E: Extension>(
        home: PathBuf,
        settings_file: PathBuf,
        machine: &str,
        extension: &'static E,
    ) -> TestApp {
        let mut inner = Inner::new(home, settings_file, machine.into(), extension.hooks());
        inner.extension = extension;
        TestApp { inner }
    }

    /// Keeps `state` in the app, as the extension keeps its own when it
    /// starts (`Host::keep_state`).
    pub fn keep<T: Any + Send + Sync>(&mut self, state: Arc<T>) {
        self.inner.kept.keep(state);
    }

    pub fn home(&self) -> &Path {
        &self.inner.home
    }

    pub fn settings(&self) -> &Settings {
        &self.inner.settings
    }

    /// The folder the settings file is in.
    pub fn app_dir(&self) -> AppDir {
        self.inner.app_dir()
    }

    /// The Claude report, as the reader stores the last one it read.
    pub fn set_claude(&mut self, report: Option<Response>) {
        self.inner.claude = report;
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.detailed_snapshot()
    }

    pub fn model(&self) -> Model {
        self.inner.model()
    }

    pub fn icon(&self, snapshot: &Snapshot, now: OffsetDateTime) -> Icon {
        self.inner.icon(snapshot, now)
    }

    pub fn on_close(&self, closing: Closing) -> OnClose {
        self.inner.on_close(closing)
    }
}

/// The model JSON the windows get from a state with something in every part
/// of the model, none of it from the clock, the time zone or the machine
/// the test runs on, with `extension` in it. The extension adds its own
/// keys to the settings file (`settings`, handed the home folder), its
/// fields to each Claude account (`account`, by slot) and its own state
/// (`fill`). The state lives in `dir`, which is removed before and after;
/// the JSON writes `dir` as `/TEST` and fixes the things read from this
/// machine outside it: another Codex switcher in /Applications is not
/// there, and `npx` is not found (the file says so, as Claude Code is found:
/// the state sets that one).
pub fn fixed_model_json<E: Extension>(
    dir: &Path,
    extension: &'static E,
    settings: impl FnOnce(&Path, &mut Value),
    account: impl Fn(u32) -> Fields,
    fill: impl FnOnce(&mut TestApp),
) -> Value {
    let _ = std::fs::remove_dir_all(dir);
    let home = dir.join("home");
    let file = dir.join("app/settings.json");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let mut json = serde_json::json!({
        "setupSeen": true,
        "machineName": "Mac phòng khách",
        "theme": "dark",
    });
    settings(&home, &mut json);
    std::fs::write(&file, json.to_string()).unwrap();
    let mut app = TestApp::new(home, file, "Mac Studio", extension);
    let inner = &mut app.inner;
    inner.app_version = "1.2.3".into();
    // Claude Code is found, whatever this machine has on its PATH.
    inner.claude_program = Some(PathBuf::from("/TEST/claude"));
    inner.reading = true;
    inner.reads = 7;
    inner.switching = Some(2);
    inner.claude = serde_json::from_str(include_str!(
        "../../crates/quota-station-core/tests/fixtures/claude-pool.json"
    ))
    .ok();
    let out = |slot: u32, active: bool| AccountOut {
        slot,
        alias: Some(format!("tài khoản {slot}")),
        ext: account(slot),
        active,
        kind: Default::default(),
        status: "fresh",
        fetched_at: Some("2026-01-16T07:00:00Z".into()),
        age_seconds: Some(60),
        windows: quota_station_core::claude::state::Windows {
            five_hour: None,
            seven_day: None,
            fable: None,
        },
        error: None,
        email: None,
    };
    inner.claude_accounts = vec![out(1, false), out(2, true), out(3, false)];
    fill(&mut app);
    let text = serde_json::to_string(&app.inner.model()).unwrap();
    let _ = std::fs::remove_dir_all(dir);
    let folder = serde_json::to_string(&dir.to_string_lossy()).unwrap();
    let folder = folder.trim_matches('"');
    let mut json: Value = serde_json::from_str(&text.replace(folder, "/TEST")).unwrap();
    json["codex"]["ccSwitch"]["installed"] = false.into();
    json["skills"]["npx"] = false.into();
    json
}
