// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the Skills tab does through `npx skills` (M11, B and G): install
//! from GitHub, update, remove for good. Each keeps the agents' boxes as
//! the owner set them: the tool writes the shared folder and Claude Code's
//! link, and the app then makes or removes the other links (and Codex's
//! entry) through the same checked batch as the table's boxes.
//!
//! An update or a removal keeps a copy of the skill's folder first; updating
//! several copies every folder before the first update.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use time::OffsetDateTime;

use super::npx::{Npx, NpxError};
use super::skills::{self, Change, Entry, Look, Outcome, SkillPlaces};
use super::Agent;
use crate::app_dir::AppDir;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    /// The skills now in the shared folder.
    pub names: Vec<String>,
    /// The boxes set after the tool ran.
    pub outcome: Outcome,
    pub error: Option<NpxError>,
}

/// The boxes of `names` set to `want` (agents present and writable), as a
/// checked batch.
pub fn set_boxes(
    places: &SkillPlaces,
    app: &AppDir,
    names: &[String],
    want: &BTreeMap<Agent, bool>,
    now: OffsetDateTime,
) -> Outcome {
    let look = skills::read(places);
    let mut changes = Vec::new();
    for name in names.iter().filter(|n| look.shared.contains(*n)) {
        for (agent, on) in want {
            let r = look.reading(*agent);
            if !r.writable() || look.on(places, name, *agent) == *on {
                continue;
            }
            changes.push(Change { name: name.clone(), agent: *agent, on: *on });
        }
    }
    if changes.is_empty() {
        return Outcome::empty(now.unix_timestamp());
    }
    let p = skills::preview(places, &changes);
    skills::write(places, app, &p.changes, &p.bases, now)
}

/// "Cài": installs `names` from `repo`, then sets each agent's box as asked.
pub fn install(
    places: &SkillPlaces,
    app: &AppDir,
    npx: &Npx,
    repo: &str,
    names: &[String],
    want: &BTreeMap<Agent, bool>,
    now: OffsetDateTime,
) -> Installed {
    let before = skills::read(places);
    let fresh: Vec<String> = names
        .iter()
        .filter(|n| skills::valid_name(n) && !before.shared.contains(*n))
        .cloned()
        .collect();
    let mut out =
        Installed { names: Vec::new(), outcome: Outcome::empty(now.unix_timestamp()), error: None };
    if fresh.is_empty() {
        out.error = Some(NpxError::Failed("đã cài".into()));
        return out;
    }
    if let Err(e) = npx.add(repo, &fresh) {
        out.error = Some(e);
    }
    let after = skills::read(places);
    out.names = fresh.into_iter().filter(|n| after.shared.contains(n)).collect();
    if !out.names.is_empty() {
        out.outcome = set_boxes(places, app, &out.names, want, now);
    }
    out
}

