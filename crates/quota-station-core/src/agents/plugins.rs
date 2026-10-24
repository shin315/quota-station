// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The plugins of Claude Code and Codex (M11, E), read only: what each has
//! installed for the user, whether it is on, which marketplace it came
//! from, and what it brings (MCP servers, skills, commands, hooks, agents).
//! The list (`name@marketplace`, the marketplace's source, on or off) goes
//! into the backup pack; a plugin's files, and the servers and skills it
//! brings, do not.
//!
//! Read by asking each agent's own command: `claude plugin list --json`
//! and `claude plugin marketplace list --json`, `codex plugin list --json`.
//! The Plugin tab never installs, removes, turns on or off a plugin; it
//! shows the commands a restore runs ([`reinstall`]). Only a restore (M12)
//! runs them, and its undo the matching removals ([`Step`]), each through
//! the agent's own command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Agent;
use crate::runner::{self, RunError};

const TIMEOUT: Duration = Duration::from_secs(30);
/// Installing fetches the marketplace and the plugin.
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(180);

/// What a plugin brings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Brings {
    pub mcp: usize,
    pub skills: Vec<String>,
    pub commands: usize,
    pub hooks: bool,
    pub agents: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    /// `name@marketplace`.
    pub id: String,
    pub enabled: bool,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub brings: Brings,
}

impl Plugin {
    pub fn marketplace(&self) -> &str {
        self.id.rsplit_once('@').map(|(_, m)| m).unwrap_or("")
    }
}

/// Where a marketplace comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marketplace {
    pub name: String,
    /// `github`, `git`, `local` or `builtin` (the agent's own).
    pub kind: String,
    /// `owner/repo`, a Git address without credentials, or `None`.
    #[serde(default)]
    pub source: Option<String>,
}

/// One agent's plugins as read now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Side {
    pub agent: Agent,
    /// `ok`, `noCli` (the command is not on this Mac) or `failed`.
    pub state: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
    #[serde(default)]
    pub marketplaces: Vec<Marketplace>,
}

impl Side {
    fn failed(agent: Agent, state: &str, detail: Option<String>) -> Self {
        Side { agent, state: state.into(), detail, plugins: Vec::new(), marketplaces: Vec::new() }
    }

    pub fn ok(&self) -> bool {
        self.state == "ok"
    }
}

/// A Git address with any user and password left out.
pub fn clean_url(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(mut u) if u.has_host() => {
            let _ = u.set_username("");
            let _ = u.set_password(None);
            u.set_query(None);
            u.set_fragment(None);
            u.to_string()
        }
        _ => String::new(),
    }
}

/// `owner/repo` for a GitHub address.
fn github_repo(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    if u.host_str()? != "github.com" {
        return None;
    }
    let path = u.path().trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    super::skills::repo_ok(path).then(|| path.to_string())
}

fn count(dir: &Path, ext: &str) -> usize {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(ext))
        .count()
}

/// What the plugin in `dir` brings, from its folders' names and its MCP
/// list's keys. No file in it is run, and no value kept.
pub fn brings(dir: &Path, manifest: &str) -> Brings {
    let mut b = Brings::default();
    let skills = dir.join("skills");
    let mut names: Vec<String> = std::fs::read_dir(&skills)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(super::skills::SKILL_MD).is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    b.skills = names;
    b.commands = count(&dir.join("commands"), ".md");
    b.agents = count(&dir.join("agents"), ".md");
    let read = |p: PathBuf| -> Option<Json> {
        let text = std::fs::read_to_string(p).ok()?;
        (text.len() < 1024 * 1024).then(|| serde_json::from_str(&text).ok()).flatten()
    };
    let plugin = read(dir.join(manifest).join("plugin.json"));
    let mcp = read(dir.join(".mcp.json"));
    b.mcp = mcp
        .as_ref()
        .and_then(|m| m.get("mcpServers").or(Some(m)))
        .and_then(Json::as_object)
        .map(|o| o.len())
        .or_else(|| plugin.as_ref()?.get("mcpServers")?.as_object().map(|o| o.len()))
        .unwrap_or(0);
    b.hooks = dir.join("hooks/hooks.json").is_file()
        || plugin.as_ref().is_some_and(|p| p.get("hooks").is_some());
    b
}

/// A plugin folder the app reads only under the home folder.
fn plugin_dir(path: &str, home: &Path) -> Option<PathBuf> {
    let p = PathBuf::from(path);
    (p.is_absolute() && p.starts_with(home) && !path.contains("/../") && p.is_dir()).then_some(p)
}

fn run(program: &Path, args: &[&str]) -> Result<Json, (String, Option<String>)> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let env = [("DISABLE_AUTOUPDATER", std::ffi::OsStr::new("1"))];
    let out = runner::run_with(program, &args, &env, None, TIMEOUT).map_err(|e| match e {
        RunError::NotFound(_) => ("noCli".to_string(), None),
        other => ("failed".to_string(), Some(other.to_string())),
    })?;
    if out.code != Some(0) {
        return Err(("failed".into(), Some(runner::first_line(&out.stderr))));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| ("failed".to_string(), Some(e.to_string())))
}

