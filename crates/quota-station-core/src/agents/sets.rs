//! Configuration sets (M15, N): a named list of which MCP servers and
//! skills are on or off, and which side the Codex switch is on, applied to
//! this whole Mac in one go after one preview. Sets are named after
//! projects ("Web app", "Viết tài liệu"), but nothing is ever written into
//! a project's folder.
//!
//! A set says, for each MCP server and skill it names, `On` or `Off`; one
//! it does not name is left as it is. For Codex it says nothing (Giữ
//! nguyên) or the side of the Codex switch it wants.
//!
//! - [`changes`] turns a set into the boxes of the MCP and Skills tables it
//!   changes: an MCP server for every agent that has it, a skill for every
//!   agent whose box can be ticked. A box that is locked is skipped and
//!   said so; an entry this Mac does not have is skipped.
//! - [`preview`] reads the files now and shows those changes through the
//!   MCP tab's and the Skills tab's own previews ([`batch::preview`],
//!   [`skills::preview`]); [`apply`] writes exactly them through their own
//!   checked writes ([`batch::write`], [`skills::write`]): each file read
//!   again, backed up without secrets, written once. Only the entries in
//!   the set change; no file is rewritten whole.
//! - Codex's provider never changes here. The windows open the Codex
//!   switch of version 1 as step 2 when the set asks for the other side.
//! - [`undo`] puts back every entry the last apply changed and that is
//!   still as the apply left it; one changed since stays as it is.
//! - [`drift`] says which entries no longer are as the applied set says
//!   ("đã lệch").
//!
//! The sets and the last apply are kept in `agents/sets.json`, names only,
//! never a secret. A set is never applied on its own.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::batch::{self, To};
use super::secrets::Secrets;
use super::skills::{self, SkillPlaces};
use super::{Agent, Places, View};
use crate::antigravity::fs::AppDir;

/// The sets and the last apply, in the app's folder.
pub const FILE: &str = "agents/sets.json";
/// The most sets kept.
pub const MAX_SETS: usize = 50;
/// The longest name, in characters.
pub const MAX_NAME: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Want {
    On,
    Off,
}

impl Want {
    fn on(self) -> bool {
        self == Want::On
    }
}

/// The side of the Codex switch a set wants: the ChatGPT login or the
/// custom provider, as the switch names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    ChatGpt,
    Custom,
}

