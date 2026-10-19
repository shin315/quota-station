//! "Dán JSON hoặc TOML" in the add form (M9): the MCP servers in a snippet
//! the owner pastes, read the way each agent reads its own file:
//! `{"mcpServers": {…}}` (Claude Code, Antigravity and most READMEs),
//! `{"servers": {…}}`, the same map without either, one `"name": {…}`
//! member on its own, or Codex's `[mcp_servers.<name>]` tables.
//!
//! The windows get each definition with its secrets masked. With a
//! Keychain the secrets go there first, under the server's name, and the
//! add takes them from it; the text itself is never kept.

use serde::Serialize;
use serde_json::{Map, Value as Json};

use super::def::{Def, Slot, MASK};
use super::secrets::Secrets;
use super::{agy_mcp, claude_mcp, codex_mcp, valid_name};

/// A paste far larger than any MCP list is not read.
const MAX: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pasted {
    pub name: String,
    /// Every secret is [`MASK`].
    pub def: Def,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paste {
    pub servers: Vec<Pasted>,
    /// Names the app does not write, or entries with nothing to run.
    pub skipped: Vec<String>,
    /// `empty`, `tooLong`, `parse`, `noName`, `none` or `keychain`: nothing
    /// is offered.
    pub error: Option<&'static str>,
    /// What the parser said, for `parse`.
    pub detail: Option<String>,
}

fn is_entry(v: &Json) -> bool {
    v.as_object()
        .is_some_and(|o| ["command", "url", "serverUrl"].iter().any(|k| o.contains_key(*k)))
}

fn entry(name: &str, v: &Json) -> Option<Def> {
    let server = if v.get("serverUrl").is_some() {
        agy_mcp::server(name, v)
    } else {
        claude_mcp::server(name, v)
    };
    server.and_then(|s| s.def)
}

fn from_json(root: &Json) -> Result<Vec<(String, Option<Def>)>, &'static str> {
    let Some(o) = root.as_object() else { return Err("none") };
    if is_entry(root) {
        return Err("noName");
    }
    let map: &Map<String, Json> = match o.get("mcpServers").or_else(|| o.get("servers")) {
        Some(Json::Object(m)) => m,
        Some(_) => return Err("none"),
        None => o,
    };
    let mut out: Vec<(String, Option<Def>)> =
        map.iter().filter(|(_, v)| v.is_object()).map(|(k, v)| (k.clone(), entry(k, v))).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Each server's name and definition (none: nothing to run), or why there
/// are none and what the parser said.
type Found = Result<Vec<(String, Option<Def>)>, (&'static str, Option<String>)>;

/// The servers in `text`, their secrets in clear: in memory only.
fn servers(text: &str) -> Found {
    let t = text.trim();
    if t.is_empty() {
        return Err(("empty", None));
    }
    if t.len() > MAX {
        return Err(("tooLong", None));
    }
    if t.starts_with('{') || t.starts_with('"') {
        let parsed = serde_json::from_str::<Json>(t)
            // `"name": { … }` copied out of a larger file.
            .or_else(|e| serde_json::from_str::<Json>(&format!("{{{t}}}")).map_err(|_| e));
        return match parsed {
            Ok(root) => from_json(&root).map_err(|e| (e, None)),
            Err(e) => {
                Err(("parse", Some(format!("JSON: line {}, column {}", e.line(), e.column()))))
            }
        };
    }
    let list = codex_mcp::read(t).map_err(|e| ("parse", Some(format!("TOML: {e}"))))?;
    Ok(list.into_iter().map(|s| (s.name, s.def)).collect())
}

/// What the add form offers for `text`. With `secrets`, every secret is
/// saved to the Keychain first; one that cannot be saved offers nothing.
pub fn paste(text: &str, secrets: Option<&Secrets>) -> Paste {
    let found = match servers(text) {
        Ok(f) => f,
        Err((error, detail)) => return Paste { error: Some(error), detail, ..Paste::default() },
    };
    let mut out = Paste::default();
    for (name, def) in found {
        let Some(def) = def.filter(|_| valid_name(&name)) else {
            out.skipped.push(name);
            continue;
        };
        let slots = def.slots();
        if let Some(keychain) = secrets {
            for slot in &slots {
                let value = def.get(&slot.id).unwrap_or_default();
                if keychain.save(&name, &slot.id, value).is_err() {
                    return Paste { error: Some("keychain"), ..Paste::default() };
                }
            }
        }
        out.servers.push(Pasted { def: def.masked(MASK), slots, name });
    }
    if out.servers.is_empty() && out.skipped.is_empty() {
        out.error = Some("none");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAP: &str = "sk-paste-trap-0123456789";

    fn names(p: &Paste) -> Vec<&str> {
        p.servers.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn reads_every_shape_an_mcp_list_comes_in() {
        let wrapped = format!(
            r#"{{"mcpServers": {{"ticket-desk": {{"command": "npx", "args": ["-y", "ticket-desk-mcp"], "env": {{"TICKET_DESK_KEY": "{TRAP}"}}}}, "docs": {{"type": "http", "url": "https://docs.example.com/mcp"}}}}}}"#
        );
        let p = paste(&wrapped, None);
        assert_eq!(names(&p), ["docs", "ticket-desk"]);
        assert_eq!(p.error, None);

        let vscode = r#"{"servers": {"notes": {"type": "stdio", "command": "notes-mcp"}}}"#;
        assert_eq!(names(&paste(vscode, None)), ["notes"]);

        let bare = r#"{"notes": {"command": "notes-mcp"}, "web": {"serverUrl": "https://web.example.com/mcp"}}"#;
        let p = paste(bare, None);
        assert_eq!(names(&p), ["notes", "web"]);
        assert!(p.servers[1].def.is_http());

        let member = r#""notes": { "command": "notes-mcp", "args": ["--quiet"] }"#;
        assert_eq!(names(&paste(member, None)), ["notes"]);

        let toml = "[mcp_servers.tracker]\ncommand = \"tracker-mcp\"\n\n[mcp_servers.tracker.env]\nTRACKER_TOKEN = \"ghp_trackertrap0123456789\"\n";
        let p = paste(toml, None);
        assert_eq!(names(&p), ["tracker"]);
        assert_eq!(p.servers[0].slots[0].id, "env:TRACKER_TOKEN");
    }

    #[test]
    fn the_windows_get_the_definition_masked() {
        let text = format!(
            r#"{{"mcpServers": {{"ticket-desk": {{"command": "ticket-desk-mcp", "env": {{"TICKET_DESK_KEY": "{TRAP}", "LOG_LEVEL": "info"}}}}}}}}"#
        );
        let p = paste(&text, None);
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains(TRAP), "{json}");
        assert!(json.contains(MASK));
        assert!(json.contains("info"));
        assert_eq!(p.servers[0].slots.len(), 1);
    }

    #[test]
    fn says_why_nothing_is_offered() {
        assert_eq!(paste("  ", None).error, Some("empty"));
        assert_eq!(paste(r#"{"command": "notes-mcp"}"#, None).error, Some("noName"));
        assert_eq!(paste(r#"{"mcpServers": {}}"#, None).error, Some("none"));
        let broken = paste(r#"{"mcpServers": {"a": }"#, None);
        assert_eq!(broken.error, Some("parse"));
        assert!(broken.detail.unwrap().starts_with("JSON: line 1"));
        assert_eq!(paste("[mcp_servers.a\ncommand = 1", None).error, Some("parse"));
        assert_eq!(paste(&"x".repeat(MAX + 1), None).error, Some("tooLong"));
        // A name `claude mcp` does not take, and an entry with nothing to run.
        let p = paste(
            r#"{"mcpServers": {"two words": {"command": "x"}, "empty": {"note": "y"}}}"#,
            None,
        );
        assert!(p.servers.is_empty());
        assert_eq!(p.skipped, ["empty", "two words"]);
        assert_eq!(p.error, None);
    }
}
