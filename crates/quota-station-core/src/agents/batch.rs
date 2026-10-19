//! A batch: the boxes ticked in the MCP table, the add and edit forms, a
//! delete, an undo. [`preview`] reads every agent's file now and shows what
//! each would become, secrets masked; [`write`] makes exactly that, and
//! only when every file it touches still is what the preview read. Each
//! file is written once for the whole batch, after its backup.
//!
//! A secret a new entry needs is taken from the agent's file the server is
//! copied from, else any agent's file that has it, else the app's Keychain
//! item. In a definition sent from the windows a secret is the mask (or
//! [`REDACTED`]): the windows never hold one. The preview never fills one
//! in at all.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};
use time::OffsetDateTime;

use super::def::{Def, MASK, REDACTED};
use super::kho::{self, Kept, Kho};
use super::secrets::Secrets;
use super::{
    agy_mcp, claude_mcp, codex_mcp, missing_command, redact, valid_name, writable, write, Agent,
    EditError, Have, Places, Reading,
};
use crate::antigravity::fs::AppDir;
use crate::codex::files as codex_files;
use crate::runner;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub name: String,
    pub agent: Agent,
    pub to: To,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum To {
    On,
    Off,
    /// Out of the agent's list. With `store`, the app keeps the server in
    /// its store, secrets in the Keychain, to turn on again later.
    Absent {
        #[serde(default)]
        store: bool,
    },
    /// This definition, added or in place of the one there. Its secrets
    /// are masked: they come from `from`'s file, another agent's or the
    /// Keychain; `keychain` names the slots to take from the Keychain first
    /// (one just typed or pasted). `off` adds it turned off.
    Def {
        def: Def,
        #[serde(default)]
        from: Option<Agent>,
        #[serde(default)]
        keychain: Vec<String>,
        #[serde(default)]
        off: bool,
    },
}

/// Why a change is not in the batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Refused {
    pub name: String,
    pub agent: Agent,
    /// `readOnly`, `shape`, `owned`, `missingKey`, `noCommand`, `sse`,
    /// `badName`, `unknown` (nothing to add it from) or `nothing` (it is so
    /// already).
    pub reason: &'static str,
}

/// One edit of one agent's list.
#[derive(Debug, Clone, PartialEq)]
enum Op {
    Enable,
    Disable,
    /// Claude Code: kept by the app, then removed.
    Park,
    Add {
        def: Def,
        from: Option<Agent>,
        keychain: Vec<String>,
        enabled: bool,
        extra: Map<String, Json>,
        unpark: bool,
        unstore: bool,
        /// A restored server the app kept off for this agent: no longer.
        unwait: bool,
    },
    Remove {
        store: bool,
        agents: Vec<Agent>,
    },
    Replace {
        def: Def,
        from: Option<Agent>,
        keychain: Vec<String>,
    },
    /// Claude Code: one the app kept, dropped (and put away with `store`).
    Unpark {
        store: bool,
        agents: Vec<Agent>,
    },
    /// A restored server waiting for this agent, dropped (and put away
    /// with `store`). No file changes.
    Unwait {
        store: bool,
        agents: Vec<Agent>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct Step {
    name: String,
    op: Op,
}

fn action(op: &Op) -> &'static str {
    match op {
        Op::Enable => "on",
        Op::Disable | Op::Park => "off",
        Op::Add { .. } => "add",
        Op::Remove { .. } | Op::Unpark { .. } | Op::Unwait { .. } => "remove",
        Op::Replace { .. } => "replace",
    }
}

fn is_hole(v: &str) -> bool {
    v == MASK || v == REDACTED
}

/// The slots of `def` still to be filled in.
pub fn holes(def: &Def) -> Vec<String> {
    match def {
        Def::Stdio { args, env, .. } => args
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                is_hole(a)
                    || a.split_once('=').is_some_and(|(k, v)| k.starts_with('-') && is_hole(v))
            })
            .map(|(i, _)| format!("arg:{i}"))
            .chain(env.iter().filter(|p| is_hole(&p.value)).map(|p| format!("env:{}", p.key)))
            .collect(),
        Def::Http { url, headers, .. } => (url.contains(MASK) || url.contains(REDACTED))
            .then(|| "url".to_string())
            .into_iter()
            .chain(
                headers.iter().filter(|p| is_hole(&p.value)).map(|p| format!("header:{}", p.key)),
            )
            .collect(),
    }
}

/// `def` with its holes filled in. With `values` false (the preview) a
/// hole only has to be found somewhere, and stays masked.
fn resolve(
    name: &str,
    def: &Def,
    from: Option<Agent>,
    keychain: &[String],
    have: &Have<'_>,
    values: bool,
) -> Result<Def, Vec<String>> {
    let mut out = def.clone();
    let mut missing = Vec::new();
    for slot in holes(def) {
        let from_keychain = || {
            if values {
                have.secrets.read(name, &slot).ok()
            } else {
                have.secrets.has(name, &slot).ok().filter(|h| *h).map(|_| MASK.to_string())
            }
        };
        let from_files =
            || have.in_files(name, &slot, from).map(|v| if values { v } else { MASK.to_string() });
        let value = if keychain.contains(&slot) {
            from_keychain().or_else(from_files)
        } else {
            from_files().or_else(from_keychain)
        };
        match value {
            Some(v) => {
                if values {
                    out.set(&slot, &v);
                }
            }
            None => missing.push(slot),
        }
    }
    if missing.is_empty() {
        Ok(out)
    } else {
        Err(missing)
    }
}

