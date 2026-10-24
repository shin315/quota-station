// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Skills, Plugin and File hướng dẫn tabs (M11): what the windows draw,
//! and the commands their boxes, dialogs and buttons use.
//!
//! The boxes are written through `quota_station_core::agents::skills`, one
//! batch after its preview, under the same guard as the MCP tab's writes
//! (one write at a time, never during a Codex switch). Installing, updating
//! and removing for good go through `npx skills`. The plugins and the
//! instruction files are only read. Once a day the app asks GitHub whether
//! the skills from GitHub have newer versions; it never updates on its own.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use quota_station_core::agents::instructions::{self, Choices, Shown};
use quota_station_core::agents::npx::{self, Listed, Npx, NpxError};
use quota_station_core::agents::pack::{self, PluginList};
use quota_station_core::agents::plugins::{self, Side};
use quota_station_core::agents::restore;
use quota_station_core::agents::skill_ops::{self, Installed, Removed, Updated};
use quota_station_core::agents::skill_updates::{self, GitHub, Updates};
use quota_station_core::agents::skills::{
    self, Change, Gathered, Outcome, Preview, SkillPlaces, Undo, View,
};
use quota_station_core::agents::{self, batch, claude_mcp, Agent};
use quota_station_core::alerts;
use quota_station_core::app_dir::AppDir;
use quota_station_core::codex::procs;
use quota_station_core::runner;
use quota_station_core::settings::SkillChecks;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner};

/// Newer versions are looked for this often.
const DAILY: i64 = 24 * 3600;
/// A check that failed is tried again after this long.
const RETRY: Duration = Duration::from_secs(3600);

/// What the app holds about the three tabs between reads.
#[derive(Debug, Default)]
pub struct Live {
    pub table: Option<View>,
    /// RFC 3339.
    pub read_at: Option<String>,
    /// `reading`, `writing`, `installing`, `updating`, `removing`,
    /// `importing` while one runs.
    pub busy: Option<&'static str>,
    /// While several skills update: the one at hand.
    pub progress: Option<Progress>,
    pub checking: bool,
    pub check_tried: Option<Instant>,
    pub last: Option<Last>,
    pub plugins: Option<Vec<Side>>,
    pub plugins_at: Option<String>,
    pub plugins_busy: bool,
    pub instr: Option<instructions::View>,
    pub instr_at: Option<String>,
}

/// "Cập nhật tất cả" or "Cập nhật cả bộ" under way: `done` of `total`
/// updated, `name` now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub name: String,
}

/// How "Cập nhật tất cả" or "Cập nhật cả bộ" ended: each skill, and the
/// folder the copies are in.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedMany {
    pub results: Vec<Updated>,
    pub backups: String,
}

/// The last thing written, for the line under the table and its undo.
#[derive(Debug, Clone)]
pub struct Last {
    pub at: String,
    /// `batch`, `install`, `import`, `update` or `remove`.
    pub kind: &'static str,
    pub names: Vec<String>,
    pub outcome: Option<Outcome>,
    pub undo: Option<UndoOf>,
}

