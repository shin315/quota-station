// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Sao lưu tab (M10 and M12, C): the backup packs in iCloud Drive (or
//! the folder chosen in Cài đặt),
//! "Sao lưu bây giờ", the packs made on their own, the keys the app keeps,
//! and the Khôi phục sheet's steps for the MCP servers, skills, plugins and
//! instruction files.
//!
//! Packs hold no secret (`quota_station_core::agents::pack`). A restore
//! writes the agents' files through the same guard as the MCP tab's and the
//! Skills tab's batches, so one write runs at a time and never during a
//! Codex switch. A key pasted in the sheet goes from the window to the
//! Keychain and is never sent back.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use quota_station_core::agents::pack::{self, Info, LoadError, PackError, Summary};
use quota_station_core::agents::restore::{
    self, Changed, Done, More, PartResult, Pasted, Pick, Plan, RunError, UndoError, Undone,
};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::{self, kho, Agent, Places};
use quota_station_core::app_dir::AppDir;
use quota_station_core::settings::AutoBackup;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::open;
use crate::reader;
use crate::state::{AppState, Inner};

/// How often the daily pack looks whether the agents changed.
const DAILY: Duration = Duration::from_secs(24 * 3600);

#[derive(Debug, Default)]
pub struct Live {
    pub packs: Option<Vec<Info>>,
    pub keys: Option<Vec<KeyView>>,
    /// RFC 3339.
    pub read_at: Option<String>,
    /// `packing` or `restoring` while one runs.
    pub busy: Option<&'static str>,
    /// The last restore, while it can be undone.
    pub restore: Option<restore::Record>,
    /// Why the last pack made on its own was not written.
    pub auto_error: Option<PackError>,
    pub daily_at: Option<Instant>,
}

/// A key the app keeps in its Keychain items: its MCP and name, never the value.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyView {
    pub mcp: String,
    pub slot: String,
    pub label: String,
    pub bitwarden: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreView {
    pub at: i64,
    pub pack: String,
    pub mac: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupView {
    /// iCloud Drive is on on this Mac (on Windows: OneDrive or Documents is
    /// there, which holds the packs by default).
    pub icloud: bool,
    /// Where the packs go, `~/…`.
    pub folder: String,
    /// The owner chose that folder; otherwise it is iCloud Drive's.
    pub custom_folder: bool,
    pub packs: Option<Vec<Info>>,
    pub keys: Option<Vec<KeyView>>,
    pub read_at: Option<String>,
    pub busy: Option<&'static str>,
    pub auto: AutoBackup,
    pub restore: Option<RestoreView>,
    pub auto_error: Option<PackError>,
}

impl Inner {
    /// Where the packs are: the folder chosen in settings, else
    /// `iCloud Drive › Quota Station Agents` (on Windows, in OneDrive or
    /// Documents).
    pub fn pack_root(&self) -> PathBuf {
        self.settings.backup_dir.clone().unwrap_or_else(|| pack::default_root(&self.home))
    }
}

pub fn view(inner: &Inner) -> BackupView {
    let live = &inner.backup;
    BackupView {
        icloud: pack::cloud_drive(&inner.home).is_dir(),
        folder: quota_station_core::runner::display_path(&inner.pack_root(), &inner.home),
        custom_folder: inner.settings.backup_dir.is_some(),
        packs: live.packs.clone(),
        keys: live.keys.clone(),
        read_at: live.read_at.clone(),
        busy: live.busy,
        auto: inner.settings.auto_backup,
        restore: live.restore.as_ref().map(|r| RestoreView {
            at: r.at,
            pack: r.pack.clone(),
            mac: r.mac.clone(),
        }),
        auto_error: live.auto_error.clone(),
    }
}

fn rfc(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}

/// The keys the app keeps, found by every name it knows a secret by: the
/// table's servers, its store, the Bitwarden hints. Reads no value.
fn keys(places: &Places, dir: &AppDir, secrets: &Secrets) -> Vec<KeyView> {
    let view = agents::look(places, dir, secrets);
    let store = kho::load(dir);
    let mut found: BTreeMap<(String, String), String> = BTreeMap::new();
    for r in &view.rows {
        for s in &r.secrets {
            found.entry((r.name.clone(), s.id.clone())).or_insert_with(|| s.label.clone());
        }
    }
    let kept = store.parked.iter().chain(&store.stored).chain(&store.waiting);
    for (name, k) in kept {
        for slot in &k.slots {
            found
                .entry((name.clone(), slot.clone()))
                .or_insert_with(|| pack::slot_label(Some(&k.def), slot));
        }
    }
    for h in &store.hints {
        found
            .entry((h.mcp.clone(), h.slot.clone()))
            .or_insert_with(|| pack::slot_label(None, &h.slot));
    }
    found
        .into_iter()
        .filter(|((mcp, slot), _)| secrets.has(mcp, slot).unwrap_or(false))
        .map(|((mcp, slot), label)| KeyView {
            bitwarden: store.hint(&mcp, &slot).map(str::to_string),
            mcp,
            slot,
            label,
        })
        .collect()
}

/// Lists the packs and the keys again and publishes them.
fn look(app: &AppHandle, state: &AppState) {
    let (places, dir, root, mac) = {
        let inner = state.lock();
        (inner.mcp_places(), inner.app_dir(), inner.pack_root(), inner.machine())
    };
    let packs = pack::list(&root, &mac);
    let keys = keys(&places, &dir, &Secrets::system());
    let record = restore::last(&dir);
    {
        let mut inner = state.lock();
        inner.backup.packs = Some(packs);
        inner.backup.keys = Some(keys);
        inner.backup.restore = record;
        inner.backup.read_at = Some(rfc(OffsetDateTime::now_utc()));
    }
    reader::publish(app, state);
}

async fn look_async(app: &AppHandle, state: &AppState) {
    let a = app.clone();
    let s = state.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &s)).await;
}

