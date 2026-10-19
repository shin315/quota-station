//! Codex's MCP list: the `[mcp_servers.<name>]` tables of `config.toml`.
//!
//! An edit touches one server's lines only: `enabled = false` written or
//! removed (Codex skips a server with `enabled = false`, and runs it when
//! the key is not there), a new table added after the last server, or a
//! server's tables (its own and its `[mcp_servers.<name>.env]` and the
//! like) removed. Every other byte stays, and [`check`] reads the result
//! back: everything but the one server must mean what it meant before, and
//! the server what the edit says. A server written inline or with dotted
//! keys is not edited. The provider keys the Codex switch owns
//! (`model_provider`, `requires_openai_auth`) are never touched here.

use std::ops::Range;

use serde_json::Value as Json;
use toml_edit::{Document, DocumentMut, Item, TableLike, Value};

use super::def::{Def, Pair};
use super::{EditError, Server};

/// Keys of a server table the app reads into a [`Def`].
const KNOWN: &[&str] = &["command", "args", "env", "cwd", "url", "http_headers", "enabled"];

fn strings(item: Option<&Item>) -> Vec<String> {
    item.and_then(Item::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn pairs(item: Option<&Item>) -> Vec<Pair> {
    item.and_then(Item::as_table_like)
        .map(|t| t.iter().filter_map(|(k, v)| v.as_str().map(|s| Pair::new(k, s))).collect())
        .unwrap_or_default()
}

fn server(name: &str, item: &Item, parent_plain: bool) -> Option<Server> {
    let t = item.as_table_like()?;
    let str_of = |k: &str| t.get(k).and_then(Item::as_str).map(str::to_string);
    let def = if let Some(command) = str_of("command") {
        Some(Def::Stdio {
            command,
            args: strings(t.get("args")),
            env: pairs(t.get("env")),
            cwd: str_of("cwd"),
        })
    } else {
        str_of("url").map(|url| Def::Http {
            url,
            headers: pairs(t.get("http_headers")),
            sse: false,
        })
    };
    let extra =
        t.iter().map(|(k, _)| k.to_string()).filter(|k| !KNOWN.contains(&k.as_str())).collect();
    let editable = parent_plain && matches!(item, Item::Table(t) if !t.is_dotted());
    Some(Server::new(
        name,
        t.get("enabled").and_then(Item::as_bool).unwrap_or(true),
        def,
        extra,
        editable,
    ))
}

/// The servers in `config.toml`, in the file's order. Only the MCP list is
/// read; the text is not kept.
pub fn read(text: &str) -> Result<Vec<Server>, String> {
    let doc: DocumentMut =
        text.parse().map_err(|e: toml_edit::TomlError| e.message().to_string())?;
    let Some(item) = doc.get("mcp_servers") else { return Ok(Vec::new()) };
    let plain = matches!(item, Item::Table(t) if !t.is_dotted());
    let Some(servers) = item.as_table_like() else { return Ok(Vec::new()) };
    Ok(servers.iter().filter_map(|(name, item)| server(name, item, plain)).collect())
}

// ===== Places in the text =====

/// The start of the line `at` is on.
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Just past the end of the line `at` is on (its newline included).
fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map(|i| at + i + 1).unwrap_or(text.len())
}

/// Every table header in the file: its path and where its line starts.
fn headers(doc: &Document<&str>) -> Vec<(Vec<String>, usize)> {
    fn walk(
        text: &str,
        path: &mut Vec<String>,
        t: &toml_edit::Table,
        out: &mut Vec<(Vec<String>, usize)>,
    ) {
        if let Some(span) = t.span() {
            if !t.is_implicit() && !t.is_dotted() {
                out.push((path.clone(), line_start(text, span.start)));
            }
        }
        for (k, item) in t.iter() {
            path.push(k.to_string());
            match item {
                Item::Table(sub) => walk(text, path, sub, out),
                Item::ArrayOfTables(a) => {
                    for sub in a.iter() {
                        walk(text, path, sub, out);
                    }
                }
                _ => {}
            }
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(doc.raw(), &mut Vec::new(), doc.as_table(), &mut out);
    out.sort_by_key(|(_, at)| *at);
    out
}

/// The comment lines just above `at` (a header's line start) belong to it.
fn before_comments(text: &str, mut at: usize) -> usize {
    while at > 0 {
        let start = line_start(text, at - 1);
        if text[start..at].trim_start().starts_with('#') {
            at = start;
        } else {
            break;
        }
    }
    at
}

/// Each header's own lines: from its line to the next header's comments.
fn regions(text: &str, heads: &[(Vec<String>, usize)]) -> Vec<(Vec<String>, Range<usize>)> {
    heads
        .iter()
        .enumerate()
        .map(|(i, (path, at))| {
            let end = heads
                .get(i + 1)
                .map(|(_, next)| before_comments(text, *next))
                .unwrap_or(text.len());
            (path.clone(), *at..end.max(*at))
        })
        .collect()
}

fn server_path(name: &str) -> Vec<String> {
    vec!["mcp_servers".to_string(), name.to_string()]
}

/// The server's table, when it is one the app edits.
fn table<'d>(doc: &'d Document<&str>, name: &str) -> Result<&'d toml_edit::Table, EditError> {
    let parent = doc.as_table().get("mcp_servers").ok_or(EditError::Missing)?;
    let Item::Table(parent) = parent else { return Err(EditError::Shape) };
    if parent.is_dotted() {
        return Err(EditError::Shape);
    }
    match parent.get(name) {
        None => Err(EditError::Missing),
        Some(Item::Table(t)) if !t.is_dotted() && t.span().is_some() => Ok(t),
        Some(_) => Err(EditError::Shape),
    }
}

fn parse(text: &str) -> Result<Document<&str>, EditError> {
    Document::parse(text).map_err(|e| EditError::Parse(e.message().to_string()))
}

// ===== Writing a server =====

/// A key as TOML writes it: bare when it can be.
fn key(k: &str) -> String {
    let bare =
        !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if bare {
        k.to_string()
    } else {
        quoted(k)
    }
}

fn quoted(s: &str) -> String {
    Value::from(s).to_string().trim().to_string()
}

fn inline(pairs: &[Pair]) -> String {
    let inner: Vec<String> =
        pairs.iter().map(|p| format!("{} = {}", key(&p.key), quoted(&p.value))).collect();
    format!("{{ {} }}", inner.join(", "))
}

/// The server's table as text, `extra` lines (other keys kept from the
/// table it replaces) after the app's own.
fn block(name: &str, def: &Def, enabled: bool, extra: &[String]) -> String {
    let mut out = format!("[mcp_servers.{}]\n", key(name));
    match def {
        Def::Stdio { command, args, env, cwd } => {
            out.push_str(&format!("command = {}\n", quoted(command)));
            if !args.is_empty() {
                let a: Vec<String> = args.iter().map(|a| quoted(a)).collect();
                out.push_str(&format!("args = [{}]\n", a.join(", ")));
            }
            if !env.is_empty() {
                out.push_str(&format!("env = {}\n", inline(env)));
            }
            if let Some(cwd) = cwd {
                out.push_str(&format!("cwd = {}\n", quoted(cwd)));
            }
        }
        Def::Http { url, headers, .. } => {
            out.push_str(&format!("url = {}\n", quoted(url)));
            if !headers.is_empty() {
                out.push_str(&format!("http_headers = {}\n", inline(headers)));
            }
        }
    }
    for line in extra {
        out.push_str(line);
        out.push('\n');
    }
    if !enabled {
        out.push_str("enabled = false\n");
    }
    out
}

/// The other keys of a server's table as `key = value` lines, so a
/// replaced server keeps them.
fn extra_lines(t: &toml_edit::Table) -> Vec<String> {
    t.iter()
        .filter(|(k, _)| !KNOWN.contains(k))
        .filter_map(|(k, item)| {
            let v = item.clone().into_value().ok()?;
            Some(format!("{} = {}", key(k), v.decorated("", "")))
        })
        .collect()
}

fn splice(text: &str, at: Range<usize>, new: &str) -> String {
    let mut out = text.to_string();
    out.replace_range(at, new);
    out
}

/// What an edit does, for [`check`].
#[derive(Debug, Clone)]
pub enum Edit<'a> {
    Enabled(bool),
    Remove,
    Put { def: &'a Def, enabled: bool },
}

/// Turns the server on (`enabled = false` removed) or off (written).
pub fn set_enabled(text: &str, name: &str, on: bool) -> Result<String, EditError> {
    let doc = parse(text)?;
    let t = table(&doc, name)?;
    let out = match (t.get_key_value("enabled"), on) {
        (Some((_, item)), true) if item.as_bool() == Some(true) => text.to_string(),
        (Some((k, item)), true) => {
            let start = line_start(text, k.span().ok_or(EditError::Shape)?.start);
            let end = line_end(text, item.span().ok_or(EditError::Shape)?.end);
            splice(text, start..end, "")
        }
        (Some((_, item)), false) => {
            let span = item.span().ok_or(EditError::Shape)?;
            splice(text, span, "false")
        }
        (None, true) => text.to_string(),
        (None, false) => {
            // After the table's last line of its own, else after its header.
            let last = t
                .iter()
                .filter(|(_, item)| !item.is_table())
                .filter_map(|(k, item)| {
                    t.get_key_value(k)?.0.span()?;
                    item.span().map(|s| s.end)
                })
                .max();
            let after = match last {
                Some(end) => line_end(text, end),
                None => line_end(text, t.span().ok_or(EditError::Shape)?.end),
            };
            let nl = if after > 0 && !text[..after].ends_with('\n') { "\n" } else { "" };
            splice(text, after..after, &format!("{nl}enabled = false\n"))
        }
    };
    check(text, &out, name, &Edit::Enabled(on))?;
    Ok(out)
}

/// Removes the server's tables.
pub fn remove(text: &str, name: &str) -> Result<String, EditError> {
    let doc = parse(text)?;
    table(&doc, name)?;
    let path = server_path(name);
    let mut cut: Vec<Range<usize>> = regions(text, &headers(&doc))
        .into_iter()
        .filter(|(p, _)| p.starts_with(&path))
        .map(|(_, r)| r)
        .collect();
    cut.sort_by_key(|r| std::cmp::Reverse(r.start));
    let mut out = text.to_string();
    for r in cut {
        out.replace_range(r, "");
    }
    check(text, &out, name, &Edit::Remove)?;
    Ok(out)
}

/// Adds the server after the last one, or at the end of the file.
pub fn add(text: &str, name: &str, def: &Def, enabled: bool) -> Result<String, EditError> {
    if def.is_sse() {
        return Err(EditError::Unsupported);
    }
    let doc = parse(text)?;
    match table(&doc, name) {
        Err(EditError::Missing) => {}
        Ok(_) => return Err(EditError::Exists),
        Err(e) => return Err(e),
    }
    if doc
        .as_table()
        .get("mcp_servers")
        .is_some_and(|i| !matches!(i, Item::Table(t) if !t.is_dotted()))
    {
        return Err(EditError::Shape);
    }
    let new = block(name, def, enabled, &[]);
    let heads = headers(&doc);
    let last = regions(text, &heads)
        .into_iter()
        .filter(|(p, _)| p.first().is_some_and(|f| f == "mcp_servers"))
        .map(|(_, r)| r.end)
        .max();
    let out = match last {
        Some(at) if at < text.len() => splice(text, at..at, &format!("{new}\n")),
        _ => {
            let mut out = text.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(&new);
            out
        }
    };
    check(text, &out, name, &Edit::Put { def, enabled })?;
    Ok(out)
}

/// Replaces the server with `def`, keeping whether it is on and its other
/// keys (timeouts, tool lists, variable names).
pub fn replace(text: &str, name: &str, def: &Def) -> Result<String, EditError> {
    if def.is_sse() {
        return Err(EditError::Unsupported);
    }
    let doc = parse(text)?;
    let t = table(&doc, name)?;
    let enabled = t.get("enabled").and_then(Item::as_bool).unwrap_or(true);
    let extra = extra_lines(t);
    let path = server_path(name);
    let own: Vec<Range<usize>> = regions(text, &headers(&doc))
        .into_iter()
        .filter(|(p, _)| p.starts_with(&path))
        .map(|(_, r)| r)
        .collect();
    let first = own.iter().map(|r| r.start).min().ok_or(EditError::Shape)?;
    let mut out = text.to_string();
    let mut cut = own.clone();
    cut.sort_by_key(|r| std::cmp::Reverse(r.start));
    // What followed the server's own lines (a blank line, often) stays.
    let tail = own.iter().find(|r| r.start == first).map(|r| {
        let body = &text[r.clone()];
        body.len() - body.trim_end_matches(['\n', ' ', '\t']).len()
    });
    for r in cut {
        out.replace_range(r, "");
    }
    let mut new = block(name, def, enabled, &extra);
    if let Some(n) = tail {
        if n > 1 {
            new.push_str(&"\n".repeat(n - 1));
        }
    }
    out.insert_str(first, &new);
    check(text, &out, name, &Edit::Put { def, enabled })?;
    Ok(out)
}

// ===== The check =====

fn json_of_value(v: &Value) -> Json {
    match v {
        Value::String(s) => Json::String(s.value().clone()),
        Value::Integer(i) => Json::from(*i.value()),
        Value::Float(f) => Json::String(f.value().to_string()),
        Value::Boolean(b) => Json::Bool(*b.value()),
        Value::Datetime(d) => Json::String(d.value().to_string()),
        Value::Array(a) => Json::Array(a.iter().map(json_of_value).collect()),
        Value::InlineTable(t) => {
            Json::Object(t.iter().map(|(k, v)| (k.to_string(), json_of_value(v))).collect())
        }
    }
}

/// What a TOML item means, as JSON, to compare two readings.
pub fn json_of(item: &Item) -> Json {
    match item {
        Item::None => Json::Null,
        Item::Value(v) => json_of_value(v),
        Item::Table(t) => json_of_table(t),
        Item::ArrayOfTables(a) => Json::Array(a.iter().map(|t| json_of_table(t)).collect()),
    }
}

fn json_of_table(t: &dyn TableLike) -> Json {
    Json::Object(t.iter().map(|(k, v)| (k.to_string(), json_of(v))).collect())
}

/// The file read back after an edit of `name`: every other key means what
/// it meant, and the server is what the edit says.
pub fn check(before: &str, after: &str, name: &str, edit: &Edit<'_>) -> Result<(), EditError> {
    let a: DocumentMut = before.parse().map_err(|_| EditError::Check("parse"))?;
    let b: DocumentMut = after.parse().map_err(|_| EditError::Check("parse"))?;
    let rest = |d: &DocumentMut| {
        let Json::Object(mut all) = json_of_table(d.as_table()) else { unreachable!() };
        let server =
            all.get_mut("mcp_servers").and_then(Json::as_object_mut).and_then(|s| s.remove(name));
        (all, server)
    };
    let (rest_a, server_a) = rest(&a);
    let (mut rest_b, server_b) = rest(&b);
    // A first server makes `mcp_servers` appear; that is the server's part.
    if !rest_a.contains_key("mcp_servers")
        && rest_b.get("mcp_servers").and_then(Json::as_object).is_some_and(|m| m.is_empty())
    {
        rest_b.remove("mcp_servers");
    }
    if rest_a != rest_b {
        return Err(EditError::Check("otherKeys"));
    }
    let ok = match edit {
        Edit::Remove => server_b.is_none(),
        Edit::Enabled(on) => match (server_a, server_b) {
            (Some(Json::Object(mut x)), Some(Json::Object(mut y))) => {
                let got = y.remove("enabled").map(|v| v.as_bool() == Some(true)).unwrap_or(true);
                x.remove("enabled");
                x == y && got == *on
            }
            _ => false,
        },
        Edit::Put { def, enabled } => {
            let doc: DocumentMut = after.parse().map_err(|_| EditError::Check("parse"))?;
            read(&doc.to_string())
                .ok()
                .and_then(|s| s.into_iter().find(|s| s.name == name))
                .is_some_and(|s| {
                    s.enabled == *enabled && s.def.as_ref().is_some_and(|d| d.same(def))
                })
        }
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

    const FILE: &str = r#"model = "gpt-5.5"

[model_providers.custom]
experimental_bearer_token = "test-key-not-real"

[mcp_servers]

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp"]

[mcp_servers.files.env]
FILES_TOKEN = "test-token-not-real"

# the docs server
[mcp_servers.docs]
url = "https://docs.example.com/mcp"
enabled = false
startup_timeout_sec = 20

[projects."/work/app"]
trust_level = "trusted"
"#;

    #[test]
    fn reads_the_servers_and_their_shape() {
        let servers = read(FILE).unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name, "files");
        let Some(Def::Stdio { env, .. }) = &servers[0].def else { panic!() };
        assert_eq!(env[0].key, "FILES_TOKEN");
        assert!(servers[0].enabled && servers[0].editable);
        assert!(!servers[1].enabled);
        assert_eq!(servers[1].extra, ["startup_timeout_sec"]);
    }

    #[test]
    fn turning_off_and_on_touches_one_line() {
        let off = set_enabled(FILE, "files", false).unwrap();
        assert_eq!(off, FILE.replace("files-mcp\"]\n", "files-mcp\"]\nenabled = false\n"));
        let on = set_enabled(FILE, "docs", true).unwrap();
        assert_eq!(on, FILE.replace("enabled = false\n", ""));
        assert_eq!(set_enabled(&off, "files", true).unwrap(), FILE);
    }

    #[test]
    fn removing_takes_the_server_s_tables_and_keeps_the_next_comment() {
        let out = remove(FILE, "files").unwrap();
        assert!(!out.contains("files"), "{out}");
        assert!(out.contains("[mcp_servers]\n\n# the docs server\n[mcp_servers.docs]"), "{out}");
        assert!(out.contains("experimental_bearer_token"));
    }

    #[test]
    fn adding_goes_after_the_last_server() {
        let def = Def::Http {
            url: "https://tracker.example.com/mcp".into(),
            headers: vec![Pair::new("Authorization", "Bearer x")],
            sse: false,
        };
        let out = add(FILE, "tracker", &def, true).unwrap();
        assert!(
            out.contains("startup_timeout_sec = 20\n\n[mcp_servers.tracker]\nurl = \"https://tracker.example.com/mcp\"\nhttp_headers = { Authorization = \"Bearer x\" }\n\n[projects"),
            "{out}"
        );
        assert_eq!(add(FILE, "files", &def, true).unwrap_err(), EditError::Exists);
        let empty = add("", "tracker", &def, false).unwrap();
        assert_eq!(empty, "[mcp_servers.tracker]\nurl = \"https://tracker.example.com/mcp\"\nhttp_headers = { Authorization = \"Bearer x\" }\nenabled = false\n");
    }

    #[test]
    fn replacing_keeps_on_or_off_and_other_keys() {
        let def =
            Def::Http { url: "https://docs2.example.com/mcp".into(), headers: vec![], sse: false };
        let out = replace(FILE, "docs", &def).unwrap();
        assert!(out.contains("[mcp_servers.docs]\nurl = \"https://docs2.example.com/mcp\"\nstartup_timeout_sec = 20\nenabled = false\n\n[projects"), "{out}");
    }

    #[test]
    fn inline_and_dotted_servers_are_not_edited() {
        let text =
            "[mcp_servers]\nfiles = { command = \"npx\" }\ndocs.url = \"https://d.example.com\"\n";
        assert_eq!(set_enabled(text, "files", false).unwrap_err(), EditError::Shape);
        assert_eq!(remove(text, "docs").unwrap_err(), EditError::Shape);
        assert!(!read(text).unwrap()[0].editable);
    }
}
