//! An agent file with its secrets left out, for the backups the app keeps
//! before every write and for the preview. Every string is looked at where
//! it sits: under a secret's name ([`def::secret_value`]), an address that
//! carries one, an argument after a secret flag, or anything shaped like a
//! token. Each is replaced by [`def::REDACTED`] and every other byte stays.
//! The names that hold a variable's name rather than its value
//! (`env_key`, `bearer_token_env_var`, `env_vars`, `env_http_headers`) are
//! kept.

use std::ops::Range;

use toml_edit::{Document, Item, Value};

use super::def::{self, REDACTED};
use super::json_text::{self, Kind, Node};

/// Keys whose value names a variable, or a setting, not a secret.
const NAME_KEYS: &[&str] = &[
    "env_key",
    "bearer_token_env_var",
    "env_vars",
    "env_http_headers",
    "cli_auth_credentials_store",
    "forced_login_method",
    "preferred_auth_method",
    "requires_openai_auth",
    "mcp_oauth_credentials_store",
];

fn quoted() -> String {
    format!("\"{REDACTED}\"")
}

/// What a string under `key` becomes, when it is a secret.
fn string(key: &str, value: &str) -> Option<String> {
    if NAME_KEYS.contains(&key) {
        return None;
    }
    if def::secret_url(value) && value.contains("://") {
        let def = def::Def::Http { url: value.into(), headers: vec![], sse: false };
        let masked = def.masked(REDACTED);
        let def::Def::Http { url, .. } = masked else { unreachable!() };
        return Some(url);
    }
    def::secret_value(key, value).then(|| REDACTED.to_string())
}

/// `args`: each secret argument, by its index, and what it becomes.
fn args(items: &[String]) -> Vec<(usize, String)> {
    def::secret_args(items)
        .into_iter()
        .enumerate()
        .filter(|(_, s)| *s)
        .map(|(i, _)| {
            let a = &items[i];
            let new = match a.split_once('=') {
                Some((flag, _)) if a.starts_with('-') => format!("{flag}={REDACTED}"),
                _ => REDACTED.to_string(),
            };
            (i, new)
        })
        .collect()
}

/// `config.toml` with its secrets left out.
pub fn toml(text: &str) -> Result<String, String> {
    let doc = Document::parse(text).map_err(|e| e.message().to_string())?;
    let mut edits = Vec::new();
    for (key, item) in doc.as_table().iter() {
        toml_item(key, item, &mut edits)?;
    }
    let out = splice(text, edits);
    // The result must still be TOML, or the backup would be useless.
    Document::parse(out.as_str()).map_err(|e| e.message().to_string())?;
    Ok(out)
}

fn toml_item(
    key: &str,
    item: &Item,
    edits: &mut Vec<(Range<usize>, String)>,
) -> Result<(), String> {
    if NAME_KEYS.contains(&key) {
        return Ok(());
    }
    match item {
        Item::Value(v) => toml_value(key, v, edits),
        Item::Table(t) => {
            for (k, i) in t.iter() {
                toml_item(k, i, edits)?;
            }
            Ok(())
        }
        Item::ArrayOfTables(a) => {
            for t in a.iter() {
                for (k, i) in t.iter() {
                    toml_item(k, i, edits)?;
                }
            }
            Ok(())
        }
        Item::None => Ok(()),
    }
}