/// The Sao lưu tab opened, or "Kiểm lại".
#[tauri::command]
pub async fn backup_read(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    look_async(&app, state.inner()).await;
    Ok(())
}

/// Builds a pack of the agents' servers now and writes it, unless `skip`
/// says the newest pack of this Mac already holds the same.
fn make(inner_of: &AppState, skip_same: bool) -> Result<Option<String>, PackError> {
    let (places, dir, root, mac, version, home) = {
        let inner = inner_of.lock();
        (
            inner.mcp_places(),
            inner.app_dir(),
            inner.pack_root(),
            inner.machine(),
            inner.app_version.clone(),
            inner.home.clone(),
        )
    };
    let readings = agents::read_all(&places);
    let store = kho::load(&dir);
    let mut p = pack::content(&home, &readings, &store, &mac, &version, OffsetDateTime::now_utc());
    let extras = crate::skills::pack_extras(inner_of);
    p.extend(Some(extras.skills), Some(extras.plugins), Some(extras.instructions));
    p.folders = extras.folders;
    if skip_same && pack::same_as_newest(&root, &mac, &p) {
        return Ok(None);
    }
    pack::write(&root, &home, &p, &readings).map(Some)
}

fn code(e: &PackError) -> String {
    match e {
        PackError::NoICloud => "noICloud".into(),
        PackError::Secret => "secret".into(),
        PackError::Io(d) => format!("io:{d}"),
    }
}

/// "Sao lưu bây giờ": the new pack's name.
#[tauri::command]
pub async fn backup_now(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let shared = state.inner().clone();
    {
        let mut inner = shared.lock();
        if inner.backup.busy.is_some() {
            return Err("busy".into());
        }
        inner.backup.busy = Some("packing");
    }
    reader::publish(&app, &shared);
    let s = shared.clone();
    let done = tauri::async_runtime::spawn_blocking(move || make(&s, false))
        .await
        .map_err(|e| e.to_string())?;
    {
        let mut inner = shared.lock();
        inner.backup.busy = None;
        if done.is_ok() {
            inner.backup.auto_error = None;
        }
    }
    look_async(&app, &shared).await;
    done.map(Option::unwrap_or_default).map_err(|e| code(&e))
}

/// A pack made on its own, in the background, when the newest one differs.
fn auto(app: &AppHandle, state: &AppState) {
    {
        let mut inner = state.lock();
        if inner.backup.busy.is_some() {
            return;
        }
        inner.backup.busy = Some("packing");
    }
    let a = app.clone();
    let s = state.clone();
    tauri::async_runtime::spawn(async move {
        let s2 = s.clone();
        let done = tauri::async_runtime::spawn_blocking(move || make(&s2, true)).await;
        {
            let mut inner = s.lock();
            inner.backup.busy = None;
            match &done {
                Ok(Ok(_)) => inner.backup.auto_error = None,
                Ok(Err(e)) => inner.backup.auto_error = Some(e.clone()),
                Err(_) => {}
            }
        }
        look_async(&a, &s).await;
    });
}

