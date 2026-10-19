//! Restoring a pack's MCP servers on this Mac (M10), in the steps the
//! Khôi phục sheet shows: [`plan`] (what each server would become, read
//! from the agents' files now; nothing written), then [`run`] with the
//! servers ticked and the keys pasted from Bitwarden, then, if wanted,
//! [`undo`] of the whole restore.
//!
//! - A server not in an agent here is added for it. One the agent has
//!   already, the same apart from its secrets and the home folder, is left
//!   alone; a different one is offered, unticked: the Mac's own stays
//!   unless the owner ticks it.
//! - Paths follow the Mac ([`pack::localize`]): `~` is this home folder, a
//!   command missing at its full path is looked for by its name.
//! - A secret is never in the pack. A key this Mac has (in an agent's file
//!   or the app's Keychain item) is used; one pasted goes to the Keychain
//!   first. A server still short of a key, or whose command is not on this
//!   Mac, is added turned off: the app keeps it ([`super::kho::Kho`]'s
//!   `waiting`, or `parked` for Claude Code), and the MCP table shows it off
//!   with its lock until the key is pasted or the command installed.
//! - Every file is written through [`batch::write`]: checked against what
//!   the plan read, backed up first without secrets, each file once.
//! - Codex's provider, logins and accounts are never touched.
//!
//! The last restore is kept in `agents/restore-last.json` (names only, no
//! secret) so that it can be undone after the app restarts: the batch that
//! puts the files back, the servers the app kept off, and the Keychain items
//! the restore made.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Map;
use time::OffsetDateTime;

use super::batch::{self, Change, To, Undo};
use super::def::{Def, REDACTED};
use super::kho::{self, Kept};
use super::pack::{self, Pack};
use super::secrets::Secrets;
use super::{def_view, kept_view, missing_command, writable, Agent, DefView, Have, Places};
use crate::antigravity::fs::AppDir;

/// The last restore, in the app's folder.
pub const FILE: &str = "agents/restore-last.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub agent: Agent,
    pub here: DefView,
    pub pack: DefView,
}

/// One line of the preview: a server to add for some agents, or one to
/// put in place of the Mac's own.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    /// `add` or `replace`.
    pub kind: &'static str,
    pub agents: Vec<Agent>,
    /// Ticked to begin with: an add is, a replace is not.
    pub checked: bool,
    /// Agents it goes to turned off, as it was in the pack.
    pub off: Vec<Agent>,
    /// Its command, when it is not on this Mac: added off.
    pub missing_command: Option<String>,
    /// Its command is somewhere else on this Mac than in the pack.
    pub moved: bool,
    /// Its secrets this Mac has nowhere.
    pub keys: Vec<String>,
    /// Over the network with no key: the agent may ask to log in again.
    pub login: bool,
    /// For a replace: the Mac's own and the pack's, secrets masked.
    pub compare: Vec<Compare>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyNeed {
    pub mcp: String,
    pub slot: String,
    pub label: String,
    pub bitwarden: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub agent: Agent,
    /// `absent` (not on this Mac), `readOnly` (its file does not read, or
    /// no `claude`), or `sse` (Codex and an SSE server, by name).
    pub reason: &'static str,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub pack: String,
    pub mac: String,
    pub at: i64,
    pub items: Vec<Item>,
    /// Servers this Mac has as the pack does.
    pub same: Vec<String>,
    pub keys: Vec<KeyNeed>,
    pub skipped: Vec<Skipped>,
    /// What each file was when read, to send back with the run.
    pub bases: BTreeMap<Agent, String>,
}

/// What a server is in an agent here: its entry, or the app's own copy.
fn here(
    readings: &[super::Reading],
    store: &kho::Kho,
    name: &str,
    agent: Agent,
) -> Option<(Def, DefView)> {
    let r = readings.iter().find(|r| r.agent == agent)?;
    if let Some(s) = r.server(name) {
        let d = s.def.as_ref()?;
        return Some((d.stripped(), def_view(d, &s.extra)));
    }
    let k = match agent {
        Agent::Claude => store.parked.get(name),
        _ => None,
    }
    .or_else(|| store.waiting.get(name).filter(|_| store.waits(name, agent)))?;
    Some((k.def.clone(), kept_view(k)))
}