/// The links to the shared folder in Claude Code's and Antigravity's
/// folders, put back to what `before` had for the skills a run touched.
fn keep_links(places: &SkillPlaces, before: &Look) {
    let after = skills::read(places);
    for agent in [Agent::Claude, Agent::Agy] {
        let (b, a) = (before.reading(agent), after.reading(agent));
        if !a.writable() {
            continue;
        }
        for name in after.shared.iter() {
            let was = matches!(b.entries.get(name), Some(Entry::Shared));
            let is = matches!(a.entries.get(name), Some(Entry::Shared));
            if was && !a.entries.contains_key(name) {
                let _ = skills::link(places, agent, name);
            } else if !was && is && !b.entries.contains_key(name) {
                let _ = skills::unlink(places, agent, name);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Updated {
    pub name: String,
    /// The copy kept before, in the app's backups.
    pub backup: Option<String>,
    pub error: Option<NpxError>,
}

/// "Cập nhật": a copy of the folder first, then `npx skills update`, then
/// every agent's box as it was.
pub fn update(
    places: &SkillPlaces,
    app: &AppDir,
    npx: &Npx,
    name: &str,
    now: OffsetDateTime,
) -> Updated {
    let mut out = update_many(places, app, npx, &[name.to_string()], now, |_, _, _| {});
    out.pop().unwrap_or(Updated { name: name.to_string(), backup: None, error: None })
}

/// "Cập nhật tất cả" and "Cập nhật cả bộ": a copy of every folder first,
/// then `npx skills update` one skill at a time, so each says how it ended,
/// then every agent's box as it was. A skill whose copy failed is not
/// updated. `step(i, total, name)` before the i-th of the `total` updates.
pub fn update_many(
    places: &SkillPlaces,
    app: &AppDir,
    npx: &Npx,
    names: &[String],
    now: OffsetDateTime,
    mut step: impl FnMut(usize, usize, &str),
) -> Vec<Updated> {
    let before = skills::read(places);
    let mut seen = BTreeSet::new();
    let mut out: Vec<Updated> = names
        .iter()
        .filter(|n| seen.insert(n.as_str()))
        .map(|name| Updated { name: name.clone(), backup: None, error: None })
        .collect();
    for u in out.iter_mut() {
        let name = u.name.as_str();
        if !skills::valid_name(name)
            || !before.shared.contains(name)
            || before.locked(name).is_none()
        {
            u.error = Some(NpxError::Failed("không có trong file khoá".into()));
            continue;
        }
        match skills::keep_folder(app, name, &places.shared.join(name), now) {
            Ok(id) => u.backup = Some(id),
            Err(e) => u.error = Some(NpxError::Failed(e.to_string())),
        }
    }
    let total = out.iter().filter(|u| u.error.is_none()).count();
    for (i, u) in out.iter_mut().filter(|u| u.error.is_none()).enumerate() {
        step(i, total, &u.name);
        if let Err(e) = npx.update(&u.name) {
            u.error = Some(e);
        }
    }
    keep_links(places, &before);
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
    pub names: Vec<String>,
    pub backups: Vec<String>,
    pub error: Option<NpxError>,
}

/// "Gỡ hẳn" (and the undo of an install): a copy of each folder first,
/// then `npx skills remove`, then the links it does not know of and the
/// app's own Codex entries.
pub fn remove(
    places: &SkillPlaces,
    app: &AppDir,
    npx: &Npx,
    names: &[String],
    now: OffsetDateTime,
) -> Removed {
    let before = skills::read(places);
    let mut out = Removed { names: Vec::new(), backups: Vec::new(), error: None };
    let names: Vec<String> = names
        .iter()
        .filter(|n| {
            skills::valid_name(n) && before.shared.contains(*n) && before.locked(n).is_some()
        })
        .cloned()
        .collect();
    if names.is_empty() {
        out.error = Some(NpxError::Failed("không có trong file khoá".into()));
        return out;
    }
    for n in &names {
        match skills::keep_folder(app, n, &places.shared.join(n), now) {
            Ok(id) => out.backups.push(id),
            Err(e) => {
                out.error = Some(NpxError::Failed(e.to_string()));
                return out;
            }
        }
    }
    if let Err(e) = npx.remove(&names) {
        out.error = Some(e);
    }
    let after = skills::read(places);
    out.names = names.iter().filter(|n| !after.shared.contains(*n)).cloned().collect();
    tidy(places, app, &out.names, now);
    out
}

/// After skills left the shared folder: the links to them that now lead
/// nowhere, and the app's own Codex entries for them, go.
pub fn tidy(places: &SkillPlaces, app: &AppDir, names: &[String], now: OffsetDateTime) {
    let after = skills::read(places);
    for n in names {
        for agent in [Agent::Claude, Agent::Agy] {
            if matches!(after.reading(agent).entries.get(n), Some(Entry::Broken { .. })) {
                let _ = skills::drop_broken(places, agent, n);
            }
        }
    }
    // Codex's entries the app wrote for a skill no longer there.
    let codex = after.reading(Agent::Codex);
    let stale: Vec<(String, bool)> = names
        .iter()
        .filter(|n| skills::codex_off(places, &codex.config, n))
        .map(|n| (n.clone(), true))
        .collect();
    if !stale.is_empty() && codex.writable() {
        let _ = skills::write_codex(places, app, codex, &stale, now);
    }
}