impl Provider {
    /// The Codex tab's name for the side Codex is on (`chatgpt`, `custom`).
    pub fn current(self) -> &'static str {
        match self {
            Provider::ChatGpt => "chatgpt",
            Provider::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Set {
    pub id: String,
    pub name: String,
    /// Not named: left as it is (Không đổi).
    #[serde(default)]
    pub mcp: BTreeMap<String, Want>,
    #[serde(default)]
    pub skills: BTreeMap<String, Want>,
    /// None: Codex stays on the side it is (Giữ nguyên).
    #[serde(default)]
    pub codex: Option<Provider>,
}

/// The last apply: what puts it back, names only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub id: String,
    /// The set's name then, for a set deleted since.
    pub name: String,
    /// Unix seconds.
    pub at: i64,
    #[serde(default)]
    pub mcp: Option<batch::Undo>,
    #[serde(default)]
    pub skills: Option<skills::Undo>,
    /// The side Codex was on before (`chatgpt`, `custom`, `other`), when the
    /// set wanted the other one: the undo offers the Codex switch back.
    #[serde(default)]
    pub codex_before: Option<String>,
}

impl Applied {
    /// Something the files can be put back from.
    pub fn can_undo(&self) -> bool {
        self.mcp.as_ref().is_some_and(|u| !u.changes.is_empty())
            || self.skills.as_ref().is_some_and(|u| !u.changes.is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Sets {
    pub sets: Vec<Set>,
    pub applied: Option<Applied>,
}

impl Sets {
    pub fn get(&self, id: &str) -> Option<&Set> {
        self.sets.iter().find(|s| s.id == id)
    }
}

pub fn load(app: &AppDir) -> Sets {
    skills::load_json(app, FILE)
}

pub fn save(app: &AppDir, sets: &Sets) -> std::io::Result<()> {
    skills::save_json(app, FILE, sets)
}

// ===== Keeping the sets =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum SetError {
    /// No name, or a longer one than [`MAX_NAME`].
    Name,
    /// Another set has that name.
    Taken,
    /// A server or skill name the app does not write.
    BadEntry(String),
    /// [`MAX_SETS`] already.
    Full,
    /// No set with that id.
    Missing,
    Io(String),
}

/// `set` as kept: the name trimmed, every entry's name checked.
fn clean(set: &Set) -> Result<Set, SetError> {
    let name = set.name.trim().to_string();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(SetError::Name);
    }
    if let Some(bad) = set.mcp.keys().find(|n| !super::valid_name(n)) {
        return Err(SetError::BadEntry(bad.clone()));
    }
    if let Some(bad) = set.skills.keys().find(|n| !skills::valid_name(n)) {
        return Err(SetError::BadEntry(bad.clone()));
    }
    Ok(Set { name, ..set.clone() })
}

fn new_id(sets: &Sets, now: OffsetDateTime) -> String {
    let mut n = now.unix_timestamp_nanos() / 1_000_000;
    loop {
        let id = format!("s{n}");
        if sets.get(&id).is_none() {
            return id;
        }
        n += 1;
    }
}

/// Saves `set`: a new one when its id is empty or not kept, else in place
/// of the one with its id. Never touches an agent's file.
pub fn put(app: &AppDir, set: &Set, now: OffsetDateTime) -> Result<Set, SetError> {
    let mut sets = load(app);
    let mut set = clean(set)?;
    let taken = |s: &Set| s.id != set.id && s.name.to_lowercase() == set.name.to_lowercase();
    if sets.sets.iter().any(taken) {
        return Err(SetError::Taken);
    }
    match sets.sets.iter_mut().find(|s| !set.id.is_empty() && s.id == set.id) {
        Some(old) => *old = set.clone(),
        None => {
            if sets.sets.len() >= MAX_SETS {
                return Err(SetError::Full);
            }
            if set.id.is_empty() {
                set.id = new_id(&sets, now);
            }
            sets.sets.push(set.clone());
        }
    }
    save(app, &sets).map_err(|e| SetError::Io(e.to_string()))?;
    Ok(set)
}

/// A set deleted, to put back with [`undelete`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Removed {
    pub set: Set,
    pub index: usize,
    /// The last apply, when it was of this set.
    pub applied: Option<Applied>,
}

/// Deletes a set. The agents' files stay as they are; the last apply goes
/// with it when it was of this set.
pub fn delete(app: &AppDir, id: &str) -> Result<Removed, SetError> {
    let mut sets = load(app);
    let index = sets.sets.iter().position(|s| s.id == id).ok_or(SetError::Missing)?;
    let set = sets.sets.remove(index);
    let applied =
        if sets.applied.as_ref().is_some_and(|a| a.id == id) { sets.applied.take() } else { None };
    save(app, &sets).map_err(|e| SetError::Io(e.to_string()))?;
    Ok(Removed { set, index, applied })
}

/// Puts a deleted set back where it was ("Hoàn tác" of a delete).
pub fn undelete(app: &AppDir, removed: &Removed) -> Result<(), SetError> {
    let mut sets = load(app);
    if sets.get(&removed.set.id).is_some() {
        return Ok(());
    }
    let set = clean(&removed.set)?;
    if sets.sets.iter().any(|s| s.name.to_lowercase() == set.name.to_lowercase()) {
        return Err(SetError::Taken);
    }
    let at = removed.index.min(sets.sets.len());
    sets.sets.insert(at, set);
    if sets.applied.is_none() {
        sets.applied = removed.applied.clone();
    }
    save(app, &sets).map_err(|e| SetError::Io(e.to_string()))
}

// ===== From a set to the boxes =====

/// An entry of the set that cannot be made as it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    /// `mcp` or `skill`.
    pub kind: &'static str,
    pub name: String,
    /// None: for every agent.
    pub agent: Option<Agent>,
    /// `notHere` (this Mac has not got it), `owned` (an agent's or app's
    /// own, never changed), or the box's lock (`readOnly`, `missingKey`,
    /// `hand`, …).
    pub reason: &'static str,
}

/// The boxes a set changes now, and what it cannot change.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Planned {
    pub mcp: Vec<batch::Change>,
    pub skills: Vec<skills::Change>,
    pub skipped: Vec<Skipped>,
}

