// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The app's Codex switch log, `codex/switch-log.jsonl` in its folder: each
//! switch made here, and each change of provider the app saw that it did
//! not make. Providers are kept by id (`openai` for the ChatGPT login).

use serde::{Deserialize, Serialize};

use crate::app_dir::AppDir;

pub const FILE: &str = "codex/switch-log.jsonl";
const KEEP: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// Unix seconds.
    pub at: i64,
    pub from: String,
    pub to: String,
    /// `switched`, `failed`, `observed` (changed outside the app), `plugins`
    /// (the plugins choice applied on the custom provider in use), `key`
    /// ("Dán khoá") or `keyUndo`. A key's line names only the provider.
    pub result: String,
    /// The background process was restarted after the switch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restarted: Option<bool>,
    /// `requires_openai_auth` as written, on the way to the custom provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<bool>,
}

pub fn read(app: &AppDir) -> Vec<Line> {
    let text = app.read_to_string(&app.path().join(FILE)).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

/// Adds one line, keeping the newest [`KEEP`].
pub fn append(app: &AppDir, line: &Line) -> std::io::Result<()> {
    let mut lines = read(app);
    lines.push(line.clone());
    write(app, &lines)
}

/// Marks the newest switch as followed by a restart of the background process.
pub fn mark_restarted(app: &AppDir, restarted: bool) -> std::io::Result<()> {
    let mut lines = read(app);
    let Some(last) =
        lines.iter_mut().rev().find(|l| l.result == "switched" || l.result == "plugins")
    else {
        return Ok(());
    };
    last.restarted = Some(restarted);
    write(app, &lines)
}

fn write(app: &AppDir, lines: &[Line]) -> std::io::Result<()> {
    let skip = lines.len().saturating_sub(KEEP);
    let mut out = String::new();
    for l in &lines[skip..] {
        out.push_str(&serde_json::to_string(l).map_err(std::io::Error::other)?);
        out.push('\n');
    }
    app.write_private(&app.path().join(FILE), &out)
}
