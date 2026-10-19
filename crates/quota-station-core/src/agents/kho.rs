//! The app's store of MCP servers that are in no agent's file: those turned
//! off in Claude Code (which has no off, so the app removed them), and
//! those put away from every agent ("Tắt ở mọi agent, giữ trong kho"). The
//! file holds each server's definition with its secrets left out, the slots
//! the secrets go back to, and the Bitwarden item to look in for each; the
//! secrets themselves are in the Keychain ([`super::secrets`]).

use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

use super::def::Def;
use super::{redact, Agent};
use crate::antigravity::fs::AppDir;

/// In the app's folder.
pub const FILE: &str = "agents/mcp-store.json";

/// A server kept by the app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kept {
    /// Every secret is [`REDACTED`].
    pub def: Def,
    /// Where the secrets go back: slot ids, in [`Def::slots`]'s order.
    #[serde(default)]
    pub slots: Vec<String>,
    /// Claude Code's members the app does not read, kept when they hold no
    /// secret.
    #[serde(default)]
    pub extra: Map<String, Json>,
    /// The agents it was in when it was put away.
    #[serde(default)]
    pub agents: Vec<Agent>,
    /// Unix seconds.
    pub at: i64,
}

impl Kept {
    /// What the store keeps of `def`, and the secrets to put in the
    /// Keychain first, by slot.
    pub fn of(def: &Def, extra: &Map<String, Json>, at: i64) -> (Kept, Vec<(String, String)>) {
        let slots = def.slots();
        let secrets = slots
            .iter()
            .filter_map(|s| def.get(&s.id).map(|v| (s.id.clone(), v.to_string())))
            .collect();
        // A member holding a secret is not kept: it would come back empty.
        let extra = extra
            .iter()
            .filter(|(k, v)| {
                let one = Json::Object(Map::from_iter([((*k).clone(), (*v).clone())]));
                let text = serde_json::to_string(&one).expect("JSON serializes");
                redact::json(&text).is_ok_and(|clean| clean == text)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let kept = Kept {
            def: def.stripped(),
            slots: slots.into_iter().map(|s| s.id).collect(),
            extra,
            agents: Vec::new(),
            at,
        };
        (kept, secrets)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kho {
    /// Turned off in Claude Code, by name.
    #[serde(default)]
    pub parked: BTreeMap<String, Kept>,
    /// Put away from every agent, by name.
    #[serde(default)]
    pub stored: BTreeMap<String, Kept>,
    /// The Bitwarden item to look in for a secret.
    #[serde(default)]
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    pub mcp: String,
    pub slot: String,
    pub bitwarden: String,
}

impl Kho {
    pub fn hint(&self, mcp: &str, slot: &str) -> Option<&str> {
        self.hints.iter().find(|h| h.mcp == mcp && h.slot == slot).map(|h| h.bitwarden.as_str())
    }

    pub fn set_hint(&mut self, mcp: &str, slot: &str, bitwarden: &str) {
        self.hints.retain(|h| !(h.mcp == mcp && h.slot == slot));
        if !bitwarden.trim().is_empty() {
            self.hints.push(Hint {
                mcp: mcp.into(),
                slot: slot.into(),
                bitwarden: bitwarden.trim().into(),
            });
        }
    }
}

pub fn path(app: &AppDir) -> PathBuf {
    app.path().join(FILE)
}

pub fn load(app: &AppDir) -> Kho {
    app.read_to_string(&path(app))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppDir, kho: &Kho) -> io::Result<()> {
    let text = serde_json::to_string_pretty(kho).map_err(io::Error::other)?;
    // Nothing secret may reach the file, whatever a caller did.
    if redact::json(&text).is_ok_and(|clean| clean == text) {
        app.write_private(&path(app), &text)
    } else {
        Err(io::Error::other("a secret would be kept in the store"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::def::Pair;

    #[test]
    fn the_store_keeps_no_secret() {
        let def = Def::Stdio {
            command: "npx".into(),
            args: vec![
                "-y".into(),
                "files-mcp".into(),
                "--api-key".into(),
                "test-arg-not-real".into(),
            ],
            env: vec![Pair::new("FILES_TOKEN", "test-token-not-real"), Pair::new("MODE", "x")],
            cwd: None,
        };
        let extra = Map::from_iter([
            ("alwaysLoad".to_string(), Json::Bool(true)),
            ("oauth".to_string(), serde_json::json!({ "clientSecret": "test-oauth-not-real" })),
        ]);
        let (kept, secrets) = Kept::of(&def, &extra, 1);
        assert_eq!(kept.slots, ["arg:3", "env:FILES_TOKEN"]);
        assert_eq!(secrets.len(), 2);
        assert_eq!(kept.extra.keys().collect::<Vec<_>>(), ["alwaysLoad"]);
        let text = serde_json::to_string(&kept).unwrap();
        assert!(!text.contains("not-real"), "{text}");

        let dir = std::env::temp_dir().join(format!("qs-kho-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let app = AppDir::new(&dir);
        let mut kho = Kho::default();
        kho.parked.insert("files".into(), kept);
        kho.set_hint("files", "env:FILES_TOKEN", "Files token");
        save(&app, &kho).unwrap();
        assert_eq!(load(&app), kho);
        let (raw, _) = Kept::of(&def, &Map::new(), 1);
        let mut leaky = raw.clone();
        leaky.def = def.clone();
        kho.stored.insert("leaky".into(), leaky);
        assert!(save(&app, &kho).is_err(), "a definition with its secrets is refused");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