/// What `set` changes in the tables as they are: for an MCP server, every
/// agent that has it (Claude Code's turned off and kept by the app counts);
/// for a skill, every agent whose box can be ticked.
pub fn changes(set: &Set, mcp: &View, skill_view: &skills::View) -> Planned {
    let mut p = Planned::default();
    for (name, want) in &set.mcp {
        let Some(row) = mcp.rows.iter().find(|r| &r.name == name) else {
            let reason =
                if mcp.owned.iter().any(|o| &o.name == name) { "owned" } else { "notHere" };
            p.skipped.push(Skipped { kind: "mcp", name: name.clone(), agent: None, reason });
            continue;
        };
        for (agent, cell) in &row.cells {
            if cell.state == "absent" || (cell.state == "on") == want.on() {
                continue;
            }
            match cell.lock {
                Some(reason) => p.skipped.push(Skipped {
                    kind: "mcp",
                    name: name.clone(),
                    agent: Some(*agent),
                    reason,
                }),
                None => p.mcp.push(batch::Change {
                    name: name.clone(),
                    agent: *agent,
                    to: if want.on() { To::On } else { To::Off },
                }),
            }
        }
    }
    for (name, want) in &set.skills {
        let Some(row) = skill_view.rows.iter().find(|r| &r.name == name) else {
            let owned = skill_view.owned.iter().any(|o| o.names.contains(name));
            let reason = if owned { "owned" } else { "notHere" };
            p.skipped.push(Skipped { kind: "skill", name: name.clone(), agent: None, reason });
            continue;
        };
        for (agent, cell) in &row.cells {
            if cell.state == "absent" || (cell.state == "on") == want.on() {
                continue;
            }
            match cell.lock {
                Some(reason) => p.skipped.push(Skipped {
                    kind: "skill",
                    name: name.clone(),
                    agent: Some(*agent),
                    reason,
                }),
                None => p.skills.push(skills::Change {
                    name: name.clone(),
                    agent: *agent,
                    on: want.on(),
                }),
            }
        }
    }
    p
}

/// An entry no longer as the applied set says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drifted {
    /// `mcp` or `skill`.
    pub kind: &'static str,
    pub name: String,
    pub want: Want,
    /// The agents where it is not.
    pub agents: Vec<Agent>,
}

/// The entries of `set` the tables no longer have as it says: those the
/// set would change if applied now. A box that cannot be ticked does not
/// count.
pub fn drift(set: &Set, mcp: &View, skill_view: &skills::View) -> Vec<Drifted> {
    let p = changes(set, mcp, skill_view);
    let mut out: Vec<Drifted> = Vec::new();
    let mut add = |kind: &'static str, name: &str, want: Want, agent: Agent| match out
        .iter_mut()
        .find(|d| d.kind == kind && d.name == name)
    {
        Some(d) => d.agents.push(agent),
        None => out.push(Drifted { kind, name: name.to_string(), want, agents: vec![agent] }),
    };
    for c in &p.mcp {
        add("mcp", &c.name, if c.to == To::On { Want::On } else { Want::Off }, c.agent);
    }
    for c in &p.skills {
        add("skill", &c.name, if c.on { Want::On } else { Want::Off }, c.agent);
    }
    out
}

// ===== The preview =====

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub name: String,
    /// The MCP tab's preview of the set's servers.
    pub mcp: batch::Preview,
    /// The Skills tab's preview of the set's skills.
    pub skills: skills::Preview,
    pub skipped: Vec<Skipped>,
    /// The side the set wants Codex on: the windows open the Codex switch
    /// for it after the write, when Codex is not on it.
    pub codex: Option<Provider>,
}

/// Reads every agent's files now and shows what applying `set` would
/// write. Nothing is written.
pub fn preview(
    places: &Places,
    skill_places: &SkillPlaces,
    app: &AppDir,
    secrets: &Secrets,
    set: &Set,
) -> Preview {
    let mcp_view = super::look(places, app, secrets);
    let skill_view = skills_view(skill_places);
    let planned = changes(set, &mcp_view, &skill_view);
    Preview {
        id: set.id.clone(),
        name: set.name.clone(),
        mcp: batch::preview(places, app, secrets, &planned.mcp),
        skills: skills::preview(skill_places, &planned.skills),
        skipped: planned.skipped,
        codex: set.codex,
    }
}

/// The Skills table as read now: rows and cells, no plugins or updates.
pub fn skills_view(places: &SkillPlaces) -> skills::View {
    let look = skills::read(places);
    skills::view(places, &look, Vec::new(), &Default::default())
}

// ===== The write =====

/// What the preview showed, sent back with the write.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Sent {
    pub mcp: Vec<batch::Change>,
    pub mcp_bases: BTreeMap<Agent, String>,
    pub skills: Vec<skills::Change>,
    pub skill_bases: BTreeMap<Agent, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum ApplyError {
    Missing,
    /// This agent's file or folder is not what the preview read: nothing
    /// written.
    Changed(Agent),
    /// The change is not one the set makes now: nothing written.
    NotTheSet,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub mcp: Option<batch::Outcome>,
    /// Not run when the MCP part failed.
    pub skills: Option<skills::Outcome>,
    /// Every part written.
    pub ok: bool,
    /// Unix seconds.
    pub at: i64,
}

