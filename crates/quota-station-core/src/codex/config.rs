//! Codex's `config.toml`: which provider it uses, the custom provider table
//! a switch goes to, its own key, the MCP list (read here; only the MCP tab
//! edits it, [`crate::agents::codex_mcp`]) and what CC Switch left behind;
//! and the edit a switch makes.
//!
//! The edit touches only the keys the switch owns: `model_provider` at the
//! top of the file (set to the custom provider's id, or removed for the
//! ChatGPT login), and `requires_openai_auth` in that provider's table when
//! switching to it. Every other byte stays as it was: the edit is made on the
//! text, at the places the parser found, and checked by [`apply`] before it
//! is returned. A switch never writes `auth.json` or `[mcp_servers]`.
//!
//! Ported in part from CC Switch (MIT, © 2025 Jason Young), commit 79e754ab,
//! `src-tauri/src/live/project/codex.rs`: the reserved and built-in provider
//! ids, how a table's own credentials are told apart (`declared_auth`), the
//! `requires_openai_auth` rule, the refusal of a table with no key of its own
//! (`keyless_fallback_error`), the placeholder CC Switch leaves in a dormant
//! table, and the check that the active profile does not choose another
//! provider (`check_effective_route`). See `NOTICE`.

use serde::Serialize;
use toml_edit::{Document, DocumentMut, Item, TableLike};

/// The id Codex uses when `model_provider` is not set: the ChatGPT login.
pub const DEFAULT_ID: &str = "openai";
/// A table under one of these ids makes Codex 0.148 and later refuse the
/// whole file.
pub const RESERVED_TABLE_IDS: &[&str] = &["openai", "ollama", "lmstudio"];
/// Codex's own providers (case-sensitive, as upstream: `OpenAI` is a valid custom id).
const BUILT_IN_IDS: &[&str] =
    &["amazon-bedrock", "amazon-bedrock-runtime", "openai", "ollama", "lmstudio"];
/// The id CC Switch writes its custom provider under; the app prefers it
/// when the file has several provider tables and none is remembered.
pub const CC_SWITCH_ID: &str = "custom";
/// The key CC Switch leaves in the table while the ChatGPT login is in use
/// (its "dormant" table, pointing at its own proxy): not a real key.
pub const PROXY_TOKEN_PLACEHOLDER: &str = "PROXY_MANAGED";
/// The model list CC Switch writes and points `model_catalog_json` at.
pub const CC_SWITCH_CATALOG: &str = "cc-switch-model-catalog.json";

fn is_built_in(id: &str) -> bool {
    BUILT_IN_IDS.contains(&id)
}

/// Where the custom provider's credentials come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyKind {
    /// `experimental_bearer_token` in the table.
    Bearer,
    /// `env_key`: an environment variable holds it.
    EnvKey,
    /// The table authenticates itself: an `auth` command, `aws`, an
    /// `Authorization` header or query parameters.
    Headers,
    /// Only CC Switch's placeholder: the real key is in CC Switch's data.
    Placeholder,
    /// No credentials of its own.
    None,
}

/// The `requires_openai_auth` the custom provider's table should hold.
///
/// Codex does not take the request's credentials from it (`env_key` and
/// `experimental_bearer_token` come first), but it decides the login
/// screen: `true` with no login in `auth.json` leaves Codex on the login
/// page; `false` while a ChatGPT login sits there makes Codex treat it as
/// logged out (no account, no token refresh, so no plugins and no remote
/// control). Only a provider with its own key (Bearer, EnvKey) follows the
/// login; Headers and None are always `false`, or the request would fall
/// back to the ChatGPT login and send it to the third party.
pub fn requires_openai_auth(kind: KeyKind, login: bool) -> bool {
    matches!(kind, KeyKind::EnvKey | KeyKind::Bearer) && login
}

/// `cli_auth_credentials_store`: where Codex keeps its login.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthStore {
    File,
    Keyring,
    Auto,
    Ephemeral,
    Unknown,
}

/// The custom provider table a switch goes to. Only facts about the key,
/// never its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Custom {
    pub id: String,
    /// Its `name`, else the id.
    pub name: String,
    pub base_url: Option<String>,
    pub key: KeyKind,
    /// The variable `env_key` names (a name, not a key).
    pub env_key: Option<String>,
    pub requires_openai_auth: Option<bool>,
    /// A plain `[model_providers.<id>]` table: other shapes are not edited.
    pub plain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mcp {
    pub name: String,
    /// `stdio` (a `command`), `http` (a `url`) or `other`.
    pub kind: &'static str,
    pub enabled: bool,
}