/// What restoring `pack` would do here, read now. Writes nothing.
pub fn plan(places: &Places, app: &AppDir, secrets: &Secrets, id: &str, pack: &Pack) -> Plan {
    let readings = super::read_all(places);
    let store = kho::load(app);
    let have = Have { readings: &readings, secrets };
    let mut items = Vec::new();
    let mut same = Vec::new();
    let mut skipped: Vec<Skipped> = Vec::new();
    let mut keys: Vec<KeyNeed> = Vec::new();
    for server in &pack.mcp.servers {
        let name = server.name.as_str();
        let mut add = Vec::new();
        let mut off = Vec::new();
        let mut replace = Vec::new();
        let mut compare = Vec::new();
        let mut alike = 0;
        let mut defs: Vec<Def> = Vec::new();
        let mut moved = false;
        for entry in &server.entries {
            let Some(r) = readings.iter().find(|r| r.agent == entry.agent) else { continue };
            if !r.present {
                if !skipped.iter().any(|s| s.agent == r.agent && s.reason == "absent") {
                    skipped.push(Skipped { agent: r.agent, reason: "absent", name: None });
                }
                continue;
            }
            if !writable(places, r) {
                if !skipped.iter().any(|s| s.agent == r.agent && s.reason == "readOnly") {
                    skipped.push(Skipped { agent: r.agent, reason: "readOnly", name: None });
                }
                continue;
            }
            if r.server(name).is_some_and(|s| s.owner.is_some()) {
                continue;
            }
            let (def, m) = pack::localize(&entry.def, &places.home);
            if entry.agent == Agent::Codex && def.is_sse() {
                skipped.push(Skipped { agent: r.agent, reason: "sse", name: Some(name.into()) });
                continue;
            }
            match here(&readings, &store, name, entry.agent) {
                Some((mine, view)) => {
                    if mine.same(&def) || pack::portable(&mine, &places.home).same(&entry.def) {
                        alike += 1;
                    } else {
                        replace.push(entry.agent);
                        compare.push(Compare {
                            agent: entry.agent,
                            here: view,
                            pack: def_view(&def.masked(super::def::MASK), &[]),
                        });
                        moved |= m;
                        defs.push(def);
                    }
                }
                None => {
                    add.push(entry.agent);
                    if !entry.enabled {
                        off.push(entry.agent);
                    }
                    moved |= m;
                    defs.push(def);
                }
            }
        }
        if add.is_empty() && replace.is_empty() {
            if alike > 0 {
                same.push(name.to_string());
            }
            continue;
        }
        let mut need: Vec<String> = Vec::new();
        for d in &defs {
            for slot in pack::holes(d) {
                if !need.contains(&slot) && !have.exists(name, &slot) {
                    need.push(slot);
                }
            }
        }
        for slot in &need {
            if !keys.iter().any(|k| k.mcp == name && &k.slot == slot) {
                let n = pack.needed(name, slot);
                keys.push(KeyNeed {
                    mcp: name.to_string(),
                    slot: slot.clone(),
                    label: n
                        .map(|n| n.label.clone())
                        .filter(|l| !l.is_empty())
                        .unwrap_or_else(|| pack::slot_label(defs.first(), slot)),
                    bitwarden: n
                        .and_then(|n| n.bitwarden.clone())
                        .or_else(|| store.hint(name, slot).map(str::to_string)),
                });
            }
        }
        let first = defs.first();
        let missing = first
            .and_then(|d| missing_command(d, &places.home))
            .filter(|m| m.path)
            .map(|m| m.command.rsplit('/').next().unwrap_or(&m.command).to_string());
        let login = first.is_some_and(|d| d.is_http() && pack::holes(d).is_empty());
        let item = |kind, agents: Vec<Agent>, off: Vec<Agent>, compare: Vec<Compare>| Item {
            name: name.to_string(),
            kind,
            checked: kind == "add",
            agents,
            off,
            missing_command: missing.clone(),
            moved,
            keys: need.clone(),
            login,
            compare,
        };
        if !add.is_empty() {
            items.push(item("add", add, off, Vec::new()));
        }
        if !replace.is_empty() {
            items.push(item("replace", replace, Vec::new(), compare));
        }
    }
    let bases = readings.iter().filter(|r| r.ok()).map(|r| (r.agent, r.base.clone())).collect();
    Plan {
        pack: id.to_string(),
        mac: pack.manifest.mac.clone(),
        at: pack.manifest.at,
        items,
        same,
        keys,
        skipped,
        bases,
    }
}

