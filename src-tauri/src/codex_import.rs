// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! "Nhập file đăng nhập (JSON)" (M8): ChatGPT sign-ins read from files the
//! owner picks in the system's file picker, or from JSON pasted in the
//! window, then kept as accounts the way `codex login` keeps them
//! ([`quota_station_core::codex::import`]).
//!
//! The window gets back who each sign-in is and what importing it would
//! do, never a token. The sign-ins wait here in memory, ten minutes at
//! most, until "Nhập" writes them. Nothing is sent to OpenAI and nothing is
//! renewed. The files picked are deleted only when the owner ticks it, and
//! only a file whose every sign-in is now kept.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use quota_station_core::codex::accounts::{Added, Store, CHATGPT_ID};
use quota_station_core::codex::import::{self, Fate, Format, Found, Kind};
use quota_station_core::codex::platform::Unix;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use time::OffsetDateTime;

use crate::codex_accounts::{failed, login_of, plan_name, refused, relist, AccountDone};
use crate::state::AppState;
use crate::windows;

/// The most files one pick reads, and the largest file or paste read.
const FILES_MAX: usize = 50;
const SIZE_MAX: u64 = 1024 * 1024;
/// How long the sign-ins read wait for "Nhập".
const HELD_FOR: Duration = Duration::from_secs(10 * 60);

/// Where sign-ins come from: a file picked, or text pasted (no name, no path).
pub struct Source {
    pub name: Option<String>,
    pub path: Option<PathBuf>,
    pub bytes: Vec<u8>,
}

impl Drop for Source {
    fn drop(&mut self) {
        self.bytes.iter_mut().for_each(|b| *b = 0);
    }
}

/// One sign-in read, and what importing it would do.
struct Item {
    name: Option<String>,
    path: Option<PathBuf>,
    found: Found,
    fate: Fate,
}

/// The sign-ins read, waiting for "Nhập".
pub struct Held {
    token: String,
    made: Instant,
    items: Vec<Item>,
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Held").field("items", &self.items.len()).finish_non_exhaustive()
    }
}

/// One sign-in read, as the window shows it. Never a token.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    /// The file's name; `None` for text pasted.
    pub file: Option<String>,
    /// `codex` or `cliproxy`.
    pub format: Option<&'static str>,
    /// Only while the owner shows names.
    pub email: Option<String>,
    pub plan: Option<String>,
    /// `new`, `renew`, `dup`, `refreshOnly`, `apiKey` or `broken`.
    pub state: &'static str,
    /// `renew`, `dup`: the account here.
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub token: String,
    pub items: Vec<ImportItem>,
}