/// What the file says, read without keeping any secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// `model_provider` at the top; `None` uses the ChatGPT login.
    pub selected: Option<String>,
    pub custom: Option<Custom>,
    /// Every provider table id, in the file's order.
    pub tables: Vec<String>,
    /// The active `profile`, when it sets any of the keys that pick a provider.
    pub profile: Option<Profile>,
    pub store: AuthStore,
    pub mcp: Vec<Mcp>,
    /// What CC Switch left in the file: `model_catalog_json`, a table under
    /// a reserved id, its dormant table.
    pub traces: Vec<String>,
    /// A top-level `openai_base_url` sends the ChatGPT login elsewhere.
    pub openai_base_url: bool,
}

/// The active `profile`'s keys that pick where requests go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub model_provider: Option<String>,
    /// `openai_base_url` or `experimental_bearer_token`, when it sets one.
    pub reroute: Option<&'static str>,
}

impl Parsed {
    /// The key of the active profile that would send requests somewhere
    /// other than `selector` (CC Switch's `check_effective_route`). Without
    /// `model_provider` at the top Codex uses `openai`: a profile that picks
    /// it goes where the ChatGPT login goes.
    pub fn profile_override(&self, selector: Option<&str>) -> Option<(String, String)> {
        let p = self.profile.as_ref()?;
        if let Some(id) = &p.model_provider {
            if id != selector.unwrap_or(DEFAULT_ID) {
                return Some((p.name.clone(), "model_provider".into()));
            }
        }
        p.reroute.map(|key| (p.name.clone(), key.to_string()))
    }