/// A server ticked in the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pick {
    pub name: String,
    /// `add` or `replace`.
    pub kind: String,
}

/// A key pasted for the restore. Its value goes to the Keychain and
/// nowhere else.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pasted {
    pub mcp: String,
    pub slot: String,
    pub value: String,
}

impl std::fmt::Debug for Pasted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} = …", self.mcp, self.slot)
    }
}

/// How one ticked server ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub name: String,
    pub kind: String,
    /// Written, on (or off as the pack had it).
    pub done: Vec<Agent>,
    /// Added turned off by the app, and why: `noKey` or `noCommand`.
    pub kept: Vec<Agent>,
    pub why: Option<&'static str>,
    /// Not written: the write stopped before it, or the plan refused it.
    pub failed: Vec<Agent>,
    /// A replace left as it was, and why: `noKey` or `noCommand`.
    pub skipped: Option<&'static str>,
    pub login: bool,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub at: i64,
    pub items: Vec<ItemResult>,
    pub failed: Option<batch::Failed>,
    /// The files written, with their backups.
    pub written: Vec<batch::Done>,
    pub keys: usize,
    pub can_undo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum RunError {
    /// A file is not what the preview read: nothing written.
    Changed(Vec<Agent>),
    /// A pasted key did not go to the Keychain: nothing written.
    Keychain(String),
}

/// A server the app keeps off for an agent after a restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptRef {
    pub name: String,
    pub agent: Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotRef {
    pub mcp: String,
    pub slot: String,
}

/// The last restore, as kept to undo it: names only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub at: i64,
    pub pack: String,
    pub mac: String,
    pub undo: Option<Undo>,
    #[serde(default)]
    pub kept: Vec<KeptRef>,
    /// Keychain items that were not there before the restore.
    #[serde(default)]
    pub created: Vec<SlotRef>,
}

pub fn last(app: &AppDir) -> Option<Record> {
    app.read_to_string(&app.path().join(FILE)).ok().and_then(|t| serde_json::from_str(&t).ok())
}

fn save_record(app: &AppDir, record: &Record) {
    if let Ok(text) = serde_json::to_string_pretty(record) {
        let _ = app.write_private(&app.path().join(FILE), &text);
    }
}

fn forget_record(app: &AppDir) {
    let _ = app.remove(&app.path().join(FILE));
}

