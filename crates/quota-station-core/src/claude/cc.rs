// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Onur Cetinkol (claude-swap)
// SPDX-License-Identifier: MIT

// Which item and file hold Claude Code's login, and how a switch writes
// them, follow claude-swap (`credentials.py`, `session.py`, `switcher.py`,
// MIT, https://github.com/realiti4/claude-swap): the Keychain item
// `Claude Code-credentials` under `$USER`, the per-folder item named from
// the folder's SHA-256, the MCP and plugin secrets kept from the live
// login, `~/.claude/.credentials.json` rewritten only when it is there, and
// only `oauthAccount` replaced in `~/.claude.json`.

//! Claude Code's side of a login: its Keychain item, its credentials file
//! and the `oauthAccount` member of `~/.claude.json`.

use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::keychain::{user_account, KeychainError};
use super::store::Who;
use super::Paths;
use crate::agents::json_text;
use crate::os::SecretStore;

/// Claude Code's item for its default folder.
pub const ACTIVE_SERVICE: &str = "Claude Code-credentials";
/// Top-level members of a credential that belong to the Mac, not to the
/// account: a switch keeps the live login's, present or absent.
const SHARED_KEYS: [&str; 5] =
    ["mcpOAuth", "mcpOAuthClientConfig", "mcpXaaIdp", "mcpXaaIdpConfig", "pluginSecrets"];

/// The item Claude Code keeps its login in when `CLAUDE_CONFIG_DIR` is
/// `dir`: the service ends with the first 8 hex digits of the SHA-256 of
/// the path as written. `None` for a path that is not plain ASCII, which
/// would need Unicode normalisation to match Claude Code's.
pub fn profile_service(dir: &Path) -> Option<String> {
    let text = dir.to_str().filter(|t| t.is_ascii())?;
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    Some(format!("{ACTIVE_SERVICE}-{hex}"))
}

/// The login Claude Code uses now, as read.
#[derive(Debug, Clone, Default)]
pub struct Live {
    /// The credential, as Claude Code stores it.
    pub credential: Option<String>,
    /// `oauthAccount` in `~/.claude.json`.
    pub oauth: Option<Value>,
}

impl Live {
    pub fn who(&self) -> Option<Who> {
        self.oauth.as_ref().map(Who::of).filter(|w| !w.email.is_empty())
    }
}

/// The Keychain item first, then `~/.claude/.credentials.json`, as Claude
/// Code reads them.
pub fn read_credential(
    paths: &Paths,
    security: &dyn SecretStore,
) -> Result<Option<String>, KeychainError> {
    if let Some(c) = security.get(ACTIVE_SERVICE, &user_account())? {
        if !c.trim().is_empty() {
            return Ok(Some(c));
        }
    }
    let file = paths.claude_home().join(".credentials.json");
    Ok(std::fs::read_to_string(file).ok().filter(|t| !t.trim().is_empty()))
}

/// `oauthAccount` of `~/.claude.json`, when the file reads.
pub fn read_oauth(paths: &Paths) -> Option<Value> {
    let text = std::fs::read_to_string(paths.claude_json()).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.get("oauthAccount").filter(|v| v.is_object()).cloned()
}

pub fn read_live(paths: &Paths, security: &dyn SecretStore) -> Result<Live, KeychainError> {
    Ok(Live { credential: read_credential(paths, security)?, oauth: read_oauth(paths) })
}

/// The `claudeAiOauth` part of a credential.
pub fn oauth_part(credential: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(credential).ok()?;
    v.get("claudeAiOauth").filter(|o| o.is_object()).cloned()
}

/// A credential with a token in it: never a wiped or empty one.
pub fn usable(credential: &str) -> bool {
    oauth_part(credential).is_some_and(|o| {
        let has = |k: &str| o.get(k).and_then(Value::as_str).is_some_and(|s| !s.is_empty());
        has("accessToken") || has("refreshToken")
    })
}

/// When the access token stops working, in milliseconds since the epoch.
pub fn expires_at(credential: &str) -> Option<i64> {
    oauth_part(credential)?.get("expiresAt")?.as_i64()
}

/// The refresh token, to tell two copies of one login apart.
pub fn refresh_token(credential: &str) -> Option<String> {
    oauth_part(credential)?.get("refreshToken")?.as_str().map(String::from)
}

/// The target's credential with the live login's MCP and plugin secrets.
pub fn merge_shared(target: &str, live: Option<&str>) -> String {
    let (Ok(Value::Object(mut out)), Some(Ok(Value::Object(live)))) =
        (serde_json::from_str::<Value>(target), live.map(serde_json::from_str::<Value>))
    else {
        return target.to_string();
    };
    for key in SHARED_KEYS {
        match live.get(key) {
            Some(v) => {
                out.insert(key.to_string(), v.clone());
            }
            None => {
                out.remove(key);
            }
        }
    }
    serde_json::to_string(&Value::Object(out)).unwrap_or_else(|_| target.to_string())
}

/// Puts `credential` on Claude Code's own item, and in
/// `~/.claude/.credentials.json` when that file is there (a running Claude
/// Code notices the file changed), never creating it.
pub fn write_credential(
    paths: &Paths,
    security: &dyn SecretStore,
    credential: &str,
) -> Result<(), std::io::Error> {
    security
        .set(ACTIVE_SERVICE, &user_account(), credential)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let file = paths.claude_home().join(".credentials.json");
    if file.is_file() {
        super::write_private(&file, credential)?;
    }
    Ok(())
}