fn text(v: &Json, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::to_string)
}

/// Claude Code's plugins installed for the user, and its marketplaces.
pub fn read_claude(claude: Option<&Path>, home: &Path) -> Side {
    let Some(claude) = claude else { return Side::failed(Agent::Claude, "noCli", None) };
    let list = match run(claude, &["plugin", "list", "--json"]) {
        Ok(v) => v,
        Err((s, d)) => return Side::failed(Agent::Claude, &s, d),
    };
    let markets = run(claude, &["plugin", "marketplace", "list", "--json"]).unwrap_or(Json::Null);
    let mut plugins: Vec<Plugin> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p.get("scope").and_then(Json::as_str) == Some("user"))
        .filter_map(|p| {
            let id = text(p, "id")?;
            let dir = text(p, "installPath").and_then(|d| plugin_dir(&d, home));
            Some(Plugin {
                id,
                enabled: p.get("enabled").and_then(Json::as_bool).unwrap_or(false),
                version: text(p, "version"),
                brings: dir.map(|d| brings(&d, ".claude-plugin")).unwrap_or_default(),
            })
        })
        .collect();
    plugins.sort_by(|a, b| a.id.cmp(&b.id));
    let mut marketplaces: Vec<Marketplace> = markets
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let name = text(m, "name")?;
            let kind = text(m, "source").unwrap_or_default();
            let (kind, source) = match kind.as_str() {
                "github" => ("github", text(m, "repo").filter(|r| super::skills::repo_ok(r))),
                "git" | "url" => {
                    ("git", text(m, "url").map(|u| clean_url(&u)).filter(|u| !u.is_empty()))
                }
                "directory" | "file" | "local" => ("local", None),
                _ => ("other", None),
            };
            Some(Marketplace { name, kind: kind.into(), source })
        })
        .collect();
    marketplaces.sort_by(|a, b| a.name.cmp(&b.name));
    Side { agent: Agent::Claude, state: "ok".into(), detail: None, plugins, marketplaces }
}

/// Codex's plugins installed, and the marketplaces they came from.
pub fn read_codex(codex: Option<&Path>, home: &Path) -> Side {
    let Some(codex) = codex else { return Side::failed(Agent::Codex, "noCli", None) };
    let list = match run(codex, &["plugin", "list", "--json"]) {
        Ok(v) => v,
        Err((s, d)) => return Side::failed(Agent::Codex, &s, d),
    };
    let mut plugins = Vec::new();
    let mut markets: BTreeMap<String, Marketplace> = BTreeMap::new();
    for p in list.get("installed").and_then(Json::as_array).into_iter().flatten() {
        let Some(id) = text(p, "pluginId") else { continue };
        if p.get("installed").and_then(Json::as_bool) == Some(false) {
            continue;
        }
        let dir = p
            .get("source")
            .filter(|s| s.get("source").and_then(Json::as_str) == Some("local"))
            .and_then(|s| text(s, "path"))
            .and_then(|d| plugin_dir(&d, home));
        let market = text(p, "marketplaceName")
            .unwrap_or_else(|| id.rsplit_once('@').map(|(_, m)| m.to_string()).unwrap_or_default());
        let ms = p.get("marketplaceSource");
        let kind = ms.and_then(|m| text(m, "sourceType")).unwrap_or_default();
        let (kind, source) = match kind.as_str() {
            "git" => {
                let url = ms.and_then(|m| text(m, "source")).unwrap_or_default();
                match github_repo(&url) {
                    Some(r) => ("github", Some(r)),
                    None => ("git", Some(clean_url(&url)).filter(|u| !u.is_empty())),
                }
            }
            // The agent's own marketplaces: bundled with the app, or its remote one.
            _ => ("builtin", None),
        };
        markets.entry(market.clone()).or_insert(Marketplace {
            name: market,
            kind: kind.into(),
            source,
        });
        plugins.push(Plugin {
            id,
            enabled: p.get("enabled").and_then(Json::as_bool).unwrap_or(false),
            version: text(p, "version"),
            brings: dir.map(|d| brings(&d, ".codex-plugin")).unwrap_or_default(),
        });
    }
    plugins.sort_by(|a, b| a.id.cmp(&b.id));
    Side {
        agent: Agent::Codex,
        state: "ok".into(),
        detail: None,
        plugins,
        marketplaces: markets.into_values().collect(),
    }
}

/// One command of the agent's own that a restore (or its undo) runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub agent: Agent,
    /// After `claude` or `codex`.
    pub args: Vec<String>,
}

impl Step {
    fn new(agent: Agent, args: &[&str]) -> Self {
        Step { agent, args: args.iter().map(|a| a.to_string()).collect() }
    }

    /// The command as the Plugin tab shows it.
    pub fn line(&self) -> String {
        format!("{} {}", cli(self.agent), self.args.join(" "))
    }
}

fn cli(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "claude",
        _ => "codex",
    }
}

/// A plugin of the agent's own marketplace: it comes with the agent (Codex
/// App's bundled ones), nothing installs it.
pub fn builtin(side: &Side, p: &Plugin) -> bool {
    side.agent == Agent::Codex
        && side.marketplaces.iter().any(|m| m.name == p.marketplace() && m.kind == "builtin")
}