/// A picked file's bytes, up to [`SIZE_MAX`]; a file that cannot be read
/// reads as nothing, which shows as not a sign-in.
pub fn read_file(path: PathBuf) -> Source {
    let mut bytes = Vec::new();
    if let Ok(f) = std::fs::File::open(&path) {
        let _ = f.take(SIZE_MAX).read_to_end(&mut bytes);
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    Source { name, path: Some(path), bytes }
}

/// Every sign-in in `sources`, and what importing each would do here.
fn read(
    store: &Store,
    sources: &[Source],
    relogin: &dyn Fn(u32) -> bool,
    now: OffsetDateTime,
) -> std::io::Result<Vec<Item>> {
    let accounts = store.list(now)?;
    let mut whence = Vec::new();
    let mut found = Vec::new();
    for s in sources {
        for f in import::read(&s.bytes) {
            whence.push((s.name.clone(), s.path.clone()));
            found.push(f);
        }
    }
    let fates = import::plan(&found, &accounts, relogin);
    Ok(whence
        .into_iter()
        .zip(found)
        .zip(fates)
        .map(|(((name, path), found), fate)| Item { name, path, found, fate })
        .collect())
}

fn item_view(i: &Item, names: bool) -> ImportItem {
    let state = match (i.fate, i.found.kind) {
        (Fate::New, _) => "new",
        (Fate::Renew { .. }, _) => "renew",
        (Fate::Dup { .. }, _) => "dup",
        (Fate::Left, Kind::RefreshOnly) => "refreshOnly",
        (Fate::Left, Kind::ApiKey) => "apiKey",
        (Fate::Left, _) => "broken",
    };
    let id = match i.fate {
        Fate::Renew { number } | Fate::Dup { number: Some(number) } => {
            Some(format!("{CHATGPT_ID}{number}"))
        }
        _ => None,
    };
    let who = i.found.who.as_ref();
    ImportItem {
        file: i.name.clone(),
        format: i.found.format.map(|f| match f {
            Format::Codex => "codex",
            Format::CliProxy => "cliproxy",
        }),
        email: who.and_then(|w| w.email.clone()).filter(|_| names),
        plan: who.and_then(|w| w.plan.as_deref()).map(plan_name),
        state,
        id,
    }
}

/// Imports what `items` plans to: new accounts added, accounts that must
/// sign in again renewed. With `remove_source`, deletes each file picked
/// whose every sign-in is now kept.
fn import_items(
    store: &Store,
    items: &[Item],
    remove_source: bool,
    now: OffsetDateTime,
) -> AccountDone {
    let mut ids = Vec::new();
    let mut kept = vec![false; items.len()];
    let mut error = None;
    for (k, i) in items.iter().enumerate() {
        let again = match i.fate {
            Fate::New => None,
            Fate::Renew { number } => Some(number),
            Fate::Dup { number } => {
                kept[k] = number.is_some();
                continue;
            }
            Fate::Left => continue,
        };
        match store.import(&i.found, again, now) {
            Ok(Added::Added { number } | Added::Renewed { number }) => {
                ids.push(format!("{CHATGPT_ID}{number}"));
                kept[k] = true;
            }
            // Added meanwhile (another import, a sign-in): kept all the same.
            Ok(Added::Dup { .. }) => kept[k] = true,
            Ok(Added::NoLogin) => {}
            Err(e) => error = Some(e.to_string()),
        }
    }
    if ids.is_empty() {
        if let Some(e) = error {
            return failed("other", Some(e));
        }
    }
    let mut paths: Vec<&Path> = items.iter().filter_map(|i| i.path.as_deref()).collect();
    paths.dedup();
    let mut removed = remove_source;
    for path in paths {
        let all_kept = items
            .iter()
            .zip(&kept)
            .filter(|(i, _)| i.path.as_deref() == Some(path))
            .all(|(_, k)| *k);
        let imported_here = items.iter().zip(&kept).any(|(i, k)| {
            *k && i.path.as_deref() == Some(path)
                && matches!(i.fate, Fate::New | Fate::Renew { .. })
        });
        if !imported_here {
            continue;
        }
        if remove_source && all_kept {
            removed &= std::fs::remove_file(path).is_ok();
        } else {
            removed = false;
        }
    }
    let pasted = items.iter().any(|i| i.path.is_none());
    AccountDone::Imported {
        ids,
        removed: removed && !items.iter().all(|i| i.path.is_none()),
        pasted,
    }
}

/// Reads `sources` and keeps them for "Nhập", in place of any read before.
fn preview(shared: &AppState, sources: Vec<Source>) -> Result<ImportPreview, String> {
    let (home, dir, relogin, names) = {
        let inner = shared.lock();
        let relogin: Vec<u32> = inner
            .codex
            .accounts
            .iter()
            .filter(|a| login_of(&inner, a.entry.number, a.place) == "relogin")
            .map(|a| a.entry.number)
            .collect();
        (inner.codex_home(), inner.app_dir(), relogin, !inner.settings.hide_names)
    };
    let platform = Unix::system();
    let store = Store::new(&dir, &home, &platform);
    let items = read(&store, &sources, &|n| relogin.contains(&n), OffsetDateTime::now_utc())
        .map_err(|e| e.to_string())?;
    drop(sources);
    let token = format!("{:x}", OffsetDateTime::now_utc().unix_timestamp_nanos());
    let view = items.iter().map(|i| item_view(i, names)).collect();
    shared.lock().codex.import = Some(Held { token: token.clone(), made: Instant::now(), items });
    Ok(ImportPreview { token, items: view })
}

/// "Chọn file…": the system's file picker (several files at once, JSON),
/// opened in CLIProxyAPI's folder when there is one; then what the files
/// hold. `None` when the owner picked nothing.
#[tauri::command]
pub async fn codex_import_pick(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ImportPreview>, String> {
    let shared = state.inner().clone();
    let user_home = shared.lock().home.clone();
    let picker = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = picker.dialog().file().add_filter("JSON", &["json"]);
        let cli_proxy = user_home.join(".cli-proxy-api");
        if cli_proxy.is_dir() {
            dialog = dialog.set_directory(cli_proxy);
        }
        if let Some(main) = picker.get_webview_window(windows::MAIN) {
            dialog = dialog.set_parent(&main);
        }
        dialog.blocking_pick_files()
    })
    .await
    .map_err(|_| "error".to_string())?;
    let Some(picked) = picked else { return Ok(None) };
    let sources: Vec<Source> = picked
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .take(FILES_MAX)
        .map(read_file)
        .collect();
    let shared2 = shared.clone();
    tauri::async_runtime::spawn_blocking(move || preview(&shared2, sources).map(Some))
        .await
        .map_err(|_| "error".to_string())?
}

