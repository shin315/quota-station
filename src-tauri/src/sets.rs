//! The Bộ cấu hình tab (M15, N): the sets the windows draw, how far the
//! applied one has drifted, and the commands its buttons and dialogs use.
//!
//! A set is applied for the whole Mac through
//! `quota_station_core::agents::sets`, after its preview, under the same
//! guard as the MCP and Skills tabs' writes (one write at a time, never
//! during a Codex switch). Codex's provider is never changed here: the
//! windows open the Codex switch as step 2. Nothing is applied on its own.

use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::sets::{
    self, Applied, ApplyError, Drifted, Outcome, Preview, Removed, Sent, Set, SetError, Undone,
};
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner};

/// What the app holds about the tab between commands.
#[derive(Debug, Default)]
pub struct Live {
    /// `applying` or `undoing` while one runs.
    pub busy: Option<&'static str>,
    /// The set deleted last, for its "Hoàn tác".
    pub removed: Option<Removed>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedView {
    pub id: String,
    pub name: String,
    /// RFC 3339.
    pub at: String,
    /// The entries no longer as the set says; none until the MCP and
    /// Skills tables are read.
    pub drift: Option<Vec<Drifted>>,
    pub can_undo: bool,
    pub codex_before: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetsView {
    pub sets: Vec<Set>,
    pub applied: Option<AppliedView>,
    pub busy: Option<&'static str>,
    /// The name of the set deleted last, while it can be put back.
    pub removed: Option<String>,
}

fn rfc(secs: i64) -> String {
    OffsetDateTime::from_unix_timestamp(secs)
        .ok()
        .and_then(|t| t.format(&Rfc3339).ok())
        .unwrap_or_default()
}

fn applied_view(inner: &Inner, kept: &sets::Sets, a: &Applied) -> AppliedView {
    let set = kept.get(&a.id);
    let drift = match (set, &inner.mcp.view, &inner.skills.table) {
        (Some(s), Some(mcp), Some(sk)) => Some(sets::drift(s, mcp, sk)),
        _ => None,
    };
    AppliedView {
        id: a.id.clone(),
        name: set.map(|s| s.name.clone()).unwrap_or_else(|| a.name.clone()),
        at: rfc(a.at),
        drift,
        can_undo: a.can_undo(),
        codex_before: a.codex_before.clone(),
    }
}

pub fn view(inner: &Inner) -> SetsView {
    let kept = sets::load(&inner.app_dir());
    SetsView {
        applied: kept.applied.as_ref().map(|a| applied_view(inner, &kept, a)),
        sets: kept.sets.clone(),
        busy: inner.sets.busy,
        removed: inner.sets.removed.as_ref().map(|r| r.set.name.clone()),
    }
}

fn set_error(e: SetError) -> String {
    match e {
        SetError::Name => "name".into(),
        SetError::Taken => "taken".into(),
        SetError::BadEntry(n) => format!("badEntry:{n}"),
        SetError::Full => "full".into(),
        SetError::Missing => "missing".into(),
        SetError::Io(_) => "io".into(),
    }
}

/// "Lưu bộ": a new set, or one edited. Never touches an agent's file.
#[tauri::command]
pub fn sets_save(app: AppHandle, state: State<'_, AppState>, set: Set) -> Result<Set, String> {
    let dir = state.lock().app_dir();
    let saved = sets::put(&dir, &set, OffsetDateTime::now_utc()).map_err(set_error)?;
    state.lock().sets.removed = None;
    reader::publish(&app, state.inner());
    Ok(saved)
}

/// "Xoá": the set goes; the agents' files stay as they are.
#[tauri::command]
pub fn sets_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let dir = state.lock().app_dir();
    let removed = sets::delete(&dir, &id).map_err(set_error)?;
    state.lock().sets.removed = Some(removed);
    reader::publish(&app, state.inner());
    Ok(())
}

/// "Hoàn tác" of a delete.
#[tauri::command]
pub fn sets_undelete(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let (dir, removed) = {
        let mut inner = state.lock();
        (inner.app_dir(), inner.sets.removed.take().ok_or("nothing")?)
    };
    sets::undelete(&dir, &removed).map_err(set_error)?;
    reader::publish(&app, state.inner());
    Ok(())
}

/// "Áp…": what the set would write, read from the files now.
#[tauri::command]
pub async fn sets_preview(state: State<'_, AppState>, id: String) -> Result<Preview, String> {
    let (places, skill_places, dir) = {
        let inner = state.lock();
        (inner.mcp_places(), inner.skill_places(), inner.app_dir())
    };
    let set = sets::load(&dir).get(&id).cloned().ok_or("missing")?;
    tauri::async_runtime::spawn_blocking(move || {
        sets::preview(&places, &skill_places, &dir, &Secrets::system(), &set)
    })
    .await
    .map_err(|e| e.to_string())
}

fn apply_error(e: ApplyError) -> String {
    match e {
        ApplyError::Missing => "missing".into(),
        ApplyError::Changed(a) => format!(
            "changed:{}",
            serde_json::to_value(a)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        ),
        ApplyError::NotTheSet => "notTheSet".into(),
    }
}

/// Starts a set's write: under the MCP and Skills tabs' guard.
fn begin(app: &AppHandle, shared: &AppState, what: &'static str) -> Result<(), String> {
    if shared.lock().sets.busy.is_some() {
        return Err("busy".into());
    }
    crate::skills::begin(app, shared, "writing")?;
    shared.lock().sets.busy = Some(what);
    reader::publish(app, shared);
    Ok(())
}

async fn end(app: &AppHandle, shared: &AppState, changed: bool) {
    shared.lock().sets.busy = None;
    crate::skills::end(app, shared, changed).await;
}

/// "Áp bộ": writes what the preview showed, when every file is still what
/// it read. Rejects with `busy` while another write or a Codex switch runs.
#[tauri::command]
pub async fn sets_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    sent: Sent,
) -> Result<Outcome, String> {
    let shared = state.inner().clone();
    let codex_now = crate::codex::view(&shared.lock()).current;
    begin(&app, &shared, "applying")?;
    let (places, skill_places, dir) = {
        let inner = shared.lock();
        (inner.mcp_places(), inner.skill_places(), inner.app_dir())
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        sets::apply(
            &places,
            &skill_places,
            &dir,
            &Secrets::system(),
            &id,
            &sent,
            codex_now,
            OffsetDateTime::now_utc(),
        )
    })
    .await;
    let changed = matches!(&done, Ok(Ok(o)) if o.mcp.as_ref().is_some_and(|m| !m.done.is_empty()) || o.skills.as_ref().is_some_and(|s| !s.done.is_empty()));
    end(&app, &shared, changed).await;
    done.map_err(|e| e.to_string())?.map_err(apply_error)
}

/// "Hoàn tác bộ": every entry the last apply changed, put back.
#[tauri::command]
pub async fn sets_undo(app: AppHandle, state: State<'_, AppState>) -> Result<Undone, String> {
    let shared = state.inner().clone();
    begin(&app, &shared, "undoing")?;
    let (places, skill_places, dir) = {
        let inner = shared.lock();
        (inner.mcp_places(), inner.skill_places(), inner.app_dir())
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        sets::undo(&places, &skill_places, &dir, &Secrets::system(), OffsetDateTime::now_utc())
    })
    .await;
    let changed = matches!(&done, Ok(Ok(u)) if u.mcp.is_some() || u.skills.is_some());
    end(&app, &shared, changed).await;
    done.map_err(|e| e.to_string())?.map_err(|_| "nothing".to_string())
}