/// Writes the preview of `set` the owner saw: the MCP servers through the
/// MCP tab's batch, then the skills through the Skills tab's, when every
/// file and folder they touch is still what the preview read. The first
/// failure stops it; what was written before it is in the undo.
/// `codex_now` is the side Codex is on now, kept for the undo when the set
/// wants the other one.
#[allow(clippy::too_many_arguments)]
pub fn apply(
    places: &Places,
    skill_places: &SkillPlaces,
    app: &AppDir,
    secrets: &Secrets,
    id: &str,
    sent: &Sent,
    codex_now: &str,
    now: OffsetDateTime,
) -> Result<Outcome, ApplyError> {
    let at = now.unix_timestamp();
    let mut sets = load(app);
    let set = sets.get(id).cloned().ok_or(ApplyError::Missing)?;

    // Every file and folder first: nothing is written when one changed.
    let readings = super::read_all(places);
    for agent in sent.mcp.iter().map(|c| c.agent).collect::<BTreeSet<_>>() {
        let base = readings.iter().find(|r| r.agent == agent).map(|r| &r.base);
        if base != sent.mcp_bases.get(&agent) {
            return Err(ApplyError::Changed(agent));
        }
    }
    let look = skills::read(skill_places);
    for agent in sent.skills.iter().map(|c| c.agent).collect::<BTreeSet<_>>() {
        if Some(&look.reading(agent).base) != sent.skill_bases.get(&agent) {
            return Err(ApplyError::Changed(agent));
        }
    }

    // Only boxes this set changes: the windows cannot slip in others.
    let planned = changes(&set, &super::look(places, app, secrets), &skills_view(skill_places));
    let ours_mcp = sent.mcp.iter().all(|c| planned.mcp.contains(c));
    let ours_skills = sent.skills.iter().all(|c| planned.skills.contains(c));
    if !ours_mcp || !ours_skills {
        return Err(ApplyError::NotTheSet);
    }

    let mut out = Outcome { mcp: None, skills: None, ok: true, at };
    let mut skill_bases = sent.skill_bases.clone();
    if !sent.mcp.is_empty() {
        let o = batch::write(places, app, secrets, &sent.mcp, &sent.mcp_bases, now);
        out.ok = o.failed.is_none();
        // Codex's `config.toml` holds both parts: the skills' write expects
        // it as the MCP write left it.
        if o.done.iter().any(|d| d.agent == Agent::Codex) && skill_bases.contains_key(&Agent::Codex)
        {
            if let Some(b) = o.undo.as_ref().and_then(|u| u.bases.get(&Agent::Codex)) {
                skill_bases.insert(Agent::Codex, b.clone());
            }
        }
        out.mcp = Some(o);
    }
    if out.ok && !sent.skills.is_empty() {
        let o = skills::write(skill_places, app, &sent.skills, &skill_bases, now);
        out.ok = o.failed.is_none();
        out.skills = Some(o);
    }

    let wrote = out.mcp.as_ref().is_some_and(|o| !o.done.is_empty())
        || out.skills.as_ref().is_some_and(|o| !o.done.is_empty());
    if wrote || out.ok {
        let codex_before =
            set.codex.filter(|p| p.current() != codex_now).map(|_| codex_now.to_string());
        sets.applied = Some(Applied {
            id: set.id.clone(),
            name: set.name.clone(),
            at,
            mcp: out.mcp.as_ref().and_then(|o| o.undo.clone()),
            skills: out.skills.as_ref().and_then(|o| o.undo.clone()),
            codex_before,
        });
        let _ = save(app, &sets);
    }
    Ok(out)
}

// ===== Hoàn tác =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum UndoError {
    /// No apply to undo.
    Nothing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undone {
    pub skills: Option<skills::Outcome>,
    pub mcp: Option<batch::Outcome>,
    /// Entries changed since the apply (`name`): left as they are.
    pub left: Vec<String>,
    /// Every entry that could be put back was.
    pub ok: bool,
    /// The side Codex was on before the set, when step 2 asked for the
    /// other: the windows offer the Codex switch back.
    pub codex_before: Option<String>,
}