/// "Dán JSON": the same reading, of text pasted in the window.
#[tauri::command]
pub async fn codex_import_text(
    state: State<'_, AppState>,
    text: String,
) -> Result<ImportPreview, String> {
    let source = Source { name: None, path: None, bytes: text.into_bytes() };
    if source.bytes.len() as u64 > SIZE_MAX {
        return Err("large".into());
    }
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || preview(&shared, vec![source]))
        .await
        .map_err(|_| "error".to_string())?
}

/// "Nhập": writes the sign-ins read under `token`. Refused with `expired`
/// after ten minutes or another read, with `busy` while Codex switches,
/// signs in or wakes an account.
#[tauri::command]
pub async fn codex_import(
    app: AppHandle,
    state: State<'_, AppState>,
    token: String,
    remove_source: bool,
) -> Result<AccountDone, String> {
    let shared = state.inner().clone();
    let (held, home, dir) = {
        let mut inner = shared.lock();
        if inner.codex.busy.is_some() || inner.codex.signing.is_some() {
            return Ok(refused("busy"));
        }
        let fresh = inner
            .codex
            .import
            .as_ref()
            .is_some_and(|h| h.token == token && h.made.elapsed() < HELD_FOR);
        let held = inner.codex.import.take();
        let Some(held) = held.filter(|_| fresh) else { return Ok(refused("expired")) };
        inner.codex.busy = Some("importing");
        (held, inner.codex_home(), inner.app_dir())
    };
    let done = tauri::async_runtime::spawn_blocking(move || {
        let platform = Unix::system();
        let store = Store::new(&dir, &home, &platform);
        import_items(&store, &held.items, remove_source, OffsetDateTime::now_utc())
    })
    .await
    .unwrap_or_else(|_| failed("other", None));
    shared.lock().codex.busy = None;
    relist(&app, &shared);
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quota_station_core::app_dir::AppDir;
    use quota_station_core::codex::accounts::{self, Place};

    /// A temporary Codex folder and app folder.
    fn mac(name: &str) -> (PathBuf, AppDir) {
        let root =
            std::env::temp_dir().join(format!("qs-codex-import-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join(".codex");
        std::fs::create_dir_all(&home).unwrap();
        (home, AppDir::new(root.join("app")))
    }

    /// A made-up CLIProxyAPI sign-in of workspace `ws-<user>`; its tokens are not real.
    fn cli_proxy(user: &str) -> String {
        serde_json::json!({
            "type": "codex",
            "id_token": "not-real",
            "access_token": "not-real",
            "refresh_token": "not-real",
            "account_id": format!("ws-{user}"),
            "email": format!("{user}@example.com"),
            "last_refresh": "2026-10-21T07:00:00Z"
        })
        .to_string()
    }

    fn picked(dir: &Path, name: &str, text: &str) -> Source {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        read_file(path)
    }

    #[test]
    fn files_picked_are_imported_and_only_the_ones_fully_kept_are_deleted_when_asked() {
        let (home, dir) = mac("files");
        let platform = Unix::default();
        let store = Store::new(&dir, &home, &platform);
        let picks = dir.path().join("picked");
        std::fs::create_dir_all(&picks).unwrap();
        let sources = vec![
            picked(&picks, "codex-ann.json", &cli_proxy("ann")),
            picked(
                &picks,
                "codex-bob.json",
                &format!("[{}, {{\"refresh_token\": \"x\"}}]", cli_proxy("bob")),
            ),
            picked(&picks, "notes.json", "{\"notes\": 1}"),
        ];
        let items = read(&store, &sources, &|_| false, OffsetDateTime::now_utc()).unwrap();
        let view: Vec<_> =
            items.iter().map(|i| (i.name.as_deref().unwrap(), item_view(i, true).state)).collect();
        assert_eq!(
            view,
            [
                ("codex-ann.json", "new"),
                ("codex-bob.json", "new"),
                ("codex-bob.json", "refreshOnly"),
                ("notes.json", "broken")
            ]
        );
        assert_eq!(item_view(&items[0], true).email.as_deref(), Some("ann@example.com"));
        assert_eq!(item_view(&items[0], false).email, None, "names hidden");
        let done = import_items(&store, &items, true, OffsetDateTime::now_utc());
        let AccountDone::Imported { ids, removed, pasted } = done else { panic!("{done:?}") };
        assert_eq!(ids, ["chatgpt:1", "chatgpt:2"]);
        assert!(!pasted);
        // ann's file held only her sign-in: gone. bob's also held a refresh
        // token the app did not take: kept, so `removed` says not all were.
        assert!(!picks.join("codex-ann.json").exists());
        assert!(picks.join("codex-bob.json").exists());
        assert!(picks.join("notes.json").exists());
        assert!(!removed);
        let list = store.list(OffsetDateTime::now_utc()).unwrap();
        assert!(list.iter().all(|a| a.place == Place::Folder));
        assert!(dir.path().join(accounts::DIR).join("2").join("auth.json").is_file());
    }

    #[test]
    fn files_are_kept_unless_asked_and_text_pasted_is_imported_the_same_way() {
        let (home, dir) = mac("paste");
        let platform = Unix::default();
        let store = Store::new(&dir, &home, &platform);
        let picks = dir.path().join("picked");
        std::fs::create_dir_all(&picks).unwrap();
        let sources = vec![picked(&picks, "codex-ann.json", &cli_proxy("ann"))];
        let items = read(&store, &sources, &|_| false, OffsetDateTime::now_utc()).unwrap();
        let done = import_items(&store, &items, false, OffsetDateTime::now_utc());
        assert!(matches!(done, AccountDone::Imported { removed: false, .. }), "{done:?}");
        assert!(picks.join("codex-ann.json").exists(), "kept: the box was not ticked");

        let pasted = vec![Source {
            name: None,
            path: None,
            bytes: format!("{}\n{}", cli_proxy("ann"), cli_proxy("bob")).into_bytes(),
        }];
        let items = read(&store, &pasted, &|_| false, OffsetDateTime::now_utc()).unwrap();
        assert_eq!(
            items.iter().map(|i| item_view(i, true).state).collect::<Vec<_>>(),
            ["dup", "new"]
        );
        assert_eq!(item_view(&items[0], true).id.as_deref(), Some("chatgpt:1"));
        assert_eq!(item_view(&items[0], true).file, None);
        let done = import_items(&store, &items, true, OffsetDateTime::now_utc());
        let AccountDone::Imported { ids, removed, pasted } = done else { panic!("{done:?}") };
        assert_eq!(
            (ids.as_slice(), removed, pasted),
            (["chatgpt:2".to_string()].as_slice(), false, true)
        );
    }

    #[test]
    fn an_account_that_must_sign_in_again_takes_the_sign_in_in_the_file() {
        let (home, dir) = mac("renew");
        let platform = Unix::default();
        let store = Store::new(&dir, &home, &platform);
        let first = vec![Source { name: None, path: None, bytes: cli_proxy("ann").into_bytes() }];
        let items = read(&store, &first, &|_| false, OffsetDateTime::now_utc()).unwrap();
        import_items(&store, &items, false, OffsetDateTime::now_utc());
        let auth = dir.path().join(accounts::DIR).join("1").join("auth.json");
        std::fs::remove_file(&auth).unwrap();
        let again = vec![Source { name: None, path: None, bytes: cli_proxy("ann").into_bytes() }];
        let items = read(&store, &again, &|n| n == 1, OffsetDateTime::now_utc()).unwrap();
        assert_eq!(item_view(&items[0], true).state, "renew");
        let done = import_items(&store, &items, false, OffsetDateTime::now_utc());
        assert!(
            matches!(done, AccountDone::Imported { ref ids, .. } if ids == &["chatgpt:1"]),
            "{done:?}"
        );
        assert!(auth.is_file());
        assert_eq!(store.list(OffsetDateTime::now_utc()).unwrap().len(), 1);
    }
}