#[derive(Debug, Clone)]
pub enum UndoOf {
    Batch(Undo),
    Install(Vec<String>),
    Import(Gathered),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastView {
    pub at: String,
    pub kind: &'static str,
    pub names: Vec<String>,
    pub outcome: Option<Outcome>,
    pub can_undo: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatesView {
    /// RFC 3339.
    pub checked_at: Option<String>,
    pub error: Option<String>,
    pub failed: Vec<String>,
    pub checking: bool,
    pub settings: SkillChecks,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillsView {
    pub table: Option<View>,
    pub read_at: Option<String>,
    pub busy: Option<&'static str>,
    pub progress: Option<Progress>,
    /// `npx` is on this Mac: installing and updating work.
    pub npx: bool,
    pub last: Option<LastView>,
    pub updates: UpdatesView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginsView {
    pub sides: Option<Vec<Side>>,
    pub read_at: Option<String>,
    pub busy: bool,
    /// What a restore would run, by agent.
    pub reinstall: BTreeMap<Agent, Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstrView {
    pub view: Option<instructions::View>,
    pub read_at: Option<String>,
}

fn rfc(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}

fn rfc_secs(at: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(at).ok().map(rfc)
}

impl Inner {
    pub fn skill_places(&self) -> SkillPlaces {
        SkillPlaces::new(&self.home, &self.codex_home(), skills::discover_npx(&self.home))
    }

    fn npx(&self, places: &SkillPlaces) -> Option<Npx> {
        let program = places.npx.clone()?;
        Some(Npx::new(program, npx::run_dir(self.app_dir().path())))
    }

    /// What a restore uses besides the MCP files: the skills' folders,
    /// `npx`, the agents' own commands for plugins.
    pub fn restore_tools(&self) -> restore::Tools {
        let places = self.skill_places();
        restore::Tools {
            npx: self.npx(&places),
            claude: claude_mcp::discover(&self.home),
            codex: procs::discover(&self.home),
            codex_home: self.codex_home(),
            root: self.pack_root(),
            skills: places,
        }
    }
}

/// A restore writes skills too: the Skills tab's jobs wait for it, and it
/// waits for theirs.
pub fn begin_restore(state: &AppState) -> Result<(), String> {
    let mut inner = state.lock();
    if inner.skills.busy.is_some() {
        return Err("busy".into());
    }
    inner.skills.busy = Some("writing");
    Ok(())
}

/// After a restore: the Skills, Plugin and File hướng dẫn tabs read again.
pub async fn end_restore(app: &AppHandle, state: &AppState) {
    state.lock().skills.busy = None;
    look_async(app, state, true).await;
    let (a, s) = (app.clone(), state.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || instr_look(&a, &s)).await;
}

pub fn skills_view(inner: &Inner) -> SkillsView {
    let live = &inner.skills;
    let u = skill_updates::load(&inner.app_dir());
    SkillsView {
        table: live.table.clone(),
        read_at: live.read_at.clone(),
        busy: live.busy,
        progress: live.progress.clone(),
        npx: skills::discover_npx(&inner.home).is_some(),
        last: live.last.as_ref().map(|l| LastView {
            at: l.at.clone(),
            kind: l.kind,
            names: l.names.clone(),
            outcome: l.outcome.clone(),
            can_undo: l.undo.is_some(),
        }),
        updates: UpdatesView {
            checked_at: u.checked_at.and_then(rfc_secs),
            error: u.error,
            failed: u.failed,
            checking: live.checking,
            settings: inner.settings.skill_checks,
        },
    }
}

pub fn plugins_view(inner: &Inner) -> PluginsView {
    let live = &inner.skills;
    let reinstall = live
        .plugins
        .iter()
        .flatten()
        .filter(|s| s.ok())
        .map(|s| (s.agent, plugins::reinstall(s)))
        .collect();
    PluginsView {
        sides: live.plugins.clone(),
        read_at: live.plugins_at.clone(),
        busy: live.plugins_busy,
        reinstall,
    }
}

pub fn instr_view(inner: &Inner) -> InstrView {
    InstrView { view: inner.skills.instr.clone(), read_at: inner.skills.instr_at.clone() }
}

// ===== Reading =====

fn read_plugins(home: &std::path::Path) -> Vec<Side> {
    let claude = claude_mcp::discover(home);
    let codex = procs::discover(home);
    vec![plugins::read_claude(claude.as_deref(), home), plugins::read_codex(codex.as_deref(), home)]
}

/// Reads the skills (and the plugins, for the skills they bring) and
/// publishes the table.
fn look(app: &AppHandle, state: &AppState, with_plugins: bool) {
    let (places, dir, home) = {
        let inner = state.lock();
        (inner.skill_places(), inner.app_dir(), inner.home.clone())
    };
    let sides = if with_plugins {
        let s = read_plugins(&home);
        let mut inner = state.lock();
        inner.skills.plugins = Some(s.clone());
        inner.skills.plugins_at = Some(rfc(OffsetDateTime::now_utc()));
        s
    } else {
        state.lock().skills.plugins.clone().unwrap_or_default()
    };
    let look = skills::read(&places);
    let table =
        skills::view(&places, &look, plugins::owned_skills(&sides), &skill_updates::load(&dir));
    {
        let mut inner = state.lock();
        inner.skills.table = Some(table);
        inner.skills.read_at = Some(rfc(OffsetDateTime::now_utc()));
    }
    reader::publish(app, state);
}

async fn look_async(app: &AppHandle, state: &AppState, with_plugins: bool) {
    let a = app.clone();
    let s = state.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &s, with_plugins)).await;
}

/// The Skills tab opened, or "Đọc lại".
#[tauri::command]
pub async fn skills_read(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    {
        let mut inner = shared.lock();
        if inner.skills.busy.is_some() {
            return Ok(());
        }
        inner.skills.busy = Some("reading");
    }
    reader::publish(&app, &shared);
    let with_plugins = shared.lock().skills.plugins.is_none();
    look_async(&app, &shared, with_plugins).await;
    shared.lock().skills.busy = None;
    reader::publish(&app, &shared);
    Ok(())
}

/// The Plugin tab opened, or "Đọc lại".
#[tauri::command]
pub async fn plugins_read(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    {
        let mut inner = shared.lock();
        if inner.skills.plugins_busy {
            return Ok(());
        }
        inner.skills.plugins_busy = true;
    }
    reader::publish(&app, &shared);
    let home = shared.lock().home.clone();
    let sides = tauri::async_runtime::spawn_blocking(move || read_plugins(&home)).await;
    {
        let mut inner = shared.lock();
        inner.skills.plugins_busy = false;
        if let Ok(s) = sides {
            inner.skills.plugins = Some(s);
            inner.skills.plugins_at = Some(rfc(OffsetDateTime::now_utc()));
        }
    }
    reader::publish(&app, &shared);
    Ok(())
}

// ===== The boxes =====

#[tauri::command]
pub async fn skills_preview(
    state: State<'_, AppState>,
    changes: Vec<Change>,
) -> Result<Preview, String> {
    let places = state.lock().skill_places();
    tauri::async_runtime::spawn_blocking(move || skills::preview(&places, &changes))
        .await
        .map_err(|e| e.to_string())
}

/// Starts a write: one at a time with the MCP tab's and a restore's, never
/// during a Codex switch or another skills job.
pub(crate) fn begin(
    app: &AppHandle,
    shared: &AppState,
    what: &'static str,
) -> Result<(SkillPlaces, AppDir), String> {
    {
        let inner = shared.lock();
        if inner.skills.busy.is_some() {
            return Err("busy".into());
        }
    }
    let (_, dir) = agents_begin(app, shared)?;
    let places = {
        let mut inner = shared.lock();
        inner.skills.busy = Some(what);
        inner.skill_places()
    };
    reader::publish(app, shared);
    Ok((places, dir))
}

fn agents_begin(app: &AppHandle, shared: &AppState) -> Result<(agents::Places, AppDir), String> {
    crate::agents::begin_write(app, shared)
}

pub(crate) async fn end(app: &AppHandle, shared: &AppState, changed: bool) {
    shared.lock().skills.busy = None;
    crate::agents::end_write(app, shared).await;
    look_async(app, shared, false).await;
    if changed {
        crate::backup::after_change(app, shared);
    }
}

fn remember(
    shared: &AppState,
    kind: &'static str,
    names: Vec<String>,
    outcome: Option<Outcome>,
    undo: Option<UndoOf>,
) {
    shared.lock().skills.last =
        Some(Last { at: rfc(OffsetDateTime::now_utc()), kind, names, outcome, undo });
}

async fn run_batch(
    app: &AppHandle,
    shared: &AppState,
    changes: Vec<Change>,
    bases: BTreeMap<Agent, String>,
) -> Result<Outcome, String> {
    let (places, dir) = begin(app, shared, "writing")?;
    let done = tauri::async_runtime::spawn_blocking(move || {
        skills::write(&places, &dir, &changes, &bases, OffsetDateTime::now_utc())
    })
    .await;
    let changed = done.as_ref().is_ok_and(|o| !o.done.is_empty());
    if let Ok(out) = &done {
        let names =
            out.undo.iter().flat_map(|u| u.changes.iter().map(|c| c.name.clone())).collect();
        remember(shared, "batch", names, Some(out.clone()), out.undo.clone().map(UndoOf::Batch));
    }
    end(app, shared, changed).await;
    done.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn skills_write(
    app: AppHandle,
    state: State<'_, AppState>,
    changes: Vec<Change>,
    bases: BTreeMap<Agent, String>,
) -> Result<Outcome, String> {
    run_batch(&app, state.inner(), changes, bases).await
}

/// "Hoàn tác" of the last thing written: a batch put back, an install
/// removed, gathered skills put where they were.
#[tauri::command]
pub async fn skills_undo(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let shared = state.inner().clone();
    let undo = shared.lock().skills.last.as_mut().and_then(|l| l.undo.take()).ok_or("nothing")?;
    reader::publish(&app, &shared);
    let result = match undo {
        UndoOf::Batch(u) => run_batch(&app, &shared, u.changes, u.bases).await.map(|o| {
            if o.failed.is_some() {
                "failed".to_string()
            } else {
                "undone".to_string()
            }
        }),
        UndoOf::Install(names) => {
            let (places, dir) = begin(&app, &shared, "removing")?;
            let n = shared.lock().npx(&places);
            let done = tauri::async_runtime::spawn_blocking(move || match n {
                Some(n) => skill_ops::remove(&places, &dir, &n, &names, OffsetDateTime::now_utc()),
                None => {
                    Removed { names: Vec::new(), backups: Vec::new(), error: Some(NpxError::NoNpx) }
                }
            })
            .await
            .map_err(|e| e.to_string());
            end(&app, &shared, true).await;
            done.map(|r| if r.error.is_some() { "failed".into() } else { "undone".into() })
        }
        UndoOf::Import(g) => {
            let (places, dir) = begin(&app, &shared, "importing")?;
            let done = tauri::async_runtime::spawn_blocking(move || {
                skills::unimport(&places, &dir, &g.moved, OffsetDateTime::now_utc())
            })
            .await
            .map_err(|e| e.to_string())?;
            end(&app, &shared, true).await;
            done.map(|_| "undone".to_string())
        }
    };
    if let Some(l) = shared.lock().skills.last.as_mut() {
        l.undo = None;
    }
    reader::publish(&app, &shared);
    result
}

/// "Bỏ liên kết": a link to nothing in an agent's folder.
#[tauri::command]
pub async fn skills_drop_link(
    app: AppHandle,
    state: State<'_, AppState>,
    agent: Agent,
    name: String,
) -> Result<(), String> {
    let shared = state.inner().clone();
    let (places, _) = begin(&app, &shared, "writing")?;
    let done =
        tauri::async_runtime::spawn_blocking(move || skills::drop_broken(&places, agent, &name))
            .await
            .map_err(|e| e.to_string());
    end(&app, &shared, false).await;
    done?.map_err(|e| e.to_string())
}

// ===== npx skills =====

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSkill {
    pub name: String,
    pub description: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoList {
    pub repo: String,
    pub skills: Vec<RepoSkill>,
}

fn npx_code(e: &NpxError) -> String {
    match e {
        NpxError::NoNpx => "noNpx".into(),
        NpxError::BadRepo => "badRepo".into(),
        NpxError::NoSkills => "noSkills".into(),
        NpxError::Timeout => "timeout".into(),
        NpxError::Failed(d) => format!("failed:{d}"),
    }
}

/// "Tìm skill": the skills in a GitHub repository, those installed marked.
#[tauri::command]
pub async fn skills_list_repo(
    state: State<'_, AppState>,
    repo: String,
) -> Result<RepoList, String> {
    let repo = npx::repo(&repo).ok_or("badRepo")?;
    let (places, n) = {
        let inner = state.lock();
        let p = inner.skill_places();
        let n = inner.npx(&p);
        (p, n)
    };
    let n = n.ok_or("noNpx")?;
    tauri::async_runtime::spawn_blocking(move || {
        let listed: Vec<Listed> = n.list(&repo).map_err(|e| npx_code(&e))?;
        let look = skills::read(&places);
        Ok(RepoList {
            skills: listed
                .into_iter()
                .map(|l| RepoSkill {
                    installed: look.shared.contains(&l.name),
                    name: l.name,
                    description: l.description,
                })
                .collect(),
            repo,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// "Đọc SKILL.md" before installing.
#[tauri::command]
pub async fn skills_read_md(
    state: State<'_, AppState>,
    repo: String,
    skill: String,
) -> Result<String, String> {
    let repo = npx::repo(&repo).ok_or("badRepo")?;
    let n = {
        let inner = state.lock();
        let p = inner.skill_places();
        inner.npx(&p)
    }
    .ok_or("noNpx")?;
    tauri::async_runtime::spawn_blocking(move || n.read(&repo, &skill).map_err(|e| npx_code(&e)))
        .await
        .map_err(|e| e.to_string())?
}

/// "Cài": installs, then sets each agent's box as asked.
#[tauri::command]
pub async fn skills_install(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: String,
    names: Vec<String>,
    agents: BTreeMap<Agent, bool>,
) -> Result<Installed, String> {
    let repo = npx::repo(&repo).ok_or("badRepo")?;
    let shared = state.inner().clone();
    let (places, dir) = begin(&app, &shared, "installing")?;
    let n = shared.lock().npx(&places);
    let Some(n) = n else {
        end(&app, &shared, false).await;
        return Err("noNpx".into());
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        skill_ops::install(&places, &dir, &n, &repo, &names, &agents, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    if let Ok(out) = &done {
        if !out.names.is_empty() {
            remember(
                &shared,
                "install",
                out.names.clone(),
                Some(out.outcome.clone()),
                Some(UndoOf::Install(out.names.clone())),
            );
        }
    }
    let changed = done.as_ref().is_ok_and(|o| !o.names.is_empty());
    end(&app, &shared, changed).await;
    done
}

/// "Cập nhật": a copy first, then `npx skills update`; the boxes stay.
#[tauri::command]
pub async fn skills_update(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Updated, String> {
    let shared = state.inner().clone();
    let (places, dir) = begin(&app, &shared, "updating")?;
    let n = shared.lock().npx(&places);
    let Some(n) = n else {
        end(&app, &shared, false).await;
        return Err("noNpx".into());
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        skill_ops::update(&places, &dir, &n, &name, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    if let Ok(u) = &done {
        if u.error.is_none() {
            remember(&shared, "update", vec![u.name.clone()], None, None);
        }
    }
    end(&app, &shared, done.as_ref().is_ok_and(|u| u.error.is_none())).await;
    done
}

/// "Cập nhật tất cả" and "Cập nhật cả bộ": a copy of each first, then
/// `npx skills update` one at a time, the one at hand in the model; the
/// boxes stay.
#[tauri::command]
pub async fn skills_update_many(
    app: AppHandle,
    state: State<'_, AppState>,
    names: Vec<String>,
) -> Result<UpdatedMany, String> {
    let shared = state.inner().clone();
    let (places, dir) = begin(&app, &shared, "updating")?;
    let (n, backups) = {
        let inner = shared.lock();
        let backups = runner::display_path(&dir.path().join(skills::BACKUP_DIR), &inner.home);
        (inner.npx(&places), backups)
    };
    let Some(n) = n else {
        end(&app, &shared, false).await;
        return Err("noNpx".into());
    };
    let (a, s) = (app.clone(), shared.clone());
    let done = tauri::async_runtime::spawn_blocking(move || {
        skill_ops::update_many(
            &places,
            &dir,
            &n,
            &names,
            OffsetDateTime::now_utc(),
            |i, total, name| {
                s.lock().skills.progress =
                    Some(Progress { done: i, total, name: name.to_string() });
                reader::publish(&a, &s);
            },
        )
    })
    .await
    .map_err(|e| e.to_string());
    shared.lock().skills.progress = None;
    let updated: Vec<String> =
        done.iter().flatten().filter(|u| u.error.is_none()).map(|u| u.name.clone()).collect();
    if !updated.is_empty() {
        remember(&shared, "update", updated.clone(), None, None);
    }
    end(&app, &shared, !updated.is_empty()).await;
    done.map(|results| UpdatedMany { results, backups })
}

/// "Bỏ qua bản này".
#[tauri::command]
pub fn skills_skip(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<(), String> {
    let dir = state.lock().app_dir();
    let mut u = skill_updates::load(&dir);
    let to = u.found.get(&name).map(|f| f.to.clone()).ok_or("nothing")?;
    u.skipped.insert(name, to);
    skill_updates::save(&dir, &u).map_err(|e| e.to_string())?;
    let a = app.clone();
    let s = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || look(&a, &s, false));
    Ok(())
}

/// "Gỡ hẳn": a copy first, then `npx skills remove`.
#[tauri::command]
pub async fn skills_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Removed, String> {
    let shared = state.inner().clone();
    let (places, dir) = begin(&app, &shared, "removing")?;
    let n = shared.lock().npx(&places);
    let Some(n) = n else {
        end(&app, &shared, false).await;
        return Err("noNpx".into());
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        skill_ops::remove(&places, &dir, &n, &[name], OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    if let Ok(r) = &done {
        if !r.names.is_empty() {
            remember(&shared, "remove", r.names.clone(), None, None);
        }
    }
    end(&app, &shared, done.as_ref().is_ok_and(|r| !r.names.is_empty())).await;
    done
}

/// "Gom": skills installed by hand moved into the shared folder.
#[tauri::command]
pub async fn skills_import(
    app: AppHandle,
    state: State<'_, AppState>,
    names: Vec<String>,
) -> Result<Gathered, String> {
    let shared = state.inner().clone();
    let (places, dir) = begin(&app, &shared, "importing")?;
    let done = tauri::async_runtime::spawn_blocking(move || {
        skills::import(&places, &dir, &names, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    if let Ok(g) = &done {
        if !g.moved.is_empty() {
            let names = g.moved.iter().map(|m| m.name.clone()).collect();
            remember(&shared, "import", names, None, Some(UndoOf::Import(g.clone())));
        }
    }
    end(&app, &shared, done.as_ref().is_ok_and(|g| !g.moved.is_empty())).await;
    done
}

/// A skill's history on GitHub, in the browser.
#[tauri::command]
pub fn skills_open_link(url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") || url.contains(char::is_whitespace) {
        return Err("invalid".into());
    }
    std::process::Command::new("/usr/bin/open").arg(&url).status().map_err(|e| e.to_string())?;
    Ok(())
}

// ===== Newer versions (G) =====

/// Asks GitHub once per repository, keeps what it found, and raises the
/// alert for versions not seen before.
fn check(app: &AppHandle, state: &AppState) {
    let (places, dir) = {
        let inner = state.lock();
        (inner.skill_places(), inner.app_dir())
    };
    let before = skill_updates::load(&dir);
    let lock = skills::read_lock(&places.lock).unwrap_or_default();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let after = skill_updates::check(&GitHub::api(), &lock, &places.shared, &before, now);
    let _ = skill_updates::save(&dir, &after);
    let found: Vec<(String, String)> = after
        .names(&lock)
        .into_iter()
        .filter_map(|n| after.found.get(&n).map(|f| (n, f.to.clone())))
        .collect();
    {
        let mut inner = state.lock();
        if let Some(id) = alerts::skill_alert(&mut inner.alerts, &found, now) {
            if inner.settings.skill_checks.notify {
                if let Some(center) = inner.center.clone() {
                    alerts::deliver(&mut inner.alerts, &[id], &*center);
                }
            }
            let _ = alerts::save(&dir, &inner.alerts);
        }
    }
    look(app, state, false);
}

fn start_check(app: &AppHandle, state: &AppState) -> bool {
    {
        let mut inner = state.lock();
        if inner.skills.checking {
            return false;
        }
        inner.skills.checking = true;
        inner.skills.check_tried = Some(Instant::now());
    }
    reader::publish(app, state);
    true
}

fn end_check(app: &AppHandle, state: &AppState) {
    state.lock().skills.checking = false;
    reader::publish(app, state);
}

/// "Kiểm bản mới".
#[tauri::command]
pub async fn skills_check(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    if !start_check(&app, &shared) {
        return Ok(());
    }
    let (a, s) = (app.clone(), shared.clone());
    let _ = tauri::async_runtime::spawn_blocking(move || check(&a, &s)).await;
    end_check(&app, &shared);
    Ok(())
}

/// After each read: once a day, with "Kiểm bản mới" on "Mỗi ngày".
pub fn daily(app: &AppHandle, state: &AppState) {
    {
        let inner = state.lock();
        if !inner.settings.skill_checks.daily
            || inner.skills.checking
            || inner.skills.check_tried.is_some_and(|t| t.elapsed() < RETRY)
        {
            return;
        }
        let u: Updates = skill_updates::load(&inner.app_dir());
        let now = OffsetDateTime::now_utc().unix_timestamp();
        if u.checked_at.is_some_and(|at| now - at < DAILY && u.error.is_none()) {
            return;
        }
        if skills::read_lock(&inner.skill_places().lock).map(|l| l.is_empty()).unwrap_or(true) {
            return;
        }
    }
    if !start_check(app, state) {
        return;
    }
    let (a, s) = (app.clone(), state.clone());
    tauri::async_runtime::spawn(async move {
        let (a2, s2) = (a.clone(), s.clone());
        let _ = tauri::async_runtime::spawn_blocking(move || check(&a2, &s2)).await;
        end_check(&a, &s);
    });
}

/// Cài đặt › Skills: "Kiểm bản mới" and "Báo bằng thông báo macOS".
#[tauri::command]
pub fn skills_set_checks(
    app: AppHandle,
    state: State<'_, AppState>,
    daily: bool,
    notify: bool,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.skill_checks = SkillChecks { daily, notify };
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(())
}

// ===== Instruction files (F) =====

/// The MCP secrets now in the agents' files: an instruction file holding
/// one is treated as holding a key.
fn known(inner_places: &agents::Places) -> Vec<String> {
    pack::secret_values(&agents::read_all(inner_places))
}

fn instr_look(app: &AppHandle, state: &AppState) {
    let (home, codex_home, dir, places, root, mac) = {
        let inner = state.lock();
        (
            inner.home.clone(),
            inner.codex_home(),
            inner.app_dir(),
            inner.mcp_places(),
            inner.pack_root(),
            inner.machine(),
        )
    };
    let choices = instructions::load(&dir);
    let read = instructions::read_all(&home, &codex_home, &choices, &known(&places));
    let newest = pack::newest(&root, &mac);
    let files: Option<(i64, Vec<instructions::Packed>)> =
        newest.map(|(at, p)| (at, p.instructions.map(|i| i.files).unwrap_or_default()));
    let view =
        instructions::view(&read, &choices, files.as_ref().map(|(at, f)| (*at, f.as_slice())));
    {
        let mut inner = state.lock();
        inner.skills.instr = Some(view);
        inner.skills.instr_at = Some(rfc(OffsetDateTime::now_utc()));
    }
    reader::publish(app, state);
}

#[tauri::command]
pub async fn instr_read(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (a, s) = (app.clone(), state.inner().clone());
    tauri::async_runtime::spawn_blocking(move || instr_look(&a, &s))
        .await
        .map_err(|e| e.to_string())
}

fn find(
    state: &AppState,
    path: &str,
) -> Result<(instructions::Read1, Vec<String>, Choices), String> {
    let (home, codex_home, dir, places) = {
        let inner = state.lock();
        (inner.home.clone(), inner.codex_home(), inner.app_dir(), inner.mcp_places())
    };
    let choices = instructions::load(&dir);
    let known = known(&places);
    let read = instructions::read_all(&home, &codex_home, &choices, &known);
    let r = read.into_iter().find(|r| r.shown == path).ok_or("missing")?;
    Ok((r, known, choices))
}

/// "Xem": the file, keys masked.
#[tauri::command]
pub async fn instr_show(state: State<'_, AppState>, path: String) -> Result<Shown, String> {
    let s = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (r, known, choices) = find(&s, &path)?;
        instructions::show(&r, &known, &choices.not_keys)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// "So sánh": the file against the newest pack's copy.
#[tauri::command]
pub async fn instr_compare(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<batch::Line>, String> {
    let s = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (r, known, _) = find(&s, &path)?;
        let (root, mac) = {
            let inner = s.lock();
            (inner.pack_root(), inner.machine())
        };
        let (_, p) = pack::newest(&root, &mac).ok_or("noPack")?;
        let files = p.instructions.map(|i| i.files).unwrap_or_default();
        let packed = files.iter().find(|f| f.path == path).ok_or("notInPack")?;
        instructions::compare(&r, packed, &known)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn change_choices(
    app: &AppHandle,
    state: &AppState,
    f: impl FnOnce(&mut Choices) -> Result<(), String>,
) -> Result<(), String> {
    let dir = state.lock().app_dir();
    let mut c = instructions::load(&dir);
    f(&mut c)?;
    instructions::save(&dir, &c).map_err(|e| e.to_string())?;
    let (a, s) = (app.clone(), state.clone());
    tauri::async_runtime::spawn_blocking(move || instr_look(&a, &s));
    Ok(())
}

/// "Trong gói": the file in the pack or not.
#[tauri::command]
pub fn instr_include(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    on: bool,
) -> Result<(), String> {
    change_choices(&app, state.inner(), |c| {
        c.excluded.retain(|p| p != &path);
        if !on {
            c.excluded.push(path);
        }
        Ok(())
    })
}

/// "Không phải khoá" for line `line` (from 1) of the file.
#[tauri::command]
pub async fn instr_not_key(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    line: usize,
) -> Result<(), String> {
    let s = state.inner().clone();
    let (r, _, _) = tauri::async_runtime::spawn_blocking({
        let s = s.clone();
        move || find(&s, &path)
    })
    .await
    .map_err(|e| e.to_string())??;
    change_choices(&app, &s, |c| {
        if instructions::mark_not_key(c, &r, line) {
            Ok(())
        } else {
            Err("missing".into())
        }
    })
}

/// "Thêm file…": a file of the home folder, by its path.
#[tauri::command]
pub fn instr_add(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let (home, dir) = {
        let inner = state.lock();
        (inner.home.clone(), inner.app_dir())
    };
    let shown = instructions::check_extra(&home, dir.path(), &path).map_err(str::to_string)?;
    let s2 = shown.clone();
    change_choices(&app, state.inner(), move |c| {
        if !c.extra.contains(&s2) {
            c.extra.push(s2);
        }
        Ok(())
    })?;
    Ok(shown)
}

/// A file added with "Thêm file…" leaves the list.
#[tauri::command]
pub fn instr_forget(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    change_choices(&app, state.inner(), |c| {
        c.extra.retain(|p| p != &path);
        c.excluded.retain(|p| p != &path);
        Ok(())
    })
}

/// "Mở file": in the default text editor. The app does not edit it.
#[tauri::command]
pub fn instr_open(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let (r, _, _) = find(state.inner(), &path)?;
    std::process::Command::new("/usr/bin/open")
        .arg("-t")
        .arg(&r.path)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ===== The backup pack =====

/// The skills, plugins and instruction files for a pack. An agent whose
/// plugins could not be read keeps the newest pack's list.
pub fn pack_extras(state: &AppState) -> PackExtras {
    let (places, home, codex_home, dir, mcp, root, mac, cached) = {
        let inner = state.lock();
        (
            inner.skill_places(),
            inner.home.clone(),
            inner.codex_home(),
            inner.app_dir(),
            inner.mcp_places(),
            inner.pack_root(),
            inner.machine(),
            inner.skills.plugins.clone(),
        )
    };
    let known = known(&mcp);
    let look = skills::read(&places);
    let (skill_list, folders) = skills::packed(&places, &look, &known);
    let sides = match cached {
        Some(s) if s.iter().all(Side::ok) => s,
        _ => read_plugins(&home),
    };
    let newest = pack::newest(&root, &mac).map(|(_, p)| p);
    let agents = sides
        .into_iter()
        .map(|s| {
            if s.ok() {
                return s;
            }
            let kept = newest
                .as_ref()
                .and_then(|p| p.plugins.as_ref())
                .and_then(|l| {
                    l.agents
                        .iter()
                        .find(|o| o.agent == s.agent && (o.state == "ok" || o.state == "kept"))
                })
                .cloned();
            match kept {
                Some(mut k) => {
                    k.state = "kept".into();
                    k
                }
                None => s,
            }
        })
        .collect();
    let choices = instructions::load(&dir);
    let read = instructions::read_all(&home, &codex_home, &choices, &known);
    let files = instructions::packed(&read, &choices);
    PackExtras {
        skills: skill_list,
        folders,
        plugins: PluginList { agents },
        instructions: pack::InstructionList { files },
    }
}

/// What [`pack_extras`] gives a pack: its sections after `mcp`, and the
/// folders of the skills installed by hand that it copies.
pub struct PackExtras {
    pub skills: pack::SkillList,
    pub folders: Vec<(String, std::path::PathBuf)>,
    pub plugins: PluginList,
    pub instructions: pack::InstructionList,
}