/// After the app wrote to an agent: a pack, when "Sau mỗi thay đổi trong
/// app" is on.
pub fn after_change(app: &AppHandle, state: &AppState) {
    if state.lock().settings.auto_backup.after_change {
        auto(app, state);
    }
}

/// After each read: once a day, a pack when an agent's list changed, with
/// "Mỗi ngày" on.
pub fn daily(app: &AppHandle, state: &AppState) {
    {
        let mut inner = state.lock();
        if !inner.settings.auto_backup.daily
            || inner.backup.daily_at.is_some_and(|t| t.elapsed() < DAILY)
        {
            return;
        }
        inner.backup.daily_at = Some(Instant::now());
    }
    auto(app, state);
}

#[tauri::command]
pub fn backup_set_auto(
    app: AppHandle,
    state: State<'_, AppState>,
    after_change: bool,
    daily: bool,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let mut settings = inner.settings.clone();
        settings.auto_backup = AutoBackup { after_change, daily };
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
    }
    reader::publish(&app, &state);
    Ok(())
}

/// "Thư mục sao lưu": a folder that exists, where the packs go from now on;
/// empty goes back to iCloud Drive. The packs already made stay where they are.
#[tauri::command]
pub fn backup_set_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    {
        let mut inner = state.lock();
        let chosen = if path.trim().is_empty() {
            None
        } else {
            let p = quota_station_core::runner::expand_path(&path, &inner.home);
            if !p.is_absolute() || !p.is_dir() {
                return Err("notFound".into());
            }
            Some(p)
        };
        let mut settings = inner.settings.clone();
        settings.backup_dir = chosen;
        settings.save(&inner.settings_file).map_err(|e| e.to_string())?;
        inner.settings = settings;
        // The list was of the other folder.
        inner.backup.packs = None;
    }
    reader::publish(&app, &state);
    Ok(())
}

/// "Mở trong Finder": the packs' folder, or iCloud Drive (on Windows,
/// OneDrive or Documents) before the first.
#[tauri::command]
pub fn backup_open_folder(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (root, home) = {
        let inner = state.lock();
        (inner.pack_root(), inner.home.clone())
    };
    let dir = if root.is_dir() { root } else { pack::cloud_drive(&home) };
    if !dir.is_dir() {
        return Err("missing".into());
    }
    open::folder(&app, &dir)
}

fn load_code(e: &LoadError) -> String {
    match e {
        LoadError::BadName => "badName".into(),
        LoadError::Missing => "missing".into(),
        LoadError::Cloud => "cloud".into(),
        LoadError::Newer => "newer".into(),
        LoadError::Invalid(d) => format!("invalid:{d}"),
    }
}

fn load(state: &AppState, id: &str) -> Result<pack::Pack, String> {
    let root = state.lock().pack_root();
    pack::load(&root, id).map_err(|e| load_code(&e))
}

/// "Xem trong gói".
#[tauri::command]
pub async fn pack_summary(state: State<'_, AppState>, id: String) -> Result<Summary, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || load(&shared, &id).map(|p| pack::summary(&id, &p)))
        .await
        .map_err(|e| e.to_string())?
}

/// Khôi phục, step 2: what the pack would do here. Writes nothing.
#[tauri::command]
pub async fn restore_plan(state: State<'_, AppState>, id: String) -> Result<Plan, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let p = load(&shared, &id)?;
        let (places, dir, tools) = {
            let inner = shared.lock();
            (inner.mcp_places(), inner.app_dir(), inner.restore_tools())
        };
        Ok(restore::plan(&places, &dir, &Secrets::system(), &tools, &id, &p))
    })
    .await
    .map_err(|e| e.to_string())?
}

