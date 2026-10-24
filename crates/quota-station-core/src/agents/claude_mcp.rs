// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Claude Code's MCP list for every project: the top-level `mcpServers` of
//! `~/.claude.json`. The app reads that key only (the rest of the file,
//! which Claude Code rewrites all the time, is skipped as it is parsed and
//! never kept), and never writes the file: Claude Code does, through
//! `claude mcp add-json <name> <json> --scope user` and
//! `claude mcp remove <name> --scope user`.
//!
//! Claude Code has no "off" for a server. The app turns one off by keeping
//! its entry in its own store ([`super::kho`], secrets in the Keychain) and
//! removing it; turning it on adds it back.
//!
//! The JSON given to `add-json` holds the server's secrets, on `claude`'s
//! command line for the moment it runs, which other processes of the same
//! macOS user can see (the same limit as `security`'s `-w`). What `claude`
//! prints is shown only with those secrets masked ([`scrub`]).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Map, Value as Json};

use super::def::{Def, Pair, MASK};
use super::Server;
use crate::runner::{self, RunError};

pub const FILE: &str = ".claude.json";
/// `~/.claude.json` keeps every project's history and can grow to a few
/// megabytes; one far larger is not read.
const MAX_FILE: u64 = 64 * 1024 * 1024;
/// Each `claude mcp` run.
pub const TIMEOUT: Duration = Duration::from_secs(30);

const KNOWN: &[&str] = &["type", "command", "args", "env", "url", "headers"];

#[derive(Deserialize)]
struct Top {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: Option<Map<String, Json>>,
}

/// The user list as Claude Code wrote it; `None` when the file is not
/// there. Holds secrets: kept only while a write runs.
pub fn read(file: &Path) -> Result<Option<Map<String, Json>>, String> {
    use std::io::Read;
    let f = match std::fs::File::open(file) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    f.take(MAX_FILE + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("larger than 64 MB".into());
    }
    let top: Top = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(Some(top.mcp_servers.unwrap_or_default()))
}

/// What a write compares: the list's SHA-256, whatever else in the file
/// changed.
pub fn hash(entries: &Map<String, Json>) -> String {
    super::write::sha256(serde_json::to_string(entries).expect("JSON serializes").as_bytes())
}

/// One entry as a [`Server`].
pub fn server(name: &str, v: &Json) -> Option<Server> {
    let o = v.as_object()?;
    let str_of = |k: &str| o.get(k).and_then(Json::as_str).map(str::to_string);
    let strings = |k: &str| -> Vec<String> {
        o.get(k)
            .and_then(Json::as_array)
            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    let pairs = |k: &str| -> Vec<Pair> {
        o.get(k)
            .and_then(Json::as_object)
            .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| Pair::new(k, s))).collect())
            .unwrap_or_default()
    };
    let kind = str_of("type");
    let def = match (kind.as_deref(), str_of("command"), str_of("url")) {
        (None | Some("stdio"), Some(command), _) => {
            Some(Def::Stdio { command, args: strings("args"), env: pairs("env"), cwd: None })
        }
        (Some("http"), _, Some(url)) | (None, None, Some(url)) => {
            Some(Def::Http { url, headers: pairs("headers"), sse: false })
        }
        (Some("sse"), _, Some(url)) => {
            Some(Def::Http { url, headers: pairs("headers"), sse: true })
        }
        _ => None,
    };
    let extra = o.keys().filter(|k| !KNOWN.contains(&k.as_str())).cloned().collect();
    Some(Server::new(name, true, def, extra, true))
}

/// The servers, by name.
pub fn servers(entries: &Map<String, Json>) -> Vec<Server> {
    entries.iter().filter_map(|(name, v)| server(name, v)).collect()
}

/// The JSON `add-json` takes for `def`, with `extra` (members Claude Code
/// had and the app does not read) kept.
pub fn entry(def: &Def, extra: &Map<String, Json>) -> Json {
    let obj = |pairs: &[Pair]| -> Json {
        Json::Object(pairs.iter().map(|p| (p.key.clone(), Json::String(p.value.clone()))).collect())
    };
    let mut m = extra.clone();
    match def {
        Def::Stdio { command, args, env, .. } => {
            m.insert("type".into(), "stdio".into());
            m.insert("command".into(), command.clone().into());
            if !args.is_empty() {
                m.insert("args".into(), args.clone().into());
            }
            if !env.is_empty() {
                m.insert("env".into(), obj(env));
            }
        }
        Def::Http { url, headers, sse } => {
            m.insert("type".into(), if *sse { "sse" } else { "http" }.into());
            m.insert("url".into(), url.clone().into());
            if !headers.is_empty() {
                m.insert("headers".into(), obj(headers));
            }
        }
    }
    Json::Object(m)
}

