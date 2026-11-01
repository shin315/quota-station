// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Antigravity's MCP list: `mcpServers` in `~/.gemini/config/mcp_config.json`,
//! which Antigravity.app, its IDE and `agy` read. A server is a `command`
//! with `args` and `env`, or a `serverUrl` with `headers`; `"disabled": true`
//! turns it off.
//!
//! An edit replaces the bytes of one member only ([`super::json_text`]):
//! `"disabled": true` written or removed, a server added after the last one
//! or removed. [`check`] reads the result back: everything but the one
//! server means what it meant before, and the server is what the edit says.

use serde_json::{Map, Value as Json};

use super::def::{Def, Pair};
use super::json_text::{self, Edit as Splice, Kind, Node, Out};
use super::{EditError, Server};

const KNOWN: &[&str] =
    &["command", "args", "env", "cwd", "serverUrl", "url", "headers", "disabled"];

fn strings(v: Option<&Json>) -> Vec<String> {
    v.and_then(Json::as_array)
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn pairs(v: Option<&Json>) -> Vec<Pair> {
    v.and_then(Json::as_object)
        .map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| Pair::new(k, s))).collect())
        .unwrap_or_default()
}

/// One server's entry as a [`Server`].
pub(crate) fn server(name: &str, v: &Json) -> Option<Server> {
    let o = v.as_object()?;
    let str_of = |k: &str| o.get(k).and_then(Json::as_str).map(str::to_string);
    let def = if let Some(command) = str_of("command") {
        Some(Def::Stdio {
            command,
            args: strings(o.get("args")),
            env: pairs(o.get("env")),
            cwd: str_of("cwd"),
        })
    } else {
        str_of("serverUrl").or_else(|| str_of("url")).map(|url| Def::Http {
            url,
            headers: pairs(o.get("headers")),
            sse: false,
        })
    };
    let extra = o.keys().filter(|k| !KNOWN.contains(&k.as_str())).cloned().collect();
    let enabled = !o.get("disabled").and_then(Json::as_bool).unwrap_or(false);
    Some(Server::new(name, enabled, def, extra, true))
}

/// The servers in the file, in the file's order. The text is not kept.
pub fn read(text: &str) -> Result<Vec<Server>, String> {
    let root = json_text::parse(text)?;
    if !root.is_object() {
        return Err("not a JSON object".into());
    }
    let Some(servers) = root.get("mcpServers").map(|m| &m.value) else { return Ok(Vec::new()) };
    if !servers.is_object() {
        return Err("mcpServers is not an object".into());
    }
    let whole: Json = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let values = whole.get("mcpServers").and_then(Json::as_object).cloned().unwrap_or_default();
    // In the file's order, which `serde_json` does not keep.
    Ok(servers
        .members()
        .iter()
        .filter_map(|m| values.get(&m.key).and_then(|v| server(&m.key, v)))
        .collect())
}

/// What an empty file holds before its first server.
pub const EMPTY: &str = "{\n  \"mcpServers\": {}\n}\n";

fn parse(text: &str) -> Result<Node, EditError> {
    let root = json_text::parse(text).map_err(EditError::Parse)?;
    if !root.is_object() {
        return Err(EditError::Shape);
    }
    Ok(root)
}

fn servers(root: &Node) -> Result<&Node, EditError> {
    let m = root.get("mcpServers").ok_or(EditError::Missing)?;
    if !m.value.is_object() {
        return Err(EditError::Shape);
    }
    Ok(&m.value)
}

/// The server's member and its index among the servers.
fn find<'n>(servers: &'n Node, name: &str) -> Result<(usize, &'n Node), EditError> {
    let members = servers.members();
    if members.iter().filter(|m| m.key == name).count() > 1 {
        return Err(EditError::Shape);
    }
    let i = members.iter().position(|m| m.key == name).ok_or(EditError::Missing)?;
    let v = &members[i].value;
    if !v.is_object() {
        return Err(EditError::Shape);
    }
    Ok((i, v))
}

/// The entry Antigravity reads, laid out like the rest of the file.
fn entry(def: &Def, enabled: bool, extra: Vec<(String, Out)>) -> Out {
    let obj = |pairs: &[Pair]| {
        Out::Obj(pairs.iter().map(|p| (p.key.clone(), Out::Str(p.value.clone()))).collect())
    };
    let mut m: Vec<(String, Out)> = Vec::new();
    match def {
        Def::Stdio { command, args, env, cwd } => {
            m.push(("command".into(), Out::Str(command.clone())));
            if !args.is_empty() {
                m.push(("args".into(), Out::Strs(args.clone())));
            }
            if !env.is_empty() {
                m.push(("env".into(), obj(env)));
            }
            if let Some(cwd) = cwd {
                m.push(("cwd".into(), Out::Str(cwd.clone())));
            }
        }
        Def::Http { url, headers, .. } => {
            m.push(("serverUrl".into(), Out::Str(url.clone())));
            if !headers.is_empty() {
                m.push(("headers".into(), obj(headers)));
            }
        }
    }
    m.extend(extra);
    if !enabled {
        m.push(("disabled".into(), Out::Bool(true)));
    }
    Out::Obj(m)
}