fn toml_value(key: &str, v: &Value, edits: &mut Vec<(Range<usize>, String)>) -> Result<(), String> {
    let span = || v.span().ok_or_else(|| "a value without a place".to_string());
    match v {
        Value::String(s) => {
            if let Some(new) = string(key, s.value()) {
                edits.push((span()?, toml_string(&new)));
            }
        }
        Value::Array(a) => {
            let strings: Vec<String> =
                a.iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect();
            let secret: Vec<usize> = if key == "args" {
                args(&strings).into_iter().map(|(i, _)| i).collect()
            } else {
                Vec::new()
            };
            for (i, item) in a.iter().enumerate() {
                if secret.contains(&i) {
                    let new = args(&strings).into_iter().find(|(j, _)| *j == i).map(|(_, n)| n);
                    let s = item.span().ok_or("a value without a place")?;
                    edits.push((s, toml_string(&new.unwrap_or_else(|| REDACTED.into()))));
                } else if item.as_str().is_some_and(def::token_shaped) {
                    let s = item.span().ok_or("a value without a place")?;
                    edits.push((s, quoted()));
                } else {
                    toml_value(key, item, edits)?;
                }
            }
        }
        Value::InlineTable(t) => {
            for (k, v) in t.iter() {
                if !NAME_KEYS.contains(&k) {
                    toml_value(k, v, edits)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn toml_string(s: &str) -> String {
    Value::from(s).to_string().trim().to_string()
}

/// A JSON agent file with its secrets left out.
pub fn json(text: &str) -> Result<String, String> {
    let root = json_text::parse(text)?;
    let mut edits = Vec::new();
    json_node("", &root, &mut edits);
    let out = splice(text, edits);
    serde_json::from_str::<serde_json::Value>(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn json_node(key: &str, node: &Node, edits: &mut Vec<(Range<usize>, String)>) {
    if NAME_KEYS.contains(&key) {
        return;
    }
    match &node.kind {
        Kind::Str(s) => {
            if let Some(new) = string(key, s) {
                edits.push((node.span.clone(), serde_json::to_string(&new).expect("serializes")));
            }
        }
        Kind::Object(members) => {
            for m in members {
                json_node(&m.key, &m.value, edits);
            }
        }
        Kind::Array(items) => {
            let strings: Vec<String> =
                items.iter().map(|n| n.as_str().unwrap_or_default().to_string()).collect();
            let secret = if key == "args" { args(&strings) } else { Vec::new() };
            for (i, item) in items.iter().enumerate() {
                if let Some((_, new)) = secret.iter().find(|(j, _)| *j == i) {
                    edits
                        .push((item.span.clone(), serde_json::to_string(new).expect("serializes")));
                } else if item.as_str().is_some_and(def::token_shaped) {
                    edits.push((item.span.clone(), quoted()));
                } else {
                    json_node(key, item, edits);
                }
            }
        }
        _ => {}
    }
}

fn splice(text: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    let mut out = text.to_string();
    edits.sort_by_key(|(at, _)| std::cmp::Reverse(at.start));
    for (at, new) in edits {
        out.replace_range(at, &new);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_codex_config_keeps_everything_but_its_secrets() {
        let text = r#"# keep this comment
model = "gpt-5.5"
cli_auth_credentials_store = "file"

[model_providers.custom]
base_url = "https://relay.example.com/v1"
env_key = "RELAY_API_KEY"
experimental_bearer_token = "test-key-not-real"   # the key
http_headers = { Authorization = "Bearer test-header-not-real", "X-Mode" = "fast" }

[mcp_servers.files]
command = "npx"
args = ["-y", "files-mcp", "--api-key", "test-arg-not-real", "--token=test-eq-not-real"]
env = { FILES_TOKEN = "test-token-not-real", MODE = "x" }

[mcp_servers.other]
command = "other-mcp"

[mcp_servers.other.env]
OTHER_SECRET = "test-sub-not-real"

[mcp_servers.web]
url = "https://u:test-pw-not-real@web.example.com/mcp?api_key=test-q-not-real&mode=fast"
bearer_token_env_var = "WEB_TOKEN"
"#;
        let out = toml(text).unwrap();
        for secret in [
            "test-key",
            "test-header",
            "test-arg",
            "test-eq",
            "test-token",
            "test-sub",
            "test-pw",
            "test-q",
        ] {
            assert!(!out.contains(secret), "{secret} left in:\n{out}");
        }
        for kept in [
            "# keep this comment",
            "cli_auth_credentials_store = \"file\"",
            "env_key = \"RELAY_API_KEY\"",
            "\"X-Mode\" = \"fast\"",
            "MODE = \"x\"",
            "--token=<redacted>",
            "bearer_token_env_var = \"WEB_TOKEN\"",
            "web.example.com/mcp?",
            "mode=fast",
            "   # the key",
        ] {
            assert!(out.contains(kept), "{kept} missing:\n{out}");
        }
        assert_eq!(out.lines().count(), text.lines().count());
        assert_eq!(toml(&out).unwrap(), out, "leaving secrets out twice changes nothing");
    }

    #[test]
    fn a_json_file_keeps_everything_but_its_secrets() {
        let text = r#"{
  "mcpServers": {
    "docs": { "serverUrl": "https://docs.example.com/mcp", "headers": { "CONTEXT7_API_KEY": "test-h-not-real" } },
    "files": { "command": "npx", "args": ["-y", "files-mcp", "--token", "test-a-not-real"], "env": { "PATH_HINT": "/opt", "GH_TOKEN": "${GH_TOKEN}" }, "disabled": true }
  }
}
"#;
        let out = json(text).unwrap();
        assert!(!out.contains("test-h") && !out.contains("test-a"), "{out}");
        assert!(
            out.contains("\"GH_TOKEN\": \"${GH_TOKEN}\""),
            "a reference is not a secret: {out}"
        );
        assert!(out.contains("\"PATH_HINT\": \"/opt\""));
        assert!(out.contains("\"disabled\": true"));
        assert_eq!(json(&out).unwrap(), out);
    }
}