    /// The provider in use now, as Codex reads the file.
    pub fn current(&self) -> Current {
        match self.selected.as_deref() {
            None | Some(DEFAULT_ID) => Current::ChatGpt,
            Some(id) if self.custom.as_ref().is_some_and(|c| c.id == id) => Current::Custom,
            Some(id) => Current::Other(id.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Current {
    ChatGpt,
    Custom,
    /// Another provider: Codex's own (`ollama`, …) or another table.
    Other(String),
}

fn non_empty_str(item: Option<&Item>) -> Option<String> {
    item.and_then(Item::as_str).map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

fn table_bool(table: &dyn TableLike, key: &str) -> Option<bool> {
    table.get(key).and_then(Item::as_bool)
}

fn declares_authorization_header(item: Option<&Item>) -> bool {
    item.and_then(Item::as_table_like)
        .is_some_and(|t| t.iter().any(|(key, _)| key.eq_ignore_ascii_case("authorization")))
}

/// The table's own credentials (CC Switch's `declared_auth` and the keyless
/// branch of `custom_route`). An `Authorization` header counts as Headers
/// whatever `requires_openai_auth` holds now: the switch sets it to `false`
/// for this kind, so the header is what Codex sends.
fn key_kind(table: &dyn TableLike) -> KeyKind {
    if table.get("env_key").is_some() {
        return KeyKind::EnvKey;
    }
    if table.get("auth").is_some() || table.get("aws").is_some() {
        return KeyKind::Headers;
    }
    match non_empty_str(table.get("experimental_bearer_token")).as_deref() {
        Some(PROXY_TOKEN_PLACEHOLDER) => return KeyKind::Placeholder,
        Some(_) => return KeyKind::Bearer,
        None => {}
    }
    if table.get("query_params").is_some()
        || declares_authorization_header(table.get("http_headers"))
        || declares_authorization_header(table.get("env_http_headers"))
    {
        return KeyKind::Headers;
    }
    KeyKind::None
}

fn store(doc: &DocumentMut) -> AuthStore {
    match doc.get("cli_auth_credentials_store").and_then(Item::as_str) {
        None | Some("file") => AuthStore::File,
        Some("keyring") => AuthStore::Keyring,
        Some("auto") => AuthStore::Auto,
        Some("ephemeral") => AuthStore::Ephemeral,
        Some(_) => AuthStore::Unknown,
    }
}

fn profile(doc: &DocumentMut) -> Option<Profile> {
    let name = non_empty_str(doc.get("profile"))?;
    let table = doc.get("profiles")?.as_table_like()?.get(&name)?.as_table_like()?;
    let model_provider = non_empty_str(table.get("model_provider"));
    let reroute = ["openai_base_url", "experimental_bearer_token"]
        .into_iter()
        .find(|key| non_empty_str(table.get(key)).is_some());
    (model_provider.is_some() || reroute.is_some()).then_some(Profile {
        name,
        model_provider,
        reroute,
    })
}

/// Parses the file. `remembered` is the custom provider the app last saw in
/// use, so a file with several tables goes back to the same one.
pub fn parse(text: &str, remembered: Option<&str>) -> Result<Parsed, String> {
    let doc: DocumentMut =
        text.parse().map_err(|e: toml_edit::TomlError| e.message().to_string())?;
    let selected = non_empty_str(doc.get("model_provider"));
    let providers = doc.get("model_providers").and_then(Item::as_table_like);
    let tables: Vec<String> = providers
        .map(|p| {
            p.iter()
                .filter(|(_, item)| item.is_table_like())
                .map(|(id, _)| id.to_string())
                .collect()
        })
        .unwrap_or_default();
    let usable = |id: &str| !is_built_in(id) && tables.iter().any(|t| t == id);
    let chosen = selected
        .as_deref()
        .filter(|id| usable(id))
        .or(remembered.filter(|id| usable(id)))
        .or(Some(CC_SWITCH_ID).filter(|id| usable(id)))
        .or_else(|| tables.iter().map(String::as_str).find(|id| !is_built_in(id)))
        .map(str::to_string);

    let mut traces = Vec::new();
    if let Some(catalog) = non_empty_str(doc.get("model_catalog_json")) {
        if catalog.ends_with(CC_SWITCH_CATALOG) {
            traces.push(format!("model_catalog_json = \"{catalog}\""));
        }
    }
    for id in RESERVED_TABLE_IDS {
        if tables.iter().any(|t| t == id) {
            traces.push(format!("[model_providers.{id}]"));
        }
    }

    let custom = chosen.and_then(|id| {
        let providers = providers?;
        let item = providers.get(&id)?;
        let table = item.as_table_like()?;
        let key = key_kind(table);
        if key == KeyKind::Placeholder {
            traces.push(format!(
                "[model_providers.{id}] experimental_bearer_token = \"{PROXY_TOKEN_PLACEHOLDER}\""
            ));
        }
        let plain = matches!(doc.get("model_providers"), Some(Item::Table(_)))
            && matches!(item, Item::Table(t) if !t.is_dotted());
        Some(Custom {
            name: non_empty_str(table.get("name")).unwrap_or_else(|| id.clone()),
            base_url: non_empty_str(table.get("base_url")),
            key,
            env_key: non_empty_str(table.get("env_key")),
            requires_openai_auth: table_bool(table, "requires_openai_auth"),
            plain,
            id,
        })
    });

    let mcp = doc
        .get("mcp_servers")
        .and_then(Item::as_table_like)
        .map(|servers| {
            servers
                .iter()
                .filter_map(|(name, item)| {
                    let t = item.as_table_like()?;
                    let kind = if t.get("command").is_some() {
                        "stdio"
                    } else if t.get("url").is_some() {
                        "http"
                    } else {
                        "other"
                    };
                    Some(Mcp {
                        name: name.to_string(),
                        kind,
                        enabled: table_bool(t, "enabled").unwrap_or(true),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Parsed {
        profile: profile(&doc),
        selected,
        custom,
        tables,
        store: store(&doc),
        mcp,
        traces,
        openai_base_url: doc.get("openai_base_url").is_some(),
    })
}

/// A fingerprint of the MCP list (names, kinds, on or off), to tell whether
/// it changed since the app last wrote the file.
pub fn mcp_fingerprint(mcp: &[Mcp]) -> String {
    mcp.iter()
        .map(|m| format!("{}:{}:{}", m.name, m.kind, if m.enabled { 1 } else { 0 }))
        .collect::<Vec<_>>()
        .join(",")
}

/// Where a switch goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    /// The ChatGPT login: no `model_provider`.
    ChatGpt,
    /// The custom provider table.
    Custom,
}

/// Why a switch cannot be made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Refusal {
    /// The file has no custom provider table.
    NoCustom,
    /// The table has no key of its own: Codex would send the ChatGPT login
    /// to the provider.
    NoKey,
    /// Only CC Switch's placeholder: the real key is in CC Switch's data.
    Placeholder,
    /// The active profile picks another provider.
    Profile { profile: String, key: String },
    /// The table is inline or dotted: not edited.
    Shape,
}

/// The edit a switch makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub target: Target,
    /// `model_provider` after the edit.
    pub selector: Option<String>,
    /// `requires_openai_auth` in the custom table, when the edit sets it.
    pub requires: Option<(String, bool)>,
}

/// The edit for `target`. `login` says whether Codex has a login (see
/// [`requires_openai_auth`]).
pub fn plan(parsed: &Parsed, target: Target, login: bool) -> Result<Plan, Refusal> {
    let (selector, requires) = match target {
        Target::ChatGpt => (None, None),
        Target::Custom => {
            let custom = parsed.custom.as_ref().ok_or(Refusal::NoCustom)?;
            match custom.key {
                KeyKind::None => return Err(Refusal::NoKey),
                KeyKind::Placeholder => return Err(Refusal::Placeholder),
                _ => {}
            }
            if !custom.plain {
                return Err(Refusal::Shape);
            }
            let requires = requires_openai_auth(custom.key, login);
            (Some(custom.id.clone()), Some((custom.id.clone(), requires)))
        }
    };
    if let Some((profile, key)) = parsed.profile_override(selector.as_deref()) {
        return Err(Refusal::Profile { profile, key });
    }
    Ok(Plan { target, selector, requires })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    Parse(String),
    /// The edited text changed more than the switch's keys, or does not say
    /// what the plan says: nothing is written.
    Check(&'static str),
}

/// One replacement in the original text.
struct Edit {
    at: std::ops::Range<usize>,
    text: String,
}

/// The start of the line `at` is on.
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Just past the end of the line `at` is on (its newline included).
fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map(|i| at + i + 1).unwrap_or(text.len())
}

fn quoted(s: &str) -> String {
    toml_edit::Value::from(s).to_string().trim().to_string()
}

/// The text with the plan's edit made. Every other line stays byte for
/// byte; the result is checked before it is returned.
pub fn apply(text: &str, plan: &Plan) -> Result<String, PatchError> {
    let doc = Document::parse(text).map_err(|e| PatchError::Parse(e.message().to_string()))?;
    let root = doc.as_table();
    let mut edits: Vec<Edit> = Vec::new();

    let current = root.get_key_value("model_provider");
    match (&plan.selector, current) {
        (Some(id), Some((_, item))) => {
            let span = item.span().ok_or(PatchError::Check("span"))?;
            edits.push(Edit { at: span, text: quoted(id) });
        }
        (Some(id), None) => {
            edits.push(Edit { at: 0..0, text: format!("model_provider = {}\n", quoted(id)) });
        }
        (None, Some((key, item))) => {
            let start = line_start(text, key.span().ok_or(PatchError::Check("span"))?.start);
            let end = line_end(text, item.span().ok_or(PatchError::Check("span"))?.end);
            edits.push(Edit { at: start..end, text: String::new() });
        }
        (None, None) => {}
    }

    if let Some((id, want)) = &plan.requires {
        let table = root
            .get("model_providers")
            .and_then(Item::as_table)
            .and_then(|p| p.get(id))
            .and_then(Item::as_table)
            .ok_or(PatchError::Check("table"))?;
        match table.get_key_value("requires_openai_auth") {
            Some((_, item)) => {
                if item.as_bool() != Some(*want) {
                    let span = item.span().ok_or(PatchError::Check("span"))?;
                    edits.push(Edit { at: span, text: want.to_string() });
                }
            }
            None => {
                // After the table's last line of its own, else after its header.
                let last = table
                    .iter()
                    .filter_map(|(k, item)| {
                        if item.is_table() {
                            return None;
                        }
                        let (key, _) = table.get_key_value(k)?;
                        key.span()?;
                        item.span().map(|s| s.end)
                    })
                    .max();
                let after = match last {
                    Some(end) => line_end(text, end),
                    None => line_end(text, table.span().ok_or(PatchError::Check("span"))?.end),
                };
                let newline = if after > 0 && !text[..after].ends_with('\n') { "\n" } else { "" };
                edits.push(Edit {
                    at: after..after,
                    text: format!("{newline}requires_openai_auth = {want}\n"),
                });
            }
        }
    }

    let mut out = text.to_string();
    edits.sort_by_key(|e| std::cmp::Reverse(e.at.start));
    for e in edits {
        out.replace_range(e.at, &e.text);
    }
    check(text, &out, plan)?;
    Ok(out)
}

fn owned_line(line: &str) -> bool {
    let t = line.trim_start();
    ["model_provider", "requires_openai_auth"]
        .iter()
        .any(|k| t.strip_prefix(k).is_some_and(|rest| rest.trim_start().starts_with('=')))
}

/// The edit made only the switch's lines, and the file now says what the
/// plan says.
fn check(before: &str, after: &str, plan: &Plan) -> Result<(), PatchError> {
    let rest = |t: &str| t.split_inclusive('\n').filter(|l| !owned_line(l)).collect::<String>();
    if rest(before) != rest(after) {
        return Err(PatchError::Check("otherLines"));
    }
    let doc: DocumentMut = after.parse().map_err(|_| PatchError::Check("parse"))?;
    if non_empty_str(doc.get("model_provider")) != plan.selector {
        return Err(PatchError::Check("selector"));
    }
    if let Some((id, want)) = &plan.requires {
        let got = doc
            .get("model_providers")
            .and_then(|p| p.get(id))
            .and_then(|t| t.get("requires_openai_auth"))
            .and_then(Item::as_bool);
        if got != Some(*want) {
            return Err(PatchError::Check("requires"));
        }
    }
    let before_doc: DocumentMut = before.parse().map_err(|_| PatchError::Check("parse"))?;
    let section = |d: &DocumentMut, key: &str| d.get(key).map(|i| i.to_string());
    for key in ["mcp_servers", "profiles"] {
        if section(&before_doc, key) != section(&doc, key) {
            return Err(PatchError::Check("section"));
        }
    }
    Ok(())
}