/// The other members of an entry, kept as they are written when it is
/// replaced.
fn extra_members(text: &str, node: &Node) -> Vec<(String, String)> {
    node.members()
        .iter()
        .filter(|m| !KNOWN.contains(&m.key.as_str()))
        .map(|m| (m.key.clone(), text[m.value.span.clone()].to_string()))
        .collect()
}

/// What an edit does, for [`check`].
#[derive(Debug, Clone)]
pub enum Edit<'a> {
    Enabled(bool),
    Remove,
    Put { def: &'a Def, enabled: bool },
}

/// Turns the server on (`"disabled"` removed) or off (`"disabled": true`).
pub fn set_enabled(text: &str, name: &str, on: bool) -> Result<String, EditError> {
    let root = parse(text)?;
    let (_, node) = find(servers(&root)?, name)?;
    let disabled = node.members().iter().position(|m| m.key == "disabled");
    let splice = match (disabled, on) {
        (Some(i), true) => Some(json_text::remove_member(node, i)),
        (Some(i), false) => {
            let v = &node.members()[i].value;
            (v.kind != Kind::Bool(true)).then(|| Splice { at: v.span.clone(), text: "true".into() })
        }
        (None, true) => None,
        (None, false) => {
            let unit = json_text::unit(text, &root);
            Some(json_text::add_member(text, node, "disabled", "true", &unit))
        }
    };
    let out = match splice {
        Some(s) => json_text::apply(text, vec![s]),
        None => text.to_string(),
    };
    check(text, &out, name, &Edit::Enabled(on))?;
    Ok(out)
}

/// Removes the server's member.
pub fn remove(text: &str, name: &str) -> Result<String, EditError> {
    let root = parse(text)?;
    let servers = servers(&root)?;
    let (i, _) = find(servers, name)?;
    let out = json_text::apply(text, vec![json_text::remove_member(servers, i)]);
    check(text, &out, name, &Edit::Remove)?;
    Ok(out)
}

/// Adds the server after the last one. An empty `text` is a missing file.
pub fn add(text: &str, name: &str, def: &Def, enabled: bool) -> Result<String, EditError> {
    let text_in = if text.trim().is_empty() { EMPTY } else { text };
    let root = parse(text_in)?;
    let unit = json_text::unit(text_in, &root);
    let out = match root.get("mcpServers") {
        Some(m) => {
            if !m.value.is_object() {
                return Err(EditError::Shape);
            }
            match find(&m.value, name) {
                Err(EditError::Missing) => {}
                Ok(_) => return Err(EditError::Exists),
                Err(e) => return Err(e),
            }
            let indent = format!("{unit}{unit}");
            let value = json_text::layout(&entry(def, enabled, Vec::new()), &indent, &unit);
            json_text::apply(
                text_in,
                vec![json_text::add_member(text_in, &m.value, name, &value, &unit)],
            )
        }
        None => {
            let indent = format!("{unit}{unit}");
            let value = json_text::layout(&entry(def, enabled, Vec::new()), &indent, &unit);
            let quoted = serde_json::to_string(name).expect("a string serializes");
            let servers = format!("{{\n{indent}{quoted}: {value}\n{unit}}}");
            json_text::apply(
                text_in,
                vec![json_text::add_member(text_in, &root, "mcpServers", &servers, &unit)],
            )
        }
    };
    check(
        if text.trim().is_empty() { "{}" } else { text },
        &out,
        name,
        &Edit::Put { def, enabled },
    )?;
    Ok(out)
}

/// Replaces the server with `def`, keeping whether it is on and its other
/// members.
pub fn replace(text: &str, name: &str, def: &Def) -> Result<String, EditError> {
    let root = parse(text)?;
    let unit = json_text::unit(text, &root);
    let (_, node) = find(servers(&root)?, name)?;
    let enabled =
        !node.members().iter().any(|m| m.key == "disabled" && m.value.kind == Kind::Bool(true));
    let indent = json_text::indent_at(text, line_start_first(text, node.span.start))
        .unwrap_or(&unit)
        .to_string();
    let extra: Vec<(String, Out)> =
        extra_members(text, node).into_iter().map(|(k, raw)| (k, Out::Raw(raw))).collect();
    let value = json_text::layout(&entry(def, enabled, extra), &indent, &unit);
    let out = json_text::apply(text, vec![Splice { at: node.span.clone(), text: value }]);
    check(text, &out, name, &Edit::Put { def, enabled })?;
    Ok(out)
}