fn agents_code(agents: &[Agent]) -> String {
    agents
        .iter()
        .map(|a| match a {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Agy => "agy",
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The words a changed file or skill is said in: the agents' codes, then
/// `file:<~/…>` and `skill:<name>`, one per line.
fn changed_code(c: &Changed) -> String {
    let mut out = vec![agents_code(&c.agents)];
    out.extend(c.files.iter().map(|f| format!("file:{f}")));
    out.extend(c.skills.iter().map(|s| format!("skill:{s}")));
    format!("changed:{}", out.join("\n"))
}

/// Starts a restore's writes: the MCP guard and the Skills tab's.
fn begin_restore(app: &AppHandle, shared: &AppState) -> Result<(Places, AppDir), String> {
    crate::skills::begin_restore(shared)?;
    crate::agents::begin_write(app, shared).inspect_err(|_| {
        shared.lock().skills.busy = None;
    })
}

async fn end_restore(app: &AppHandle, shared: &AppState) {
    crate::agents::end_write(app, shared).await;
    crate::skills::end_restore(app, shared).await;
    look_async(app, shared).await;
}

/// Khôi phục, step 4: the servers ticked, the keys pasted, the files as the
/// preview read them, and the skills, plugins and instruction files ticked.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn restore_run(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    picks: Vec<Pick>,
    more: Option<More>,
    keys: Vec<Pasted>,
    bases: BTreeMap<Agent, String>,
) -> Result<Done, String> {
    let more = more.unwrap_or_default();
    let shared = state.inner().clone();
    let p = {
        let s = shared.clone();
        let id = id.clone();
        tauri::async_runtime::spawn_blocking(move || load(&s, &id))
            .await
            .map_err(|e| e.to_string())??
    };
    let (places, dir) = begin_restore(&app, &shared)?;
    let tools = {
        let mut inner = shared.lock();
        inner.backup.busy = Some("restoring");
        inner.restore_tools()
    };
    reader::publish(&app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        restore::run(
            &places,
            &dir,
            &Secrets::system(),
            &tools,
            &id,
            &p,
            &picks,
            &more,
            &keys,
            &bases,
            OffsetDateTime::now_utc(),
        )
    })
    .await
    .map_err(|e| e.to_string());
    shared.lock().backup.busy = None;
    end_restore(&app, &shared).await;
    let done = done?.map_err(|e| match e {
        RunError::Changed(a) => format!("changed:{}", agents_code(&a)),
        RunError::Keychain(_) => "keychain".to_string(),
    })?;
    if !done.written.is_empty() || done.can_undo {
        after_change(&app, &shared);
    }
    Ok(done)
}

/// "Thử lại" on one skill, plugin or instruction file of the last restore.
#[tauri::command]
pub async fn restore_retry(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    part: String,
    item: String,
    agent: Option<Agent>,
) -> Result<PartResult, String> {
    let shared = state.inner().clone();
    let p = {
        let s = shared.clone();
        let id = id.clone();
        tauri::async_runtime::spawn_blocking(move || load(&s, &id))
            .await
            .map_err(|e| e.to_string())??
    };
    let (places, dir) = begin_restore(&app, &shared)?;
    let tools = {
        let mut inner = shared.lock();
        inner.backup.busy = Some("restoring");
        inner.restore_tools()
    };
    reader::publish(&app, &shared);
    let done = tauri::async_runtime::spawn_blocking(move || {
        restore::retry(
            &places,
            &dir,
            &tools,
            &id,
            &p,
            &part,
            &item,
            agent,
            OffsetDateTime::now_utc(),
        )
    })
    .await
    .map_err(|e| e.to_string());
    shared.lock().backup.busy = None;
    end_restore(&app, &shared).await;
    let done = done?;
    if done.ok {
        after_change(&app, &shared);
    }
    Ok(done)
}

/// "Hoàn tác lần khôi phục"; `force` once the owner said yes to files
/// changed since.
#[tauri::command]
pub async fn restore_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    force: bool,
) -> Result<Undone, String> {
    let shared = state.inner().clone();
    let (places, dir) = begin_restore(&app, &shared)?;
    let tools = shared.lock().restore_tools();
    let done = tauri::async_runtime::spawn_blocking(move || {
        restore::undo(&places, &dir, &Secrets::system(), &tools, force, OffsetDateTime::now_utc())
    })
    .await
    .map_err(|e| e.to_string());
    end_restore(&app, &shared).await;
    done?.map_err(|e| match e {
        UndoError::Nothing => "nothing".to_string(),
        UndoError::Changed(c) => changed_code(&c),
        UndoError::Failed(_) => "failed".to_string(),
    })
}

/// "Xoá khoá": the Keychain item and its Bitwarden hint go. The agents'
/// files keep their own copy.
#[tauri::command]
pub async fn backup_delete_key(
    app: AppHandle,
    state: State<'_, AppState>,
    mcp: String,
    slot: String,
) -> Result<(), String> {
    let shared = state.inner().clone();
    let dir = shared.lock().app_dir();
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        Secrets::system().delete(&mcp, &slot).map_err(|e| e.to_string())?;
        let mut store = kho::load(&dir);
        store.set_hint(&mcp, &slot, "");
        kho::save(&dir, &store).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?;
    let a = app.clone();
    let s = shared.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || crate::agents::look(&a, &s)).await;
    look_async(&app, &shared).await;
    done
}