/// `~/.claude.json` with `oauthAccount` set to `oauth`, every other byte
/// kept. A file that is not there gets only that member.
pub fn spliced(text: Option<&str>, oauth: &Value) -> Result<String, String> {
    let Some(text) = text else {
        let mut out = serde_json::Map::new();
        out.insert("oauthAccount".into(), oauth.clone());
        return Ok(serde_json::to_string_pretty(&Value::Object(out)).expect("serializes") + "\n");
    };
    let root = json_text::parse(text)?;
    if !root.is_object() {
        return Err("~/.claude.json is not an object".into());
    }
    let unit = json_text::unit(text, &root);
    // serde's two-space steps become the file's own, line by line: a JSON
    // string never holds a raw line break, so only indentation is touched.
    let pretty = |indent: &str| {
        let text = serde_json::to_string_pretty(oauth).expect("serializes");
        let lines: Vec<String> = text
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let body = line.trim_start_matches(' ');
                let steps = (line.len() - body.len()) / 2;
                let lead = if i == 0 { String::new() } else { indent.to_string() };
                format!("{lead}{}{body}", unit.repeat(steps))
            })
            .collect();
        lines.join("\n")
    };
    let edit = match root.get("oauthAccount") {
        Some(m) => {
            let indent = json_text::indent_at(text, m.key_span.start).unwrap_or("").to_string();
            json_text::Edit { at: m.value.span.clone(), text: pretty(&indent) }
        }
        None => json_text::add_member(text, &root, "oauthAccount", &pretty(&unit), &unit),
    };
    Ok(json_text::apply(text, vec![edit]))
}

/// Writes `oauthAccount` into `~/.claude.json` (temporary file, rename,
/// mode 600). A file that does not read as JSON is kept beside it as
/// `.claude.json.unreadable-<time>` before the new one is written.
pub fn write_oauth(paths: &Paths, oauth: &Value) -> Result<(), std::io::Error> {
    let file = paths.claude_json();
    let text = super::read_optional(&file)?;
    let next = match spliced(text.as_deref(), oauth) {
        Ok(next) => next,
        Err(_) => {
            let stamp = time::OffsetDateTime::now_utc().unix_timestamp();
            let keep = file.with_file_name(format!(".claude.json.unreadable-{stamp}"));
            std::fs::copy(&file, keep)?;
            spliced(None, oauth).map_err(std::io::Error::other)?
        }
    };
    super::write_private(&file, &next)
}

/// Puts `~/.claude.json` back as `text` was, or removes it when there was none.
pub fn restore_config(paths: &Paths, text: Option<&str>) -> Result<(), std::io::Error> {
    match text {
        Some(t) => super::write_private(&paths.claude_json(), t),
        None => match std::fs::remove_file(paths.claude_json()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_profile_item_is_named_from_the_folder() {
        let name = profile_service(Path::new("/Users/x/claude-a")).unwrap();
        assert!(name.starts_with("Claude Code-credentials-"));
        assert_eq!(name.len(), ACTIVE_SERVICE.len() + 9);
        assert_ne!(Some(name), profile_service(Path::new("/Users/x/claude-a/")));
        assert_eq!(profile_service(Path::new("/Users/ngọc/a")), None);
    }

    #[test]
    fn only_oauth_account_changes_in_the_config() {
        let text = "{\n  \"numStartups\": 7,\n  \"oauthAccount\": {\n    \"emailAddress\": \"a@example.com\"\n  },\n  \"zeta\": [1, 2]\n}\n";
        let out =
            spliced(Some(text), &json!({"emailAddress": "b@example.com", "accountUuid": "u2"}))
                .unwrap();
        assert!(out.starts_with("{\n  \"numStartups\": 7,\n  \"oauthAccount\": {\n    \"accountUuid\": \"u2\",\n    \"emailAddress\": \"b@example.com\"\n  },"), "{out}");
        assert!(out.ends_with("  \"zeta\": [1, 2]\n}\n"), "{out}");
        let without = "{\n  \"a\": 1\n}\n";
        let added = spliced(Some(without), &json!({"emailAddress": "b@example.com"})).unwrap();
        let v: Value = serde_json::from_str(&added).unwrap();
        assert_eq!(v["a"], 1);
        assert_eq!(v["oauthAccount"]["emailAddress"], "b@example.com");
    }

    #[test]
    fn a_switch_keeps_the_macs_mcp_secrets() {
        let target = r#"{"claudeAiOauth":{"accessToken":"t2"},"mcpOAuth":{"old":1},"trustedDeviceToken":"d2"}"#;
        let live = r#"{"claudeAiOauth":{"accessToken":"t1"},"pluginSecrets":{"p":1}}"#;
        let v: Value = serde_json::from_str(&merge_shared(target, Some(live))).unwrap();
        assert_eq!(v["claudeAiOauth"]["accessToken"], "t2");
        assert_eq!(v["trustedDeviceToken"], "d2");
        assert_eq!(v["pluginSecrets"], json!({"p": 1}));
        assert!(v.get("mcpOAuth").is_none());
    }

    #[test]
    fn a_wiped_login_is_not_usable() {
        assert!(usable(r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":""}}"#));
        assert!(!usable(r#"{"claudeAiOauth":{"accessToken":"","refreshToken":""}}"#));
        assert!(!usable("not json"));
    }
}