/// The members of an entry the app does not read.
pub fn extra(v: &Json) -> Map<String, Json> {
    v.as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, _)| !KNOWN.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// The commands as the preview shows them, secrets masked.
pub fn add_line(name: &str, masked: &Json) -> String {
    let json = serde_json::to_string(masked).expect("JSON serializes");
    format!("claude mcp add-json {name} '{}' --scope user", json.replace('\'', "'\\''"))
}

pub fn remove_line(name: &str) -> String {
    format!("claude mcp remove {name} --scope user")
}

/// Where `claude` is usually installed, in the order the app tries them.
pub fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut out = vec![home.join(".local/bin/claude"), home.join(".claude/local/claude")];
    out.extend(
        runner::bin_dirs(home)
            .into_iter()
            .map(|d| d.join("claude"))
            .filter(|p| !p.starts_with(home.join(".local/bin"))),
    );
    out.dedup();
    out
}

pub fn discover(home: &Path) -> Option<PathBuf> {
    candidates(home).into_iter().find(|p| p.is_file())
}

/// Every secret in `text` replaced by the mask.
pub fn scrub(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    let mut s: Vec<&String> = secrets.iter().filter(|s| s.len() >= 4).collect();
    // The longest first, so a secret that contains another goes whole.
    s.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for secret in s {
        out = out.replace(secret.as_str(), MASK);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Timeout,
    Spawn(String),
    /// `claude` failed: its exit code and first line, secrets masked.
    Failed {
        code: Option<i32>,
        message: String,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Timeout => write!(f, "claude did not answer within {}s", TIMEOUT.as_secs()),
            CliError::Spawn(e) => write!(f, "could not run claude: {e}"),
            CliError::Failed { code: Some(c), message } => {
                write!(f, "claude exited {c}: {message}")
            }
            CliError::Failed { code: None, message } => write!(f, "claude was stopped: {message}"),
        }
    }
}

/// The `claude` command line tool.
#[derive(Debug, Clone)]
pub struct Cli {
    pub program: PathBuf,
}

impl Cli {
    fn run(&self, args: &[String], secrets: &[String]) -> Result<(), CliError> {
        // No update check while it runs for the app.
        let env = [("DISABLE_AUTOUPDATER", std::ffi::OsStr::new("1"))];
        let out =
            runner::run_with(&self.program, args, &env, None, TIMEOUT).map_err(|e| match e {
                RunError::Timeout(_) => CliError::Timeout,
                other => CliError::Spawn(scrub(&other.to_string(), secrets)),
            })?;
        match out.code {
            Some(0) => Ok(()),
            code => {
                let first = runner::first_line(&out.stderr);
                let first = if first.is_empty() { runner::first_line(&out.stdout) } else { first };
                Err(CliError::Failed { code, message: scrub(&first, secrets) })
            }
        }
    }

    /// `claude mcp add-json <name> <json> --scope user`.
    pub fn add(&self, name: &str, entry: &Json, secrets: &[String]) -> Result<(), CliError> {
        let json = serde_json::to_string(entry).expect("JSON serializes");
        let args = ["mcp", "add-json", name, &json, "--scope", "user"].map(str::to_string);
        self.run(&args, secrets)
    }

    /// `claude mcp remove <name> --scope user`.
    pub fn remove(&self, name: &str) -> Result<(), CliError> {
        let args = ["mcp", "remove", name, "--scope", "user"].map(str::to_string);
        self.run(&args, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_read_as_servers() {
        let v: Json = serde_json::json!({
            "a": { "type": "stdio", "command": "npx", "args": ["-y", "a-mcp"], "env": {} },
            "b": { "type": "http", "url": "https://b.example.com/mcp", "headers": { "Authorization": "Bearer x" } },
            "c": { "command": "c-mcp", "alwaysLoad": true },
            "d": { "type": "sse", "url": "https://d.example.com/sse" }
        });
        let s = servers(v.as_object().unwrap());
        assert_eq!(s.len(), 4);
        assert!(matches!(&s[1].def, Some(Def::Http { sse: false, .. })));
        assert_eq!(s[2].extra, ["alwaysLoad"]);
        assert!(s[3].def.as_ref().unwrap().is_sse());
    }

    #[test]
    fn scrubbing_masks_every_secret() {
        let out =
            scrub("bad JSON near \"sk-test-abc\" and abc", &["sk-test-abc".into(), "abc".into()]);
        assert_eq!(out, "bad JSON near \"••••••\" and abc");
    }
}