/// The first non-space byte of the line `at` is on.
fn line_start_first(text: &str, at: usize) -> usize {
    let start = text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    start + text[start..].len() - text[start..].trim_start_matches([' ', '\t']).len()
}

/// The file read back after an edit of `name`.
pub fn check(before: &str, after: &str, name: &str, edit: &Edit<'_>) -> Result<(), EditError> {
    let a: Json = serde_json::from_str(before).map_err(|_| EditError::Check("parse"))?;
    let b: Json = serde_json::from_str(after).map_err(|_| EditError::Check("parse"))?;
    let split = |v: Json| -> Option<(Map<String, Json>, Option<Json>)> {
        let Json::Object(mut all) = v else { return None };
        let server =
            all.get_mut("mcpServers").and_then(Json::as_object_mut).and_then(|s| s.remove(name));
        Some((all, server))
    };
    let (rest_a, server_a) = split(a).ok_or(EditError::Check("parse"))?;
    let (mut rest_b, server_b) = split(b).ok_or(EditError::Check("parse"))?;
    if !rest_a.contains_key("mcpServers")
        && rest_b.get("mcpServers").and_then(Json::as_object).is_some_and(|m| m.is_empty())
    {
        rest_b.remove("mcpServers");
    }
    if rest_a != rest_b {
        return Err(EditError::Check("otherKeys"));
    }
    let ok = match edit {
        Edit::Remove => server_b.is_none(),
        Edit::Enabled(on) => match (server_a, server_b) {
            (Some(Json::Object(mut x)), Some(Json::Object(mut y))) => {
                let off = y.remove("disabled").and_then(|v| v.as_bool()).unwrap_or(false);
                x.remove("disabled");
                x == y && off != *on
            }
            _ => false,
        },
        Edit::Put { def, enabled } => server_b
            .as_ref()
            .and_then(|v| server(name, v))
            .is_some_and(|s| s.enabled == *enabled && s.def.as_ref().is_some_and(|d| d.same(def))),
    };
    if ok {
        Ok(())
    } else {
        Err(EditError::Check("server"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{
  "mcpServers": {
    "files": {
      "command": "npx",
      "args": ["-y", "files-mcp"],
      "env": { "FILES_TOKEN": "test-token-not-real" }
    },
    "docs": {
      "disabled": true,
      "serverUrl": "https://docs.example.com/mcp",
      "timeout": 30
    }
  }
}
"#;

    #[test]
    fn reads_in_the_file_s_order() {
        let s = read(FILE).unwrap();
        assert_eq!(s.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["files", "docs"]);
        assert!(s[0].enabled && !s[1].enabled);
        assert_eq!(s[1].extra, ["timeout"]);
    }

    #[test]
    fn turning_off_and_on_touches_one_member() {
        let off = set_enabled(FILE, "files", false).unwrap();
        assert_eq!(
            off,
            FILE.replace(
                "\"test-token-not-real\" }\n",
                "\"test-token-not-real\" },\n      \"disabled\": true\n"
            )
        );
        let on = set_enabled(FILE, "docs", true).unwrap();
        assert_eq!(on, FILE.replace("\"disabled\": true,\n      ", ""));
        assert_eq!(set_enabled(&off, "files", true).unwrap(), FILE);
    }

    #[test]
    fn adding_and_removing() {
        let def = Def::Stdio { command: "notes-mcp".into(), args: vec![], env: vec![], cwd: None };
        let out = add(FILE, "notes", &def, true).unwrap();
        assert!(out.contains("      \"timeout\": 30\n    },\n    \"notes\": {\n      \"command\": \"notes-mcp\"\n    }\n  }\n}\n"), "{out}");
        assert_eq!(remove(&out, "notes").unwrap(), FILE);
        let fresh = add("", "notes", &def, false).unwrap();
        assert_eq!(fresh, "{\n  \"mcpServers\": {\n    \"notes\": {\n      \"command\": \"notes-mcp\",\n      \"disabled\": true\n    }\n  }\n}\n");
        let bare = add("{}", "notes", &def, true).unwrap();
        serde_json::from_str::<Json>(&bare).unwrap();
        assert_eq!(read(&bare).unwrap().len(), 1);
    }

    #[test]
    fn replacing_keeps_off_and_other_members() {
        let def =
            Def::Http { url: "https://docs2.example.com/mcp".into(), headers: vec![], sse: false };
        let out = replace(FILE, "docs", &def).unwrap();
        assert!(out.contains("\"docs\": {\n      \"serverUrl\": \"https://docs2.example.com/mcp\",\n      \"timeout\": 30,\n      \"disabled\": true\n    }"), "{out}");
    }
}