/// Adding the marketplace `name` from where `side` had it, when it can be:
/// from GitHub or a Git address.
pub fn market_step(side: &Side, name: &str) -> Option<Step> {
    let m = side.marketplaces.iter().find(|m| m.name == name)?;
    let src = m.source.as_deref().filter(|_| m.kind == "github" || m.kind == "git")?;
    Some(Step::new(side.agent, &["plugin", "marketplace", "add", src]))
}

/// Installing `p` as it was: Claude Code's turned off again after, when it
/// was off. Codex has no command to turn one off.
pub fn install_steps(agent: Agent, p: &Plugin) -> Vec<Step> {
    match agent {
        Agent::Claude => {
            let mut out = vec![Step::new(agent, &["plugin", "install", &p.id])];
            if !p.enabled {
                out.push(Step::new(agent, &["plugin", "disable", &p.id]));
            }
            out
        }
        _ => vec![Step::new(agent, &["plugin", "add", &p.id])],
    }
}

/// What takes a plugin the restore installed away again.
pub fn uninstall_step(agent: Agent, id: &str) -> Step {
    match agent {
        Agent::Claude => Step::new(agent, &["plugin", "uninstall", id]),
        _ => Step::new(agent, &["plugin", "remove", id]),
    }
}

/// What takes a marketplace the restore added away again (Claude Code's).
pub fn market_remove_step(name: &str) -> Step {
    Step::new(Agent::Claude, &["plugin", "marketplace", "remove", name])
}

/// What a restore would run to get the plugins back, in order: each
/// marketplace added, each plugin installed, those off turned off. The
/// agent's own marketplaces come with the agent.
pub fn reinstall(side: &Side) -> Vec<String> {
    let mut used: Vec<&str> = Vec::new();
    for p in side.plugins.iter().filter(|p| !builtin(side, p)) {
        if !used.contains(&p.marketplace()) {
            used.push(p.marketplace());
        }
    }
    let mut out = Vec::new();
    for m in side.marketplaces.iter().filter(|m| used.contains(&m.name.as_str())) {
        if let Some(step) = market_step(side, &m.name) {
            out.push(step.line());
        }
    }
    for p in side.plugins.iter().filter(|p| !builtin(side, p)) {
        out.extend(install_steps(side.agent, p).iter().map(Step::line));
    }
    out
}

/// Runs `step` with the agent's command `program`: the arguments as a list,
/// no shell, nothing asked (no terminal to answer). The error is the
/// command's first line.
pub fn run_step(program: &Path, step: &Step) -> Result<(), String> {
    let env = [("DISABLE_AUTOUPDATER", std::ffi::OsStr::new("1"))];
    let out = runner::run_with(program, &step.args, &env, None, INSTALL_TIMEOUT)
        .map_err(|e| e.to_string())?;
    if out.code == Some(0) {
        return Ok(());
    }
    let err = runner::first_line(&out.stderr);
    Err(if err.is_empty() { runner::first_line(&out.stdout) } else { err })
}

/// The skills plugins bring, by plugin: shown in the Skills tab, never
/// changed there.
pub fn owned_skills(sides: &[Side]) -> Vec<super::skills::Owned> {
    sides
        .iter()
        .flat_map(|s| {
            s.plugins.iter().filter(|p| !p.brings.skills.is_empty()).map(|p| super::skills::Owned {
                what: p.id.clone(),
                agent: s.agent,
                names: p.brings.skills.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_lose_their_credentials() {
        assert_eq!(
            clean_url("https://user:pw@git.example.com/a/b.git?x=1"),
            "https://git.example.com/a/b.git"
        );
        assert_eq!(
            github_repo("https://github.com/acme/plugins.git").as_deref(),
            Some("acme/plugins")
        );
        assert_eq!(github_repo("https://git.example.com/acme/plugins.git"), None);
    }

    #[test]
    fn the_commands_a_restore_would_run() {
        let side = Side {
            agent: Agent::Claude,
            state: "ok".into(),
            detail: None,
            plugins: vec![
                Plugin {
                    id: "issue-hub@acme-plugins".into(),
                    enabled: true,
                    version: None,
                    brings: Brings::default(),
                },
                Plugin {
                    id: "review-kit@acme-plugins".into(),
                    enabled: false,
                    version: None,
                    brings: Brings::default(),
                },
            ],
            marketplaces: vec![
                Marketplace {
                    name: "acme-plugins".into(),
                    kind: "github".into(),
                    source: Some("acme/claude-plugins".into()),
                },
                Marketplace {
                    name: "unused".into(),
                    kind: "github".into(),
                    source: Some("x/y".into()),
                },
            ],
        };
        assert_eq!(
            reinstall(&side),
            [
                "claude plugin marketplace add acme/claude-plugins",
                "claude plugin install issue-hub@acme-plugins",
                "claude plugin install review-kit@acme-plugins",
                "claude plugin disable review-kit@acme-plugins",
            ]
        );
    }
}