/// A new entry's definition, the agent its secrets come from, the members
/// it keeps, and whether it leaves the app's parked or stored copies.
type Source = (Def, Option<Agent>, Map<String, Json>, bool, bool);

/// What a new entry for `name` is copied from: the restored copy waiting
/// for this agent, the app's own copy for Claude Code, another agent's
/// entry (one that is on first), or the store.
fn source(readings: &[Reading], store: &Kho, name: &str, agent: Agent) -> Option<Source> {
    if let Some(k) = store.waiting.get(name).filter(|_| store.waits(name, agent)) {
        return Some((k.def.clone(), None, Map::new(), false, false));
    }
    if agent == Agent::Claude {
        if let Some(k) = store.parked.get(name) {
            return Some((k.def.clone(), None, k.extra.clone(), true, false));
        }
    }
    let mut found: Vec<(&Reading, &super::Server)> = readings
        .iter()
        .filter(|r| r.ok() && r.agent != agent)
        .filter_map(|r| r.server(name).map(|s| (r, s)))
        .filter(|(_, s)| s.def.is_some() && s.owner.is_none())
        .collect();
    found.sort_by_key(|(r, s)| (!s.enabled, r.agent));
    if let Some((r, s)) = found.first() {
        let def = s.def.as_ref().expect("filtered").stripped();
        return Some((def, Some(r.agent), Map::new(), false, false));
    }
    if agent != Agent::Claude {
        if let Some(k) = store.parked.get(name) {
            return Some((k.def.clone(), None, Map::new(), false, false));
        }
    }
    store.stored.get(name).map(|k| {
        let extra = if agent == Agent::Claude { k.extra.clone() } else { Map::new() };
        (k.def.clone(), None, extra, false, true)
    })
}

struct Planned {
    steps: BTreeMap<Agent, Vec<Step>>,
    refused: Vec<Refused>,
    accepted: Vec<Change>,
}

fn plan(
    places: &Places,
    readings: &[Reading],
    store: &Kho,
    have: &Have<'_>,
    changes: &[Change],
) -> Planned {
    let mut p = Planned { steps: BTreeMap::new(), refused: Vec::new(), accepted: Vec::new() };
    // The last change for a box wins.
    let mut last: Vec<&Change> = Vec::new();
    for c in changes {
        last.retain(|x| !(x.name == c.name && x.agent == c.agent));
        last.push(c);
    }
    for c in last {
        let refuse = |p: &mut Planned, reason| {
            p.refused.push(Refused { name: c.name.clone(), agent: c.agent, reason })
        };
        let Some(r) = readings.iter().find(|r| r.agent == c.agent) else { continue };
        if !writable(places, r) {
            refuse(&mut p, "readOnly");
            continue;
        }
        let server = r.server(&c.name);
        if server.is_some_and(|s| s.owner.is_some()) {
            refuse(&mut p, "owned");
            continue;
        }
        if c.agent == Agent::Claude && !valid_name(&c.name) {
            refuse(&mut p, "badName");
            continue;
        }
        let editable = server.is_some_and(|s| s.editable && s.def.is_some());
        let parked =
            c.agent == Agent::Claude && server.is_none() && store.parked.contains_key(&c.name);
        let waits = server.is_none() && store.waits(&c.name, c.agent);
        let present: Vec<Agent> = readings
            .iter()
            .filter(|r| {
                r.server(&c.name).is_some()
                    || (r.agent == Agent::Claude && store.parked.contains_key(&c.name))
                    || store.waits(&c.name, r.agent)
            })
            .map(|r| r.agent)
            .collect();
        // An entry to add: what it needs must be there.
        let check_add = |def: &Def,
                         from: Option<Agent>,
                         keychain: &[String]|
         -> Result<(), &'static str> {
            if !valid_name(&c.name) {
                return Err("badName");
            }
            if c.agent == Agent::Codex && def.is_sse() {
                return Err("sse");
            }
            if missing_command(def, &places.home).is_some_and(|m| m.path) {
                return Err("noCommand");
            }
            resolve(&c.name, def, from, keychain, have, false).map(|_| ()).map_err(|_| "missingKey")
        };
        let op =
            match (&c.to, server) {
                (To::On, Some(s)) if s.enabled => Err("nothing"),
                (To::On, Some(_)) if !editable => Err("shape"),
                (To::On, Some(s)) => {
                    match s.def.as_ref().and_then(|d| missing_command(d, &places.home)) {
                        Some(m) if m.path => Err("noCommand"),
                        _ => Ok(Op::Enable),
                    }
                }
                (To::On, None) => match source(readings, store, &c.name, c.agent) {
                    None => Err("unknown"),
                    Some((def, from, extra, unpark, unstore)) => {
                        check_add(&def, from, &[]).map(|()| Op::Add {
                            def,
                            from,
                            keychain: Vec::new(),
                            enabled: true,
                            extra,
                            unpark,
                            unstore,
                            unwait: waits,
                        })
                    }
                },
                (To::Off, Some(s)) if !s.enabled => Err("nothing"),
                (To::Off, Some(_)) if !editable => Err("shape"),
                (To::Off, Some(_)) if c.agent == Agent::Claude => Ok(Op::Park),
                (To::Off, Some(_)) => Ok(Op::Disable),
                (To::Off, None) => Err("nothing"),
                (To::Absent { .. }, Some(_)) if !editable => Err("shape"),
                (To::Absent { store }, Some(_)) => {
                    Ok(Op::Remove { store: *store, agents: present.clone() })
                }
                (To::Absent { store }, None) if waits => {
                    Ok(Op::Unwait { store: *store, agents: present.clone() })
                }
                (To::Absent { store }, None) if parked => {
                    Ok(Op::Unpark { store: *store, agents: present.clone() })
                }
                (To::Absent { .. }, None) => Err("nothing"),
                (To::Def { .. }, Some(_)) if !editable => Err("shape"),
                (To::Def { def, from, keychain, .. }, Some(_)) => check_add(def, *from, keychain)
                    .map(|()| Op::Replace {
                        def: def.clone(),
                        from: *from,
                        keychain: keychain.clone(),
                    }),
                (To::Def { off: true, .. }, None) if c.agent == Agent::Claude => Err("nothing"),
                (To::Def { def, from, keychain, off }, None) => check_add(def, *from, keychain)
                    .map(|()| Op::Add {
                        def: def.clone(),
                        from: *from,
                        keychain: keychain.clone(),
                        enabled: !off,
                        extra: store
                            .parked
                            .get(&c.name)
                            .filter(|_| parked)
                            .map(|k| k.extra.clone())
                            .unwrap_or_default(),
                        unpark: parked,
                        unstore: store.stored.contains_key(&c.name),
                        unwait: waits,
                    }),
            };
        match op {
            Ok(op) => {
                p.steps.entry(c.agent).or_default().push(Step { name: c.name.clone(), op });
                p.accepted.push(c.clone());
            }
            Err(reason) => refuse(&mut p, reason),
        }
    }
    p
}

