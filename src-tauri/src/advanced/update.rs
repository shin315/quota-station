// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Cập nhật từ mã nguồn", a Mac's only: pulls the clone set in settings
//! and runs its `scripts/install.sh` over the running copy, then opens the
//! new copy when the owner asks. Its row in Cài đặt › Nâng cao is among the
//! private part's keys in the app's own row (`ext::View::App`), drawn by
//! [`view`]; where an update stands is the advanced part's own state
//! ([`Live`], in [`AdvancedState`]), and its commands reach the app only
//! through the host. Each keeps the name, the arguments and the answer it
//! had among the app's own commands.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use quota_station_core::advanced::settings::Own;
use quota_station_core::app_dir::AppDir;
use quota_station_core::runner;
use quota_station_core::settings::Settings;
use quota_station_core::update::{self, Outcome, SourceError, Step, Tools, UpdateError};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::advanced::state::{AdvancedState, Fields};
use crate::ext::{Host, Model, ModelInput, COMMIT};

/// Where an update from source stands, between commands.
#[derive(Debug, Default)]
pub struct Live {
    /// The step running now; `None` when no update runs.
    pub updating: Option<Step>,
    /// How the last update ended, when it did.
    pub outcome: Option<Outcome>,
    pub error: Option<UpdateError>,
}

/// "Mã nguồn" and "Cập nhật từ mã nguồn" in Cài đặt › Nâng cao, as keys of
/// the app's own row (`app`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    /// `~/…`; `None` when not set.
    pub source: Option<String>,
    /// `notSet`, `ok`, `missing` or `notSource`.
    pub source_status: &'static str,
    /// This copy runs from a `.app`, so an update has somewhere to go.
    pub bundled: bool,
    pub updating: Option<Step>,
    pub outcome: Option<Outcome>,
    pub error: Option<UpdateError>,
    /// The last install's output, `~/…`.
    pub log: String,
}

/// The row as it stands now, for the copy running here.
pub fn view(input: &ModelInput<'_>) -> SourceView {
    let author = input.kept.get::<AdvancedState>().unwrap_or_default();
    let own = author.lock();
    source_view(input, &own.update, running_dest().is_some())
}

/// The row from `input`, where the update stands (`live`), and whether this
/// copy runs from a `.app` (`bundled`).
pub fn source_view(input: &ModelInput<'_>, live: &Live, bundled: bool) -> SourceView {
    let source = Own::of(input.settings).source_dir;
    let source = source.as_deref();
    let source_status = match source.map(update::check_source) {
        None => "notSet",
        Some(Ok(())) => "ok",
        Some(Err(e)) => e.code(),
    };
    SourceView {
        source: source.map(|p| runner::display_path(p, input.home)),
        source_status,
        bundled,
        updating: live.updating,
        outcome: live.outcome.clone(),
        error: live.error.clone(),
        log: runner::display_path(&log_file(input.app_dir), input.home),
    }
}

/// The last install's output, in the app's folder.
fn log_file(app_dir: &AppDir) -> PathBuf {
    app_dir.path().join("update/install.log")
}

/// The folder holding the `.app` this copy runs from.
fn running_dest() -> Option<PathBuf> {
    std::env::current_exe().ok().as_deref().and_then(update::install_dest)
}

/// The `.app` this copy runs from.
fn running_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    update::install_dest(&exe)?;
    exe.ancestors().nth(3).map(Path::to_path_buf)
}

/// The clone `path` names (`~` is `home`), or `None` for an empty path;
/// `missing` or `notSource` for a folder that is not one.
fn chosen_source(path: &str, home: &Path) -> Result<Option<PathBuf>, String> {
    if path.trim().is_empty() {
        return Ok(None);
    }
    let p = runner::expand_path(path, home);
    update::check_source(&p).map_err(|e: SourceError| e.code().to_string())?;
    Ok(Some(p))
}

/// An update starts: `busy` while one runs, `noSource` with no clone set in
/// `settings`; else the clone, with the first step shown and the last
/// update's end forgotten.
fn begin(live: &mut Live, settings: &Settings) -> Result<PathBuf, String> {
    if live.updating.is_some() {
        return Err("busy".into());
    }
    let source = Own::of(settings).source_dir.ok_or("noSource")?;
    live.updating = Some(Step::Pulling);
    live.outcome = None;
    live.error = None;
    Ok(source)
}

/// The update ended with `result`.
fn end(live: &mut Live, result: Result<Outcome, UpdateError>) {
    live.updating = None;
    match result {
        Ok(outcome) => live.outcome = Some(outcome),
        Err(error) => live.error = Some(error),
    }
}