/// Restores the servers ticked in the preview of `pack`. `bases` are the
/// files as the preview read them: a file changed since stops the restore
/// before anything is written. Pasted keys go to the Keychain first; a
/// server still short of one, or of its command, is kept off by the app.
#[allow(clippy::too_many_arguments)]
pub fn run(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    id: &str,
    pack: &Pack,
    picks: &[Pick],
    pasted: &[Pasted],
    bases: &BTreeMap<Agent, String>,
    now: OffsetDateTime,
) -> Result<Done, RunError> {
    let at = now.unix_timestamp();
    let fresh = plan(places, app, secrets, id, pack);
    let picked: Vec<&Item> = fresh
        .items
        .iter()
        .filter(|i| picks.iter().any(|p| p.name == i.name && p.kind == i.kind))
        .collect();
    let involved: BTreeSet<Agent> = picked.iter().flat_map(|i| i.agents.iter().copied()).collect();
    let changed: Vec<Agent> =
        involved.iter().copied().filter(|a| bases.get(a) != fresh.bases.get(a)).collect();
    if !changed.is_empty() {
        return Err(RunError::Changed(changed));
    }

    // Every secret of the servers restored, and whether its Keychain item is
    // there before: the undo removes the items the restore made.
    let mut slots: BTreeSet<(String, String)> = BTreeSet::new();
    for item in &picked {
        let server = pack.mcp.servers.iter().find(|s| s.name == item.name);
        for e in server.into_iter().flat_map(|s| &s.entries) {
            for slot in pack::holes(&e.def) {
                slots.insert((item.name.clone(), slot));
            }
        }
    }
    let before: BTreeSet<(String, String)> =
        slots.iter().filter(|(m, s)| secrets.has(m, s).unwrap_or(true)).cloned().collect();

    let wanted: BTreeSet<(&str, &str)> = picked
        .iter()
        .flat_map(|i| i.keys.iter().map(move |k| (i.name.as_str(), k.as_str())))
        .collect();
    let mut saved: Vec<SlotRef> = Vec::new();
    for p in pasted {
        if p.value.trim().is_empty() || !wanted.contains(&(p.mcp.as_str(), p.slot.as_str())) {
            continue;
        }
        if let Err(e) = secrets.save(&p.mcp, &p.slot, p.value.trim()) {
            for s in &saved {
                let _ = secrets.delete(&s.mcp, &s.slot);
            }
            return Err(RunError::Keychain(e.to_string()));
        }
        saved.push(SlotRef { mcp: p.mcp.clone(), slot: p.slot.clone() });
    }
    let mut store = kho::load(app);
    let mut hinted = false;
    for n in &pack.secrets.secrets {
        if let Some(b) = n.bitwarden.as_deref().filter(|_| store.hint(&n.mcp, &n.slot).is_none()) {
            store.set_hint(&n.mcp, &n.slot, b);
            hinted = true;
        }
    }
    if hinted {
        let _ = kho::save(app, &store);
    }

    let readings = super::read_all(places);
    let have = Have { readings: &readings, secrets };
    let mut changes: Vec<Change> = Vec::new();
    let mut keep: Vec<(String, Agent, Def, &'static str)> = Vec::new();
    let mut results: Vec<ItemResult> = Vec::new();
    for item in &picked {
        let mut result = ItemResult {
            name: item.name.clone(),
            kind: item.kind.to_string(),
            done: Vec::new(),
            kept: Vec::new(),
            why: None,
            failed: Vec::new(),
            skipped: None,
            login: item.login,
            command: item.missing_command.clone(),
        };
        let Some(server) = pack.mcp.servers.iter().find(|s| s.name == item.name) else {
            continue;
        };
        let keychain: Vec<String> =
            saved.iter().filter(|s| s.mcp == item.name).map(|s| s.slot.clone()).collect();
        for agent in &item.agents {
            let Some(entry) = server.entries.iter().find(|e| e.agent == *agent) else { continue };
            let (def, _) = pack::localize(&entry.def, &places.home);
            let no_key = pack::holes(&def).iter().any(|s| !have.exists(&item.name, s));
            let no_cmd = missing_command(&def, &places.home).is_some_and(|m| m.path);
            let why = if no_key {
                Some("noKey")
            } else if no_cmd {
                Some("noCommand")
            } else {
                None
            };
            if item.kind == "replace" {
                if let Some(w) = why {
                    result.skipped = Some(w);
                    continue;
                }
                let to =
                    To::Def { def, from: Some(*agent), keychain: keychain.clone(), off: false };
                changes.push(Change { name: item.name.clone(), agent: *agent, to });
            } else if why.is_some() || (!entry.enabled && *agent == Agent::Claude) {
                result.why = result.why.or(why);
                keep.push((item.name.clone(), *agent, def, why.unwrap_or("packOff")));
            } else {
                let to =
                    To::Def { def, from: None, keychain: keychain.clone(), off: !entry.enabled };
                changes.push(Change { name: item.name.clone(), agent: *agent, to });
            }
        }
        results.push(result);
    }

    let mut out = None;
    if !changes.is_empty() {
        let p = batch::preview(places, app, secrets, &changes);
        for r in &p.refused {
            if let Some(res) = results.iter_mut().find(|x| x.name == r.name) {
                res.failed.push(r.agent);
            }
        }
        if !p.changes.is_empty() {
            out = Some(batch::write(places, app, secrets, &p.changes, bases, now));
        }
        let done: BTreeSet<Agent> =
            out.iter().flat_map(|o| o.done.iter().map(|d| d.agent)).collect();
        for c in &p.changes {
            if let Some(res) = results.iter_mut().find(|x| x.name == c.name) {
                if done.contains(&c.agent) {
                    res.done.push(c.agent);
                } else {
                    res.failed.push(c.agent);
                }
            }
        }
    }
    let nothing = out.as_ref().is_some_and(|o| o.done.is_empty() && o.failed.is_some());
    if nothing {
        // Nothing written: the keys pasted for it go too.
        for s in &saved {
            let _ = secrets.delete(&s.mcp, &s.slot);
        }
        let o = out.expect("checked");
        return Ok(Done {
            at,
            items: results,
            failed: o.failed,
            written: Vec::new(),
            keys: 0,
            can_undo: false,
        });
    }

    let mut store = kho::load(app);
    let mut kept_refs = Vec::new();
    for (name, agent, def, _) in &keep {
        let slots = pack::holes(def);
        let def = mask_holes(def);
        if *agent == Agent::Claude {
            let kept = Kept { def, slots, extra: Map::new(), agents: Vec::new(), at };
            store.parked.insert(name.clone(), kept);
        } else {
            let k = store.waiting.entry(name.clone()).or_insert_with(|| Kept {
                def,
                slots,
                extra: Map::new(),
                agents: Vec::new(),
                at,
            });
            if !k.agents.contains(agent) {
                k.agents.push(*agent);
                k.agents.sort();
            }
        }
        if let Some(res) = results.iter_mut().find(|x| &x.name == name) {
            res.kept.push(*agent);
        }
        kept_refs.push(KeptRef { name: name.clone(), agent: *agent });
    }
    if !keep.is_empty() {
        let _ = kho::save(app, &store);
    }

    let created: Vec<SlotRef> = slots
        .iter()
        .filter(|s| !before.contains(s))
        .map(|(m, s)| SlotRef { mcp: m.clone(), slot: s.clone() })
        .collect();
    let undo = out.as_ref().and_then(|o| o.undo.clone());
    let can_undo = undo.is_some() || !kept_refs.is_empty() || !saved.is_empty();
    if can_undo {
        save_record(
            app,
            &Record {
                at,
                pack: id.to_string(),
                mac: pack.manifest.mac.clone(),
                undo,
                kept: kept_refs,
                created,
            },
        );
    }
    Ok(Done {
        at,
        items: results,
        failed: out.as_ref().and_then(|o| o.failed.clone()),
        written: out.map(|o| o.done).unwrap_or_default(),
        keys: saved.len(),
        can_undo,
    })
}

/// A definition's holes as the store keeps them: [`REDACTED`].
fn mask_holes(def: &Def) -> Def {
    let mut out = def.clone();
    for slot in pack::holes(def) {
        out.set(&slot, REDACTED);
    }
    out
}

/// The agents whose files changed since the last restore wrote them.
pub fn changed_since(places: &Places, record: &Record) -> Vec<Agent> {
    let Some(undo) = &record.undo else { return Vec::new() };
    let readings = super::read_all(places);
    undo.bases
        .iter()
        .filter(|(a, base)| {
            readings.iter().find(|r| r.agent == **a).map(|r| &r.base) != Some(*base)
        })
        .map(|(a, _)| *a)
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "camelCase")]
pub enum UndoError {
    /// No restore to undo.
    Nothing,
    /// These files changed after the restore: the owner says first.
    Changed(Vec<Agent>),
    /// The files could not be put back.
    Failed(batch::Failed),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Undone {
    pub outcome: Option<batch::Outcome>,
    pub kept: usize,
    pub keys: usize,
}

/// Undoes the last restore: its batch put back, the servers it kept off
/// dropped, the Keychain items it made removed. A file changed since is
/// written only with `force`, and then only the restored servers' entries.
pub fn undo(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    force: bool,
    now: OffsetDateTime,
) -> Result<Undone, UndoError> {
    let record = last(app).ok_or(UndoError::Nothing)?;
    let changed = changed_since(places, &record);
    if !changed.is_empty() && !force {
        return Err(UndoError::Changed(changed));
    }
    let mut outcome = None;
    if let Some(u) = &record.undo {
        // An entry already as it was before (removed by hand since) needs
        // nothing.
        let p = batch::preview(places, app, secrets, &u.changes);
        if !p.changes.is_empty() {
            let bases = if changed.is_empty() { &u.bases } else { &p.bases };
            let out = batch::write(places, app, secrets, &p.changes, bases, now);
            if out.done.is_empty() {
                if let Some(f) = out.failed {
                    return Err(UndoError::Failed(f));
                }
            }
            outcome = Some(out);
        }
    }
    let mut store = kho::load(app);
    for k in &record.kept {
        if k.agent == Agent::Claude {
            store.parked.remove(&k.name);
        } else {
            store.unwait(&k.name, k.agent);
        }
    }
    if !record.kept.is_empty() {
        let _ = kho::save(app, &store);
    }
    let mut keys = 0;
    for s in &record.created {
        if secrets.delete(&s.mcp, &s.slot).is_ok() {
            keys += 1;
        }
    }
    forget_record(app);
    Ok(Undone { outcome, kept: record.kept.len(), keys })
}