// ===== Applying the steps to a file's text =====

/// One file's text after `step`, with `def` the step's definition with
/// its secrets filled in (or masked, for the preview).
fn edit(agent: Agent, text: &str, step: &Step, def: Option<&Def>) -> Result<String, EditError> {
    let name = step.name.as_str();
    let def = || def.ok_or(EditError::Check("definition"));
    match (agent, &step.op) {
        (Agent::Codex, Op::Enable) => codex_mcp::set_enabled(text, name, true),
        (Agent::Codex, Op::Disable) => codex_mcp::set_enabled(text, name, false),
        (Agent::Codex, Op::Add { enabled, .. }) => codex_mcp::add(text, name, def()?, *enabled),
        (Agent::Codex, Op::Remove { .. }) => codex_mcp::remove(text, name),
        (Agent::Codex, Op::Replace { .. }) => codex_mcp::replace(text, name, def()?),
        (Agent::Agy, Op::Enable) => agy_mcp::set_enabled(text, name, true),
        (Agent::Agy, Op::Disable) => agy_mcp::set_enabled(text, name, false),
        (Agent::Agy, Op::Add { enabled, .. }) => agy_mcp::add(text, name, def()?, *enabled),
        (Agent::Agy, Op::Remove { .. }) => agy_mcp::remove(text, name),
        (Agent::Agy, Op::Replace { .. }) => agy_mcp::replace(text, name, def()?),
        _ => Err(EditError::Check("agent")),
    }
}

/// The definition a step writes, filled in or masked.
fn step_def(step: &Step, have: &Have<'_>, values: bool) -> Result<Option<Def>, Vec<String>> {
    match &step.op {
        Op::Add { def, from, keychain, .. } | Op::Replace { def, from, keychain } => {
            resolve(&step.name, def, *from, keychain, have, values).map(Some)
        }
        _ => Ok(None),
    }
}

/// What `def` drops when written for `agent`.
fn dropped(agent: Agent, def: &Def) -> Vec<String> {
    match (agent, def) {
        (Agent::Claude, Def::Stdio { cwd: Some(_), .. }) => vec!["cwd".into()],
        _ => Vec::new(),
    }
}

fn for_agent(agent: Agent, def: &Def) -> Def {
    match (agent, def) {
        (Agent::Claude, Def::Stdio { command, args, env, .. }) => {
            Def::Stdio { command: command.clone(), args: args.clone(), env: env.clone(), cwd: None }
        }
        _ => def.clone(),
    }
}