/// Undoes the last apply, entry by entry: the skills, then the MCP
/// servers, each through its tab's checked write read from the files now.
/// An entry put back by hand already needs nothing; one an agent no longer
/// has stays gone; one changed since some other way stays as it is. Codex's
/// provider is not touched: the windows offer the Codex switch for it.
pub fn undo(
    places: &Places,
    skill_places: &SkillPlaces,
    app: &AppDir,
    secrets: &Secrets,
    now: OffsetDateTime,
) -> Result<Undone, UndoError> {
    let mut sets = load(app);
    let applied = sets.applied.clone().ok_or(UndoError::Nothing)?;
    let mut out = Undone {
        skills: None,
        mcp: None,
        left: Vec::new(),
        ok: true,
        codex_before: applied.codex_before.clone(),
    };
    let mut left: BTreeSet<String> = BTreeSet::new();

    if let Some(u) = applied.skills.as_ref().filter(|u| !u.changes.is_empty()) {
        let view = skills_view(skill_places);
        let here = |c: &skills::Change| {
            view.rows
                .iter()
                .find(|r| r.name == c.name)
                .and_then(|r| r.cells.get(&c.agent))
                .is_some_and(|cell| cell.state != "absent")
        };
        let (changes, gone): (Vec<skills::Change>, Vec<skills::Change>) =
            u.changes.iter().cloned().partition(here);
        left.extend(gone.into_iter().map(|c| c.name));
        let p = skills::preview(skill_places, &changes);
        left.extend(p.refused.iter().filter(|r| r.reason != "nothing").map(|r| r.name.clone()));
        if !p.changes.is_empty() {
            let o = skills::write(skill_places, app, &p.changes, &p.bases, now);
            out.ok &= o.failed.is_none();
            out.skills = Some(o);
        }
    }

    if let Some(u) = applied.mcp.as_ref().filter(|u| out.ok && !u.changes.is_empty()) {
        let view = super::look(places, app, secrets);
        // Only on and off, and only where the agent still has the server
        // (Claude Code's kept off by the app counts): one removed since is
        // not added back from another agent.
        let here = |c: &batch::Change| {
            matches!(c.to, To::On | To::Off)
                && view
                    .rows
                    .iter()
                    .find(|r| r.name == c.name)
                    .and_then(|r| r.cells.get(&c.agent))
                    .is_some_and(|cell| cell.state != "absent")
        };
        let (changes, gone): (Vec<batch::Change>, Vec<batch::Change>) =
            u.changes.iter().cloned().partition(here);
        left.extend(gone.into_iter().map(|c| c.name));
        let p = batch::preview(places, app, secrets, &changes);
        left.extend(p.refused.iter().filter(|r| r.reason != "nothing").map(|r| r.name.clone()));
        if !p.changes.is_empty() {
            let o = batch::write(places, app, secrets, &p.changes, &p.bases, now);
            out.ok &= o.failed.is_none();
            out.mcp = Some(o);
        }
    }

    out.left = left.into_iter().collect();
    if out.ok {
        sets.applied = None;
        let _ = save(app, &sets);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(mcp: &[(&str, Want)], skills: &[(&str, Want)]) -> Set {
        Set {
            id: String::new(),
            name: "Web app".into(),
            mcp: mcp.iter().map(|(n, w)| (n.to_string(), *w)).collect(),
            skills: skills.iter().map(|(n, w)| (n.to_string(), *w)).collect(),
            codex: None,
        }
    }

    #[test]
    fn a_set_keeps_names_the_app_writes() {
        assert_eq!(clean(&Set { name: "  ".into(), ..set(&[], &[]) }), Err(SetError::Name));
        assert_eq!(clean(&set(&[("a b", Want::On)], &[])), Err(SetError::BadEntry("a b".into())));
        assert_eq!(
            clean(&set(&[], &[("../x", Want::Off)])),
            Err(SetError::BadEntry("../x".into()))
        );
        assert_eq!(
            clean(&Set { name: " Web app ".into(), ..set(&[], &[]) }).unwrap().name,
            "Web app"
        );
        let long = "x".repeat(MAX_NAME + 1);
        assert_eq!(clean(&Set { name: long, ..set(&[], &[]) }), Err(SetError::Name));
    }

    #[test]
    fn the_sets_file_reads_back_as_written() {
        let s = Sets {
            sets: vec![Set { codex: Some(Provider::ChatGpt), ..set(&[("docs", Want::On)], &[]) }],
            applied: None,
        };
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains("\"codex\":\"chatGpt\""), "{text}");
        assert!(text.contains("\"docs\":\"on\""), "{text}");
        assert_eq!(serde_json::from_str::<Sets>(&text).unwrap(), s);
    }
}
