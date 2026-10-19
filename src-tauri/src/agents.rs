//! The MCP tab (M9): the table of every agent's MCP servers the windows
//! draw, and the commands its boxes, forms and preview use.
//!
//! The windows never get a secret: the table says which secrets a server
//! has and whether the app can find them; the preview masks them; a key
//! typed or pasted goes from the form straight to the Keychain. Every
//! write goes through `quota_station_core::agents::batch`, after the owner
//! saw its preview.

use quota_station_core::agents::batch::{self, Change, Outcome, Preview, Undo};
use quota_station_core::agents::paste::{self, Paste};
use quota_station_core::agents::secrets::Secrets;
use quota_station_core::agents::{self, claude_mcp, kho, Agent, Places, View};
use quota_station_core::antigravity::fs::AppDir;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner};

/// What the app holds about the MCP tab between reads.
#[derive(Debug, Default)]
pub struct Live {
    pub view: Option<View>,
    /// RFC 3339.
    pub read_at: Option<String>,
    /// `reading` or `writing` while one runs.
    pub busy: Option<&'static str>,
    /// The last write, for the line under the table and its undo.
    pub last: Option<LastWrite>,
}

#[derive(Debug, Clone)]
pub struct LastWrite {
    pub at: String,
    pub outcome: Outcome,
    /// Taken once: an undo is not undone from the same line.
    pub undo: Option<Undo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpView {
    pub table: Option<View>,
    pub read_at: Option<String>,
    pub busy: Option<&'static str>,
    pub last: Option<LastView>,
    pub backups: Vec<BackupView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastView {
    pub at: String,
    pub outcome: Outcome,
    pub can_undo: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupView {
    pub agent: Agent,
    /// When it was taken, RFC 3339.
    pub at: String,
    pub file: String,
    /// `name:action` of the write it came before, when the MCP tab made it.
    pub changes: Vec<String>,
}

impl Inner {
    /// Where each agent keeps its MCP list on this Mac.
    pub fn mcp_places(&self) -> Places {
        Places::new(&self.home, &self.codex_home(), claude_mcp::discover(&self.home))
    }
}

fn rfc(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}

pub fn view(inner: &Inner) -> McpView {
    let live = &inner.mcp;
    let app = inner.app_dir();
    let log = batch::log_read(&app);
    let mut backups = Vec::new();
    for agent in Agent::ALL {
        let (dir, prefix) = agent.backups();
        for path in agents::write::list(&app.path().join(dir), prefix) {
            let Some(at) = std::fs::metadata(&path).and_then(|m| m.modified()).ok() else {
                continue;
            };
            let file =
                path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let changes = log
                .iter()
                .find(|l| l.agent == agent && l.backup.as_deref() == Some(file.as_str()))
                .map(|l| l.changes.clone())
                .unwrap_or_default();
            backups.push(BackupView { agent, at: rfc(OffsetDateTime::from(at)), file, changes });
        }
    }
    backups.sort_by(|a, b| b.at.cmp(&a.at));
    McpView {
        table: live.view.clone(),
        read_at: live.read_at.clone(),
        busy: live.busy,
        last: live.last.as_ref().map(|l| LastView {
            at: l.at.clone(),
            outcome: l.outcome.clone(),
            can_undo: l.undo.is_some(),
        }),
        backups,
    }
}

/// Reads every agent's list again and publishes the table.
pub fn look(app: &AppHandle, state: &AppState) {
    let (places, dir) = {
        let inner = state.lock();
        (inner.mcp_places(), inner.app_dir())
    };
    let table = agents::look(&places, &dir, &Secrets::system());
    {
        let mut inner = state.lock();
        inner.mcp.view = Some(table);
        inner.mcp.read_at = Some(rfc(OffsetDateTime::now_utc()));
    }
    reader::publish(app, state);
}

/// The MCP tab opened, or "Đọc lại".
#[tauri::command]
pub async fn mcp_read(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    {
        let mut inner = shared.lock();
        if inner.mcp.busy.is_some() {
            return Ok(());
        }
        inner.mcp.busy = Some("reading");
    }
    reader::publish(&app, &shared);
    let a = app.clone();
    let s = shared.clone();
    let done = tauri::async_runtime::spawn_blocking(move || look(&a, &s)).await;
    shared.lock().mcp.busy = None;
    reader::publish(&app, &shared);
    done.map_err(|e| e.to_string())
}

/// What `changes` would write, read from the files now.
#[tauri::command]
pub async fn mcp_preview(
    state: State<'_, AppState>,
    changes: Vec<Change>,
) -> Result<Preview, String> {
    let (places, dir) = {
        let inner = state.lock();
        (inner.mcp_places(), inner.app_dir())
    };
    tauri::async_runtime::spawn_blocking(move || {
        batch::preview(&places, &dir, &Secrets::system(), &changes)
    })
    .await
    .map_err(|e| e.to_string())
}

/// Writes the batch the owner saw in the preview: `bases` are the files as
/// the preview read them. Rejects with `busy` while a write or a Codex
/// switch runs.
/// Starts a write to the agents' files: one at a time, never during a
/// Codex switch. The MCP tab's batches and a restore (M10) both go
/// through here.
pub fn begin_write(app: &AppHandle, shared: &AppState) -> Result<(Places, AppDir), String> {
    let out = {
        let mut inner = shared.lock();
        if inner.mcp.busy == Some("writing") || inner.codex.busy.is_some() {
            return Err("busy".into());
        }
        inner.mcp.busy = Some("writing");
        // A Codex look that read the file before this write must not put the
        // old reading back.
        inner.codex.generation += 1;
        (inner.mcp_places(), inner.app_dir())
    };
    reader::publish(app, shared);
    Ok(out)
}

/// Ends a write begun with [`begin_write`]: reads the table again and asks
/// the Codex tab to look.
pub async fn end_write(app: &AppHandle, shared: &AppState) {
    {
        let mut inner = shared.lock();
        inner.mcp.busy = None;
        inner.codex.generation += 1;
    }
    let a = app.clone();
    let s = shared.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &s)).await;
    crate::codex::request(shared, false);
}

async fn run_write(
    app: &AppHandle,
    shared: &AppState,
    changes: Vec<Change>,
    bases: std::collections::BTreeMap<Agent, String>,
) -> Result<Outcome, String> {
    let (places, dir) = begin_write(app, shared)?;
    let done = tauri::async_runtime::spawn_blocking(move || {
        batch::write(&places, &dir, &Secrets::system(), &changes, &bases, OffsetDateTime::now_utc())
    })
    .await;
    if let Ok(out) = &done {
        shared.lock().mcp.last = Some(LastWrite {
            at: rfc(OffsetDateTime::now_utc()),
            outcome: out.clone(),
            undo: out.undo.clone(),
        });
    }
    end_write(app, shared).await;
    if done.as_ref().is_ok_and(|o| !o.done.is_empty()) {
        crate::backup::after_change(app, shared);
    }
    done.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mcp_write(
    app: AppHandle,
    state: State<'_, AppState>,
    changes: Vec<Change>,
    bases: std::collections::BTreeMap<Agent, String>,
) -> Result<Outcome, String> {
    run_write(&app, state.inner(), changes, bases).await
}

/// "Hoàn tác": the last write's batch put back, when the files are still
/// what it wrote.
#[tauri::command]
pub async fn mcp_undo(app: AppHandle, state: State<'_, AppState>) -> Result<Outcome, String> {
    let shared = state.inner().clone();
    let undo = shared.lock().mcp.last.as_mut().and_then(|l| l.undo.take()).ok_or("nothing")?;
    let out = run_write(&app, &shared, undo.changes, undo.bases).await?;
    // The undo's own undo is not offered.
    if let Some(l) = shared.lock().mcp.last.as_mut() {
        l.undo = None;
    }
    reader::publish(&app, &shared);
    Ok(out)
}

/// A key typed in the add form or pasted from Bitwarden: straight to the
/// Keychain, with the Bitwarden item it came from. Never sent back.
#[tauri::command]
pub async fn mcp_save_secret(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    slot: String,
    value: String,
    bitwarden: Option<String>,
) -> Result<(), String> {
    if !agents::valid_name(&name) || value.is_empty() {
        return Err("invalid".into());
    }
    let shared = state.inner().clone();
    let dir = shared.lock().app_dir();
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        Secrets::system().save(&name, &slot, &value).map_err(|e| e.to_string())?;
        if let Some(b) = bitwarden {
            let mut store = kho::load(&dir);
            store.set_hint(&name, &slot, &b);
            kho::save(&dir, &store).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?;
    let a = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &shared)).await;
    done
}

/// "Dán JSON hoặc TOML": the servers in the pasted text, masked. With
/// `save` ("Tiếp: xem trước") their secrets go to the Keychain first, for
/// the add to take from there, with the Bitwarden item they are in.
#[tauri::command]
pub async fn mcp_paste(
    state: State<'_, AppState>,
    text: String,
    save: bool,
    bitwarden: Option<String>,
) -> Result<Paste, String> {
    let dir = state.lock().app_dir();
    tauri::async_runtime::spawn_blocking(move || {
        let secrets = Secrets::system();
        let out = paste::paste(&text, save.then_some(&secrets));
        if let Some(b) = bitwarden.filter(|b| save && !b.trim().is_empty() && out.error.is_none()) {
            let mut store = kho::load(&dir);
            for s in &out.servers {
                for slot in &s.slots {
                    store.set_hint(&s.name, &slot.id, b.trim());
                }
            }
            kho::save(&dir, &store).map_err(|e| e.to_string())?;
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// "Xoá hẳn": the server leaves the app's store, and with `keys` its
/// Keychain items.
#[tauri::command]
pub async fn mcp_forget(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    slots: Vec<String>,
    keys: bool,
) -> Result<(), String> {
    let shared = state.inner().clone();
    let dir = shared.lock().app_dir();
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let mut store = kho::load(&dir);
        let mut all: Vec<String> = slots;
        for k in [store.parked.remove(&name), store.stored.remove(&name)].into_iter().flatten() {
            all.extend(k.slots);
        }
        all.sort();
        all.dedup();
        if keys {
            let secrets = Secrets::system();
            for slot in &all {
                secrets.delete(&name, slot).map_err(|e| e.to_string())?;
                store.set_hint(&name, slot, "");
            }
        }
        kho::save(&dir, &store).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?;
    let a = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || look(&a, &shared)).await;
    done
}

/// The backups' folder, opened in Finder.
#[tauri::command]
pub fn mcp_open_backups(state: State<'_, AppState>, agent: Agent) -> Result<(), String> {
    let dir = state.lock().app_dir().path().join(agent.backups().0);
    if !dir.is_dir() {
        return Err("missing".into());
    }
    std::process::Command::new("/usr/bin/open").arg(&dir).status().map_err(|e| e.to_string())?;
    Ok(())
}