/// "Mở lại" may go: the last update installed a new copy (`notInstalled`
/// otherwise) and the rules editor holds no changes (`rulesDirty`).
fn may_relaunch(own: &Fields) -> Result<(), String> {
    if !matches!(own.update.outcome, Some(Outcome::Installed { .. })) {
        return Err("notInstalled".into());
    }
    if own.rules_dirty {
        return Err("rulesDirty".into());
    }
    Ok(())
}

/// The clone to update from; empty forgets it. Rejects with `missing` or
/// `notSource`.
#[tauri::command]
pub fn set_source_dir(app: AppHandle, path: String) -> Result<Model, String> {
    let host = Host::of(&app);
    host.locked(|app| {
        let chosen = chosen_source(&path, app.home())?;
        let mut settings = app.settings().clone();
        Own::change(&mut settings, |own| own.source_dir = chosen);
        app.save_settings(settings)
    })?;
    host.publish();
    Ok(host.model())
}

/// "Cập nhật từ mã nguồn": `git pull --ff-only` in the clone, then its
/// install script into the folder this copy runs from, unless this copy was
/// built from that commit already. The outcome comes in the model. Rejects
/// with `busy`, `noSource` or `unbundled`.
#[tauri::command]
pub async fn update_from_source(
    app: AppHandle,
    own: State<'_, Arc<AdvancedState>>,
) -> Result<Model, String> {
    let host = Host::of(&app);
    let own = own.inner().clone();
    let dest = running_dest().ok_or("unbundled")?;
    let settings = host.settings();
    let log = log_file(&host.app_dir());
    let source = begin(&mut own.lock().update, &settings)?;
    host.publish();
    let (step_host, step_own) = (host.clone(), own.clone());
    let result = tauri::async_runtime::spawn_blocking(move || {
        let on_step = |step: Step| {
            step_own.lock().update.updating = Some(step);
            step_host.publish();
        };
        update::update(&Tools::system(), &source, &dest, COMMIT, Some(&log), &on_step)
    })
    .await
    .unwrap_or_else(|e| Err(UpdateError::Spawn { detail: e.to_string() }));
    end(&mut own.lock().update, result);
    host.publish();
    Ok(host.model())
}