// ===== The preview =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// `ctx`, `add`, `del`, or `gap` for lines left out.
    pub op: &'static str,
    pub text: String,
    /// The line holds a masked secret.
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub name: String,
    /// `on`, `off`, `add`, `remove` or `replace`.
    pub action: &'static str,
    /// The file's lines that change (Codex, Antigravity).
    pub lines: Vec<Line>,
    /// The commands run (Claude Code), secrets masked.
    pub commands: Vec<String>,
    /// A secret goes from the Keychain or another agent's file.
    pub secret: bool,
    /// What the agent's entry cannot hold.
    pub dropped: Vec<String>,
    /// Where the definition comes from, when copied.
    pub from: Option<Agent>,
    /// The app keeps it (Claude Code's off, a delete into the store).
    pub kept: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub agent: Agent,
    pub file: String,
    /// The backup folder, as `~/…`.
    pub backups: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub groups: Vec<Group>,
    pub refused: Vec<Refused>,
    /// The changes shown, to send back with the write.
    pub changes: Vec<Change>,
    /// What each file was when read, to send back with the write.
    pub bases: BTreeMap<Agent, String>,
}

/// The lines that differ between two texts, with a little around them.
pub fn diff(before: &str, after: &str) -> Vec<Line> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf]
    {
        suf += 1;
    }
    let am = &a[pre..a.len() - suf];
    let bm = &b[pre..b.len() - suf];
    let mut ops: Vec<(&'static str, &str)> = Vec::new();
    if am.len() * bm.len() <= 250_000 {
        // The longest common run of lines, from the end.
        let (n, m) = (am.len(), bm.len());
        let mut t = vec![vec![0u32; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i][j] =
                    if am[i] == bm[j] { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && am[i] == bm[j] {
                ops.push(("ctx", am[i]));
                i += 1;
                j += 1;
            } else if i < n && (j == m || t[i + 1][j] >= t[i][j + 1]) {
                ops.push(("del", am[i]));
                i += 1;
            } else {
                ops.push(("add", bm[j]));
                j += 1;
            }
        }
    } else {
        ops.extend(am.iter().map(|l| ("del", *l)));
        ops.extend(bm.iter().map(|l| ("add", *l)));
    }
    let mut out: Vec<(&'static str, &str)> = Vec::new();
    out.extend(a[pre.saturating_sub(2)..pre].iter().map(|l| ("ctx", *l)));
    // A long run of unchanged lines inside is cut to its ends.
    let mut k = 0;
    while k < ops.len() {
        if ops[k].0 == "ctx" {
            let run = ops[k..].iter().take_while(|o| o.0 == "ctx").count();
            if run > 3 {
                out.push(ops[k]);
                out.push(("gap", ""));
                out.push(ops[k + run - 1]);
            } else {
                out.extend(&ops[k..k + run]);
            }
            k += run;
        } else {
            out.push(ops[k]);
            k += 1;
        }
    }
    let tail = a.len() - suf;
    out.extend(a[tail..(tail + 1).min(a.len())].iter().map(|l| ("ctx", *l)));
    out.into_iter()
        .map(|(op, l)| Line { op, text: l.replace(REDACTED, MASK), secret: l.contains(REDACTED) })
        .collect()
}

fn redact_text(agent: Agent, text: &str) -> Result<String, String> {
    match agent {
        Agent::Codex => redact::toml(text),
        _ => redact::json(text),
    }
}

/// The Claude Code commands of one step, secrets masked.
fn claude_lines(step: &Step, def: Option<&Def>) -> Vec<String> {
    let masked = |d: &Def, extra: &Map<String, Json>| {
        let d = for_agent(Agent::Claude, &d.masked(MASK));
        claude_mcp::add_line(&step.name, &mask_holes(claude_mcp::entry(&d, extra)))
    };
    match (&step.op, def) {
        (Op::Park | Op::Remove { .. }, _) => vec![claude_mcp::remove_line(&step.name)],
        (Op::Add { extra, .. }, Some(d)) => vec![masked(d, extra)],
        (Op::Replace { .. }, Some(d)) => {
            vec![claude_mcp::remove_line(&step.name), masked(d, &Map::new())]
        }
        _ => Vec::new(),
    }
}

/// [`REDACTED`] shown as the mask.
fn mask_holes(v: Json) -> Json {
    match v {
        Json::String(s) => Json::String(s.replace(REDACTED, MASK)),
        Json::Array(a) => Json::Array(a.into_iter().map(mask_holes).collect()),
        Json::Object(o) => Json::Object(o.into_iter().map(|(k, v)| (k, mask_holes(v))).collect()),
        other => other,
    }
}

/// Reads every agent's file now and shows what `changes` would write.
pub fn preview(places: &Places, app: &AppDir, secrets: &Secrets, changes: &[Change]) -> Preview {
    let readings = super::read_all(places);
    let store = kho::load(app);
    let have = Have { readings: &readings, secrets };
    let mut planned = plan(places, &readings, &store, &have, changes);
    let mut groups = Vec::new();
    let mut bases = BTreeMap::new();
    let mut late: Vec<(String, Agent)> = Vec::new();
    for (agent, steps) in &planned.steps {
        let r = readings.iter().find(|r| r.agent == *agent).expect("planned from a reading");
        bases.insert(*agent, r.base.clone());
        let mut text = r.text.clone().unwrap_or_default();
        let mut blocks = Vec::new();
        for step in steps {
            let def = match step_def(step, &have, false) {
                Ok(d) => d.map(|d| for_agent(*agent, &d)),
                Err(_) => {
                    late.push((step.name.clone(), *agent));
                    continue;
                }
            };
            let (lines, commands) = if *agent == Agent::Claude {
                (Vec::new(), claude_lines(step, def.as_ref()))
            } else if matches!(step.op, Op::Unpark { .. } | Op::Unwait { .. }) {
                (Vec::new(), Vec::new())
            } else {
                match edit(*agent, &text, step, def.as_ref()) {
                    Ok(next) => {
                        let before = if text.trim().is_empty() {
                            Ok(String::new())
                        } else {
                            redact_text(*agent, &text)
                        };
                        let shown = match (before, redact_text(*agent, &next)) {
                            (Ok(a), Ok(b)) => diff(&a, &b),
                            _ => Vec::new(),
                        };
                        text = next;
                        (shown, Vec::new())
                    }
                    Err(_) => {
                        late.push((step.name.clone(), *agent));
                        continue;
                    }
                }
            };
            let (def_of, from) = match &step.op {
                Op::Add { def, from, .. } | Op::Replace { def, from, .. } => (Some(def), *from),
                _ => (None, None),
            };
            blocks.push(Block {
                name: step.name.clone(),
                action: action(&step.op),
                lines,
                commands,
                secret: def_of.is_some_and(|d| !holes(d).is_empty()),
                dropped: def_of.map(|d| dropped(*agent, d)).unwrap_or_default(),
                from,
                kept: matches!(
                    step.op,
                    Op::Park
                        | Op::Remove { store: true, .. }
                        | Op::Unpark { store: true, .. }
                        | Op::Unwait { store: true, .. }
                ),
            });
        }
        if !blocks.is_empty() {
            let (dir, _) = agent.backups();
            groups.push(Group {
                agent: *agent,
                file: runner::display_path(places.file(*agent), &places.home),
                backups: runner::display_path(&app.path().join(dir), &places.home),
                blocks,
            });
        }
    }
    // A step that could not be made after all leaves the batch.
    for (name, agent) in late {
        planned.accepted.retain(|c| !(c.name == name && c.agent == agent));
        planned.refused.push(Refused { name, agent, reason: "shape" });
    }
    Preview { groups, refused: planned.refused, changes: planned.accepted, bases }
}

// ===== The write =====

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Failure {
    /// The file is not what the preview read: nothing written to it.
    Changed,
    /// The batch is not the one the preview showed: nothing written.
    Refused {
        refused: Vec<Refused>,
    },
    /// The file could not be edited as planned.
    Edit {
        error: EditError,
    },
    Write {
        error: write::WriteError,
    },
    /// `claude` failed; its words, secrets masked.
    Cli {
        detail: String,
    },
    Keychain {
        detail: String,
    },
    /// After `claude` ran, its list is not what it should be.
    Verify,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub agent: Agent,
    pub changes: usize,
    /// The backup's file name.
    pub backup: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failed {
    pub agent: Agent,
    pub error: Failure,
}

/// The batch that puts back what a write did, and what the files must
/// still be for it to run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Undo {
    pub changes: Vec<Change>,
    pub bases: BTreeMap<Agent, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub done: Vec<Done>,
    pub failed: Option<Failed>,
    pub undo: Option<Undo>,
    /// Unix seconds.
    pub at: i64,
}

/// One line of `agents/mcp-log.jsonl`: what was written, never a value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub at: i64,
    pub agent: Agent,
    /// `name:action` for each change.
    pub changes: Vec<String>,
    pub backup: Option<String>,
    pub result: String,
}

pub const LOG: &str = "agents/mcp-log.jsonl";
const LOG_KEPT: usize = 200;

pub fn log_read(app: &AppDir) -> Vec<LogLine> {
    app.read_to_string(&app.path().join(LOG))
        .map(|t| t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect())
        .unwrap_or_default()
}

fn log_append(app: &AppDir, line: LogLine) {
    let mut lines = log_read(app);
    lines.push(line);
    let skip = lines.len().saturating_sub(LOG_KEPT);
    let text: String = lines[skip..]
        .iter()
        .filter_map(|l| serde_json::to_string(l).ok())
        .map(|l| l + "\n")
        .collect();
    let _ = app.write_private(&app.path().join(LOG), &text);
}

/// Every secret of `def`, to mask in what `claude` prints.
fn values(def: &Def) -> Vec<String> {
    def.slots().iter().filter_map(|s| def.get(&s.id).map(str::to_string)).collect()
}

/// Keeps the server's secrets in the Keychain and the rest in the store.
#[allow(clippy::too_many_arguments)]
fn keep(
    store: &mut Kho,
    app: &AppDir,
    secrets: &Secrets,
    name: &str,
    def: &Def,
    extra: &Map<String, Json>,
    parked: bool,
    agents: &[Agent],
    at: i64,
) -> Result<(), Failure> {
    let (mut kept, values) = Kept::of(def, extra, at);
    for (slot, value) in &values {
        secrets.save(name, slot, value).map_err(|e| Failure::Keychain { detail: e.to_string() })?;
    }
    if parked {
        store.parked.insert(name.to_string(), kept);
    } else {
        if let Some(old) = store.stored.get(name) {
            for a in &old.agents {
                if !kept.agents.contains(a) {
                    kept.agents.push(*a);
                }
            }
        }
        for a in agents {
            if !kept.agents.contains(a) {
                kept.agents.push(*a);
            }
        }
        kept.agents.sort();
        store.stored.insert(name.to_string(), kept);
    }
    kho::save(app, store).map_err(|e| Failure::Keychain { detail: e.to_string() })
}

/// `agent` stops waiting for the restored `name`; with `keep_it` the server
/// goes to the store for the agents it was in.
fn unwait(store: &mut Kho, name: &str, agent: Agent, keep_it: bool, agents: &[Agent]) {
    if keep_it {
        if let Some(k) = store.waiting.get(name) {
            let mut kept = k.clone();
            kept.agents = agents.to_vec();
            store.stored.insert(name.to_string(), kept);
        }
    }
    store.unwait(name, agent);
}

/// What puts one step back.
fn inverse(agent: Agent, step: &Step, old: Option<&Def>, old_enabled: bool) -> Option<Change> {
    let to = match &step.op {
        Op::Enable => To::Off,
        Op::Disable => To::On,
        Op::Park => To::On,
        Op::Add { unpark: true, .. } | Op::Add { unwait: true, .. } => To::Off,
        Op::Add { unstore, .. } => To::Absent { store: *unstore },
        Op::Remove { .. } | Op::Replace { .. } => {
            let old = old?;
            To::Def {
                def: old.stripped(),
                from: None,
                keychain: old.slots().into_iter().map(|s| s.id).collect(),
                off: !old_enabled,
            }
        }
        Op::Unpark { .. } | Op::Unwait { .. } => return None,
    };
    Some(Change { name: step.name.clone(), agent, to })
}

/// Writes `changes`, when every file they touch is still what `bases`
/// says (what the preview read). Codex's and Antigravity's files go first,
/// each in one write; then Claude Code's commands, one per change. The
/// first failure stops the batch there; what was written before it is in
/// the undo.
pub fn write(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    changes: &[Change],
    bases: &BTreeMap<Agent, String>,
    now: OffsetDateTime,
) -> Outcome {
    let at = now.unix_timestamp();
    let readings = super::read_all(places);
    let mut store = kho::load(app);
    let have = Have { readings: &readings, secrets };
    let planned = plan(places, &readings, &store, &have, changes);
    let mut out = Outcome { done: Vec::new(), failed: None, undo: None, at };
    if !planned.refused.is_empty() {
        let agent = planned.refused[0].agent;
        out.failed = Some(Failed { agent, error: Failure::Refused { refused: planned.refused } });
        return out;
    }
    for agent in planned.steps.keys() {
        let r = readings.iter().find(|r| r.agent == *agent).expect("planned from a reading");
        if bases.get(agent) != Some(&r.base) {
            out.failed = Some(Failed { agent: *agent, error: Failure::Changed });
            return out;
        }
    }

    let mut undo: Vec<Change> = Vec::new();
    let order = [Agent::Codex, Agent::Agy, Agent::Claude];
    for agent in order {
        let Some(steps) = planned.steps.get(&agent) else { continue };
        let r = readings.iter().find(|r| r.agent == agent).expect("planned from a reading");
        let result = if agent == Agent::Claude {
            write_claude(places, app, secrets, &mut store, r, steps, &have, now, &mut undo)
        } else {
            write_file(places, app, secrets, &mut store, r, steps, &have, now, &mut undo)
        };
        match result {
            Ok(done) => out.done.push(done),
            Err(error) => {
                out.failed = Some(Failed { agent, error });
                break;
            }
        }
    }
    if !undo.is_empty() {
        let after = super::read_all(places);
        let mut bases = BTreeMap::new();
        for c in &undo {
            if let Some(r) = after.iter().find(|r| r.agent == c.agent) {
                bases.insert(c.agent, r.base.clone());
            }
        }
        undo.reverse();
        out.undo = Some(Undo { changes: undo, bases });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn write_file(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    store: &mut Kho,
    r: &Reading,
    steps: &[Step],
    have: &Have<'_>,
    now: OffsetDateTime,
    undo: &mut Vec<Change>,
) -> Result<Done, Failure> {
    let agent = r.agent;
    let at = now.unix_timestamp();
    let base = r.text.clone();
    let mut text = base.clone().unwrap_or_default();
    let mut inverses = Vec::new();
    let mut after_write: Vec<&Step> = Vec::new();
    for step in steps {
        let def = step_def(step, have, true)
            .map_err(|_| Failure::Refused {
                refused: vec![Refused { name: step.name.clone(), agent, reason: "missingKey" }],
            })?
            .map(|d| for_agent(agent, &d));
        let old = r.server(&step.name);
        if matches!(step.op, Op::Unwait { .. }) {
            after_write.push(step);
            continue;
        }
        if let Op::Remove { store: keep_it, agents } = &step.op {
            if let Some(d) = old.and_then(|s| s.def.as_ref()) {
                if *keep_it {
                    keep(store, app, secrets, &step.name, d, &Map::new(), false, agents, at)?;
                } else {
                    // So that the undo can put it back; a delete does not
                    // stop for it.
                    for s in d.slots() {
                        if let Some(v) = d.get(&s.id) {
                            let _ = secrets.save(&step.name, &s.id, v);
                        }
                    }
                }
            }
        }
        text = edit(agent, &text, step, def.as_ref()).map_err(|error| Failure::Edit { error })?;
        inverses.extend(inverse(
            agent,
            step,
            old.and_then(|s| s.def.as_ref()),
            old.is_some_and(|s| s.enabled),
        ));
        after_write.push(step);
    }
    let file = places.file(agent);
    let (dir, prefix) = agent.backups();
    let backups = app.path().join(dir);
    let clean = match &base {
        Some(b) => {
            Some(redact_text(agent, b).map_err(|e| Failure::Edit { error: EditError::Parse(e) })?)
        }
        None => None,
    };
    let backup =
        clean.as_ref().map(|c| write::Backup { dir: &backups, prefix, bytes: c.as_bytes() });
    // Only the app's store changes (a waiting server dropped): the file
    // stays as it is.
    let untouched = text == base.clone().unwrap_or_default();
    let written = if untouched {
        Ok(None)
    } else {
        write::write(file, base.as_deref().map(str::as_bytes), &text, backup, now)
    };
    let names: Vec<String> =
        steps.iter().map(|s| format!("{}:{}", s.name, action(&s.op))).collect();
    match written {
        Ok(backup) => {
            let backup =
                backup.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
            if agent == Agent::Codex && !untouched {
                let mut record = codex_files::load(app);
                let mcp = crate::codex::config::parse(&text, None)
                    .map(|p| crate::codex::config::mcp_fingerprint(&p.mcp))
                    .unwrap_or_default();
                record.follow(&r.base, &write::sha256(text.as_bytes()), mcp);
                let _ = codex_files::save(app, &record);
            }
            for step in after_write {
                match &step.op {
                    Op::Add { unstore, unwait, .. } if *unstore || *unwait => {
                        if *unstore {
                            store.stored.remove(&step.name);
                        }
                        if *unwait {
                            store.unwait(&step.name, agent);
                        }
                    }
                    Op::Unwait { store: keep_it, agents } => {
                        unwait(store, &step.name, agent, *keep_it, agents);
                    }
                    _ => continue,
                }
                let _ = kho::save(app, store);
            }
            log_append(
                app,
                LogLine {
                    at,
                    agent,
                    changes: names,
                    backup: backup.clone(),
                    result: "written".into(),
                },
            );
            undo.extend(inverses);
            Ok(Done { agent, changes: steps.len(), backup })
        }
        Err(error) => {
            log_append(
                app,
                LogLine { at, agent, changes: names, backup: None, result: "failed".into() },
            );
            Err(match error {
                write::WriteError::Changed => Failure::Changed,
                e => Failure::Write { error: e },
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_claude(
    places: &Places,
    app: &AppDir,
    secrets: &Secrets,
    store: &mut Kho,
    r: &Reading,
    steps: &[Step],
    have: &Have<'_>,
    now: OffsetDateTime,
    undo: &mut Vec<Change>,
) -> Result<Done, Failure> {
    let at = now.unix_timestamp();
    let cli = claude_mcp::Cli {
        program: places.claude_cli.clone().ok_or(Failure::Cli { detail: "no claude".into() })?,
    };
    let entries = r.entries.clone().unwrap_or_default();
    // The list before the first command, secrets left out.
    let (dir, prefix) = Agent::Claude.backups();
    let backup = if r.entries.is_some() {
        let whole = serde_json::to_string_pretty(&Json::Object(Map::from_iter([(
            "mcpServers".to_string(),
            Json::Object(entries.clone()),
        )])))
        .expect("JSON serializes");
        let clean =
            redact::json(&whole).map_err(|e| Failure::Edit { error: EditError::Parse(e) })?;
        let path = write::keep(clean.as_bytes(), &app.path().join(dir), prefix, now)
            .map_err(|e| Failure::Write { error: e.into() })?;
        path.file_name().map(|n| n.to_string_lossy().to_string())
    } else {
        None
    };
    let mut done = 0;
    let mut names = Vec::new();
    let result = (|| -> Result<(), Failure> {
        for step in steps {
            let old = r.server(&step.name);
            let old_def = old.and_then(|s| s.def.clone());
            let old_extra = entries.get(&step.name).map(claude_mcp::extra).unwrap_or_default();
            let cli_err = |e: claude_mcp::CliError| Failure::Cli { detail: e.to_string() };
            match &step.op {
                Op::Park => {
                    let d = old_def.as_ref().ok_or(Failure::Verify)?;
                    keep(store, app, secrets, &step.name, d, &old_extra, true, &[], at)?;
                    cli.remove(&step.name).map_err(cli_err)?;
                }
                Op::Remove { store: keep_it, agents } => {
                    if let Some(d) = &old_def {
                        if *keep_it {
                            keep(
                                store, app, secrets, &step.name, d, &old_extra, false, agents, at,
                            )?;
                        } else {
                            for s in d.slots() {
                                if let Some(v) = d.get(&s.id) {
                                    let _ = secrets.save(&step.name, &s.id, v);
                                }
                            }
                        }
                    }
                    cli.remove(&step.name).map_err(cli_err)?;
                }
                Op::Unwait { store: keep_it, agents } => {
                    unwait(store, &step.name, Agent::Claude, *keep_it, agents);
                    kho::save(app, store)
                        .map_err(|e| Failure::Keychain { detail: e.to_string() })?;
                }
                Op::Unpark { store: keep_it, agents } => {
                    if let Some(mut k) = store.parked.remove(&step.name) {
                        if *keep_it {
                            k.agents = agents.clone();
                            store.stored.insert(step.name.clone(), k);
                        }
                        kho::save(app, store)
                            .map_err(|e| Failure::Keychain { detail: e.to_string() })?;
                    }
                }
                Op::Add { extra, unpark, unstore, unwait, .. } => {
                    let d = step_def(step, have, true)
                        .map_err(|_| Failure::Refused {
                            refused: vec![Refused {
                                name: step.name.clone(),
                                agent: Agent::Claude,
                                reason: "missingKey",
                            }],
                        })?
                        .map(|d| for_agent(Agent::Claude, &d))
                        .ok_or(Failure::Verify)?;
                    cli.add(&step.name, &claude_mcp::entry(&d, extra), &values(&d))
                        .map_err(cli_err)?;
                    if *unpark {
                        store.parked.remove(&step.name);
                    }
                    if *unstore {
                        store.stored.remove(&step.name);
                    }
                    if *unwait {
                        store.unwait(&step.name, Agent::Claude);
                    }
                    if *unpark || *unstore || *unwait {
                        let _ = kho::save(app, store);
                    }
                }
                Op::Replace { .. } => {
                    let d = step_def(step, have, true)
                        .map_err(|_| Failure::Refused {
                            refused: vec![Refused {
                                name: step.name.clone(),
                                agent: Agent::Claude,
                                reason: "missingKey",
                            }],
                        })?
                        .map(|d| for_agent(Agent::Claude, &d))
                        .ok_or(Failure::Verify)?;
                    cli.remove(&step.name).map_err(cli_err)?;
                    if let Err(e) =
                        cli.add(&step.name, &claude_mcp::entry(&d, &old_extra), &values(&d))
                    {
                        // Put the one that was there back.
                        if let (Some(v), Some(od)) = (entries.get(&step.name), &old_def) {
                            let _ = cli.add(&step.name, v, &values(od));
                        }
                        return Err(cli_err(e));
                    }
                }
                Op::Enable | Op::Disable => return Err(Failure::Verify),
            }
            verify(places, step, have)?;
            names.push(format!("{}:{}", step.name, action(&step.op)));
            undo.extend(inverse(Agent::Claude, step, old_def.as_ref(), true));
            done += 1;
        }
        Ok(())
    })();
    let line = LogLine {
        at,
        agent: Agent::Claude,
        changes: names,
        backup: backup.clone(),
        result: if result.is_ok() { "written".into() } else { "failed".into() },
    };
    log_append(app, line);
    result.map(|()| Done { agent: Agent::Claude, changes: done, backup })
}

/// Claude Code's list after one command says what the step meant.
fn verify(places: &Places, step: &Step, have: &Have<'_>) -> Result<(), Failure> {
    if matches!(step.op, Op::Unpark { .. } | Op::Unwait { .. }) {
        return Ok(());
    }
    let entries = claude_mcp::read(&places.claude_json).ok().flatten().unwrap_or_default();
    let now = entries.get(&step.name).and_then(|v| claude_mcp::server(&step.name, v));
    let ok = match &step.op {
        Op::Park | Op::Remove { .. } => now.is_none(),
        Op::Add { .. } | Op::Replace { .. } => {
            let want =
                step_def(step, have, true).ok().flatten().map(|d| for_agent(Agent::Claude, &d));
            now.and_then(|s| s.def).zip(want).is_some_and(|(got, want)| got.same(&want))
        }
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(Failure::Verify)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_diff_shows_the_changed_lines_with_a_little_around() {
        let a = "a\nb\nc\nd\ne\nf\n";
        let b = "a\nb\nc\nX\ne\nf\n";
        let lines = diff(a, b);
        let ops: Vec<(&str, &str)> = lines.iter().map(|l| (l.op, l.text.as_str())).collect();
        assert_eq!(ops, [("ctx", "b"), ("ctx", "c"), ("del", "d"), ("add", "X"), ("ctx", "e")]);
        let added = diff("", "[x]\nk = \"<redacted>\"\n");
        assert_eq!(added.len(), 2);
        assert!(added[1].secret && added[1].text.contains(MASK));
    }

    #[test]
    fn holes_are_the_masked_places() {
        let d = Def::Stdio {
            command: "npx".into(),
            args: vec!["--token".into(), MASK.into(), format!("--key={REDACTED}")],
            env: vec![
                super::super::def::Pair::new("A", MASK),
                super::super::def::Pair::new("B", "x"),
            ],
            cwd: None,
        };
        assert_eq!(holes(&d), ["arg:1", "arg:2", "env:A"]);
    }
}
