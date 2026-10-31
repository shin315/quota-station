// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The Thống kê tab (M16, J): reads Claude Code's and Codex's session logs
//! on this Mac into the app's index (`quota_station_core::stats`), off the
//! UI thread, with each agent's progress in the model, and answers the
//! tab's report for a period.
//!
//! The logs are read when the tab opens and when "Đọc lại" is pressed,
//! never on a timer. Only the counts are kept; nothing is sent anywhere.

use std::sync::Arc;
use std::time::{Duration, Instant};

use quota_station_core::history;
use quota_station_core::local_time;
use quota_station_core::stats::index::{self, Index, Progress, Roots};
use quota_station_core::stats::report::{self, Period, Report, Timeline};
#[cfg(feature = "author-tools")]
use quota_station_core::switcher;
use serde::Serialize;
use tauri::{AppHandle, State};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::reader;
use crate::state::{AppState, Inner};

/// The windows hear about a read's progress at most this often.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// What the app holds about the tab between commands.
#[derive(Debug, Default)]
pub struct Live {
    pub reading: bool,
    /// The read running is the first one: no log was read before.
    pub first: bool,
    /// Logs read of the logs found, per agent, while a read runs.
    pub progress: Vec<Progress>,
    /// Reads finished (and "Đã biết" pressed), so the tab asks again.
    pub generation: u64,
    /// The last read could not save the index.
    pub failed: bool,
    /// The index as last read or saved; loaded from disk when needed.
    pub index: Option<Arc<Index>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    pub reading: bool,
    pub first: bool,
    pub progress: Vec<Progress>,
    pub generation: u64,
    pub failed: bool,
    /// RFC 3339; none before the first read.
    pub read_at: Option<String>,
}

fn rfc(secs: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(secs).ok()?.format(&Rfc3339).ok()
}

pub fn view(inner: &Inner) -> StatsView {
    let live = &inner.stats;
    StatsView {
        reading: live.reading,
        first: live.first,
        progress: live.progress.clone(),
        generation: live.generation,
        failed: live.failed,
        read_at: live.index.as_ref().and_then(|i| i.read_at).and_then(rfc),
    }
}

/// The index in memory, else from the app's folder.
fn index_of(shared: &AppState) -> Arc<Index> {
    let dir = {
        let inner = shared.lock();
        if let Some(i) = &inner.stats.index {
            return i.clone();
        }
        inner.app_dir()
    };
    let loaded = Arc::new(index::load(&dir));
    shared.lock().stats.index.get_or_insert(loaded).clone()
}

/// "Đọc lại", and the tab opening: reads what the logs gained since the
/// last read. A read already running is left to finish.
#[tauri::command]
pub fn stats_read(app: AppHandle, state: State<'_, AppState>) {
    let shared = state.inner().clone();
    let (dir, roots) = {
        let mut inner = shared.lock();
        if inner.stats.reading {
            return;
        }
        inner.stats.reading = true;
        inner.stats.progress.clear();
        (inner.app_dir(), Roots::new(&inner.home, &inner.codex_home()))
    };
    reader::publish(&app, &shared);
    std::thread::Builder::new()
        .name("stats-reader".into())
        .spawn(move || {
            let mut ix = (*index_of(&shared)).clone();
            shared.lock().stats.first = ix.logs.is_empty();
            let mut last = Instant::now();
            ix.update(&roots, OffsetDateTime::now_utc().unix_timestamp(), &mut |p| {
                {
                    let mut inner = shared.lock();
                    let live = &mut inner.stats.progress;
                    match live.iter_mut().find(|q| q.agent == p.agent) {
                        Some(q) => *q = p,
                        None => live.push(p),
                    }
                }
                if last.elapsed() >= PROGRESS_EVERY || p.done == 0 {
                    last = Instant::now();
                    reader::publish(&app, &shared);
                }
            });
            let saved = index::save(&dir, &ix);
            {
                let mut inner = shared.lock();
                let live = &mut inner.stats;
                live.reading = false;
                live.first = false;
                live.progress.clear();
                live.failed = saved.is_err();
                live.generation += 1;
                live.index = Some(Arc::new(ix));
            }
            reader::publish(&app, &shared);
        })
        .expect("the stats reader starts");
}

/// The tab's numbers for a period, from the index (no log is opened).
#[tauri::command]
pub async fn stats_report(state: State<'_, AppState>, period: Period) -> Result<Report, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ix = index_of(&shared);
        let (dir, home, now_slot) = {
            let inner = shared.lock();
            (inner.app_dir(), inner.home.clone(), inner.seen.claude)
        };
        #[cfg(feature = "author-tools")]
        let entries = history::collect(&dir, &switcher::log_file(&home), usize::MAX);
        #[cfg(not(feature = "author-tools"))]
        let entries = history::collect(&dir, usize::MAX);
        let accounts = Timeline::from_history(&entries, now_slot);
        report::report(
            &ix,
            period,
            OffsetDateTime::now_utc(),
            &local_time::offset_at,
            &accounts,
            &home,
        )
    })
    .await
    .map_err(|e| e.to_string())
}

/// "Đã biết" on the notice of skipped logs.
#[tauri::command]
pub fn stats_ack(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.inner().clone();
    if shared.lock().stats.reading {
        return Err("busy".into());
    }
    let mut ix = (*index_of(&shared)).clone();
    ix.acknowledge();
    let dir = shared.lock().app_dir();
    index::save(&dir, &ix).map_err(|_| "io".to_string())?;
    {
        let mut inner = shared.lock();
        inner.stats.index = Some(Arc::new(ix));
        inner.stats.generation += 1;
    }
    reader::publish(&app, &shared);
    Ok(())
}