/// "Mở lại" after an update: quits, and a small shell waits for this
/// process to end before opening the new copy (opening it while this one
/// runs would only bring this one forward). Rejects with `notInstalled`,
/// `rulesDirty` (the rules editor holds changes) or `unbundled`.
#[tauri::command]
pub fn relaunch(app: AppHandle, own: State<'_, Arc<AdvancedState>>) -> Result<(), String> {
    may_relaunch(&own.lock())?;
    let bundle = running_bundle().ok_or("unbundled")?;
    let wait = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open "$2""#;
    Command::new("/bin/sh")
        .args(["-c", wait, "sh"])
        .arg(std::process::id().to_string())
        .arg(&bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use quota_station_core::advanced::settings::Own;
    use quota_station_core::app_dir::AppDir;
    use quota_station_core::settings::Settings;
    use quota_station_core::update::{Outcome, SourceError, Step, UpdateError};
    use serde_json::json;

    use super::{begin, chosen_source, end, may_relaunch, source_view, Live};
    use crate::advanced::state::AdvancedState;
    use crate::ext::{Kept, ModelInput};

    fn with_source(dir: Option<&str>) -> Settings {
        let mut settings = Settings::default();
        Own::change(&mut settings, |own| own.source_dir = dir.map(PathBuf::from));
        settings
    }

    fn row(settings: &Settings, live: &Live, bundled: bool) -> serde_json::Value {
        let kept = Kept::default();
        let app_dir = AppDir::new(PathBuf::from("/Users/me/Library/QS"));
        let input = ModelInput {
            kept: &kept,
            settings,
            home: Path::new("/Users/me"),
            app_dir: &app_dir,
            claude_accounts: &[],
        };
        serde_json::to_value(source_view(&input, live, bundled)).unwrap()
    }

    #[test]
    fn the_source_row_shows_the_clone_and_where_an_update_stands() {
        let live = Live {
            updating: Some(Step::Installing),
            outcome: None,
            error: Some(UpdateError::Pull { detail: "diverged".into() }),
        };
        assert_eq!(
            row(&with_source(Some("/Users/me/nowhere/qs")), &live, true),
            json!({
                "source": "~/nowhere/qs",
                "sourceStatus": "missing",
                "bundled": true,
                "updating": "installing",
                "outcome": null,
                "error": { "kind": "pull", "detail": "diverged" },
                "log": "~/Library/QS/update/install.log",
            })
        );
        let done = Live {
            outcome: Some(Outcome::UpToDate { commit: "abc1234".into() }),
            ..Live::default()
        };
        assert_eq!(
            row(&with_source(None), &done, false),
            json!({
                "source": null,
                "sourceStatus": "notSet",
                "bundled": false,
                "updating": null,
                "outcome": { "kind": "upToDate", "commit": "abc1234" },
                "error": null,
                "log": "~/Library/QS/update/install.log",
            })
        );
    }

    #[test]
    fn the_source_row_tells_a_clone_from_a_plain_folder() {
        let dir = std::env::temp_dir().join(format!("qs-source-row-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        let path = dir.to_str().unwrap().to_owned();
        let status =
            |settings: &Settings| row(settings, &Live::default(), false)["sourceStatus"].clone();
        assert_eq!(status(&with_source(Some(&path))), json!("notSource"));
        assert_eq!(chosen_source(&path, Path::new("/")), Err("notSource".into()));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("scripts/install.sh"), "").unwrap();
        assert_eq!(status(&with_source(Some(&path))), json!("ok"));
        assert_eq!(chosen_source(&path, Path::new("/")), Ok(Some(dir.clone())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_path_forgets_the_clone_and_a_missing_one_is_refused() {
        assert_eq!(chosen_source("  ", Path::new("/Users/me")), Ok(None));
        assert_eq!(
            chosen_source("~/nowhere/qs", Path::new("/Users/me/none")),
            Err(SourceError::Missing.code().into())
        );
    }

    #[test]
    fn an_update_starts_once_with_a_clone_set_and_keeps_how_it_ended() {
        let mut live =
            Live { error: Some(UpdateError::Spawn { detail: "x".into() }), ..Live::default() };
        assert_eq!(begin(&mut live, &with_source(None)), Err("noSource".into()));
        assert_eq!(live.updating, None, "nothing starts without a clone");
        assert_eq!(begin(&mut live, &with_source(Some("/srv/qs"))), Ok(PathBuf::from("/srv/qs")));
        assert_eq!(live.updating, Some(Step::Pulling));
        assert_eq!(live.error, None, "the last end is forgotten");
        assert_eq!(begin(&mut live, &with_source(Some("/srv/qs"))), Err("busy".into()));
        let installed = Outcome::Installed { commit: "abc1234".into(), version: None, build: None };
        end(&mut live, Ok(installed.clone()));
        assert_eq!(
            (live.updating, live.outcome.clone(), live.error.clone()),
            (None, Some(installed), None)
        );
        assert!(begin(&mut live, &with_source(Some("/srv/qs"))).is_ok());
        assert_eq!(live.outcome, None);
        end(&mut live, Err(UpdateError::Timeout { step: Step::Pulling }));
        assert_eq!(live.error, Some(UpdateError::Timeout { step: Step::Pulling }));
        assert_eq!(live.outcome, None);
    }

    #[test]
    fn the_app_opens_again_only_after_an_install_with_the_rules_saved() {
        let author = AdvancedState::default();
        assert_eq!(may_relaunch(&author.lock()), Err("notInstalled".into()));
        author.lock().update.outcome = Some(Outcome::UpToDate { commit: "abc1234".into() });
        assert_eq!(may_relaunch(&author.lock()), Err("notInstalled".into()));
        author.lock().update.outcome = Some(Outcome::Installed {
            commit: "abc1234".into(),
            version: Some("1.2.3".into()),
            build: Some("412".into()),
        });
        author.lock().rules_dirty = true;
        assert_eq!(may_relaunch(&author.lock()), Err("rulesDirty".into()));
        author.lock().rules_dirty = false;
        assert_eq!(may_relaunch(&author.lock()), Ok(()));
    }

    #[test]
    fn the_row_reads_where_the_update_stands_from_what_the_extension_keeps() {
        let author = Arc::new(AdvancedState::default());
        author.lock().update.updating = Some(Step::Pulling);
        let mut kept = Kept::default();
        kept.keep(author);
        let settings = with_source(None);
        let app_dir = AppDir::new(PathBuf::from("/Users/me/Library/QS"));
        let input = ModelInput {
            kept: &kept,
            settings: &settings,
            home: Path::new("/Users/me"),
            app_dir: &app_dir,
            claude_accounts: &[],
        };
        assert_eq!(super::view(&input).updating, Some(Step::Pulling));
    }
}
