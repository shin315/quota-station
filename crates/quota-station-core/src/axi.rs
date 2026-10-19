//! The part of quota-axi's `--json` report Quota Station reads.
//!
//! Only the fields the app uses are declared. Serde drops every other field,
//! so identity fields such as `email` or `organization` never enter the app
//! even if a caller passed `--full`.

use serde::Deserialize;

/// Report schemas this build understands. Schema 6 is schema 5 plus
/// `accountKey` lanes; the fields read here are the same in both.
pub const SUPPORTED_SCHEMAS: [u32; 2] = [5, 6];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub generated_at: String,
    pub schema_version: u32,
    #[serde(default)]
    pub providers: Vec<Provider>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub provider: String,
    #[serde(default)]
    pub account_key: Option<String>,
    /// The credentials this row speaks for: Codex's own login is `codex-home`.
    #[serde(default)]
    pub account_keys: Vec<String>,
    #[serde(default)]
    pub windows: Vec<Window>,
    #[serde(default)]
    pub quota_semantics: Option<QuotaSemantics>,
    #[serde(default)]
    pub state: State,
    #[serde(default)]
    pub not_set_up: bool,
    #[serde(default)]
    pub claude_swap: Option<ClaudeSwap>,
    #[serde(default)]
    pub claude_pool: Option<ClaudePool>,
    #[serde(default)]
    pub cockpit: Option<Cockpit>,
    #[serde(default)]
    pub agy_pool: Option<AgyPool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub percent_remaining: Option<f64>,
    #[serde(default)]
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub refreshed_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSemantics {
    #[serde(default)]
    pub effective_availability: Vec<Effective>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Effective {
    pub scope: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub effective_percent_remaining: Option<f64>,
    #[serde(default)]
    pub limiting_window_ids: Vec<String>,
}

/// One effective scope of a claude-swap slot or a Cockpit account.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub scope: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub effective_percent_remaining: Option<f64>,
    #[serde(default)]
    pub limited_by: Option<String>,
    #[serde(default)]
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSwap {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub active_slot: Option<u32>,
    #[serde(default)]
    pub slots: Vec<SwapSlot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapSlot {
    pub slot: u32,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub disabled: bool,
    pub status: String,
    #[serde(default)]
    pub age_seconds: Option<f64>,
    #[serde(default)]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub effective_percent_remaining: Option<f64>,
    #[serde(default)]
    pub limited_by: Option<String>,
    #[serde(default)]
    pub resets_at: Option<String>,
    #[serde(default)]
    pub scopes: Vec<Scope>,
    /// claude-swap's last fetch error for the slot (`invalid_grant`,
    /// `http-429`), or why no reading could be used (`no_reading`).
    #[serde(default)]
    pub error: Option<String>,
    /// When the slot's login itself ends (claude-swap's `loginExpiresAt`),
    /// for a quota-axi that passes it through.
    #[serde(default)]
    pub login_expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePool {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub rotation: Option<Rotation>,
    #[serde(default)]
    pub members: Vec<PoolMember>,
    #[serde(default)]
    pub excluded: Vec<PoolExcluded>,
    #[serde(default)]
    pub scopes: Vec<ClaudePoolScope>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rotation {
    #[serde(default)]
    pub team: Vec<u32>,
    #[serde(default)]
    pub personal: Option<u32>,
    #[serde(default)]
    pub personal_hours: Option<String>,
    #[serde(default)]
    pub expire: Vec<Expire>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Expire {
    pub slot: u32,
    pub at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolMember {
    pub slot: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolExcluded {
    pub slot: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePoolScope {
    pub scope: String,
    pub effective_percent_remaining: f64,
    pub slots_of_room: f64,
    #[serde(default)]
    pub next_refill_at: Option<String>,
    #[serde(default)]
    pub slots: Vec<RoomSlot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSlot {
    #[serde(alias = "account")]
    pub slot: u32,
    #[serde(default)]
    pub room_percent: f64,
    #[serde(default)]
    pub refill_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cockpit {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// Whose files quota-axi read: `quota-station` (this app's folder,
    /// numbered as the app lists them) or `cockpit`; none when a folder was
    /// named, or from an older fork.
    #[serde(default)]
    pub source: Option<String>,
    /// How the account in use was told: `identity`, `reading` (the live
    /// reading matched its numbers) or `cockpit` (the binding last written).
    #[serde(default)]
    pub active_basis: Option<String>,
    #[serde(default)]
    pub accounts: Vec<CockpitAccount>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CockpitAccount {
    pub account: u32,
    #[serde(default)]
    pub active: bool,
    pub status: String,
    #[serde(default)]
    pub age_seconds: Option<f64>,
    #[serde(default)]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgyPool {
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub rotation: Option<AgyRotation>,
    #[serde(default)]
    pub members: Vec<AgyMember>,
    #[serde(default)]
    pub scopes: Vec<AgyPoolScope>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgyRotation {
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgyMember {
    pub account: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgyPoolScope {
    pub scope: String,
    pub effective_percent_remaining: f64,
    pub accounts_of_room: f64,
    #[serde(default)]
    pub next_refill_at: Option<String>,
    #[serde(default)]
    pub accounts: Vec<RoomSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// stdout was not a quota-axi JSON report.
    Malformed(String),
    /// A report schema this build does not read.
    UnsupportedSchema(u32),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Malformed(why) => write!(f, "malformed quota-axi JSON: {why}"),
            ParseError::UnsupportedSchema(n) => write!(f, "unsupported quota-axi schema {n}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses one `quota-axi --json` report.
pub fn parse(text: &str) -> Result<Response, ParseError> {
    let response: Response =
        serde_json::from_str(text).map_err(|e| ParseError::Malformed(e.to_string()))?;
    if !SUPPORTED_SCHEMAS.contains(&response.schema_version) {
        return Err(ParseError::UnsupportedSchema(response.schema_version));
    }
    Ok(response)
}

impl Response {
    /// The row a sidecar report rides on: quota-axi attaches `claudeSwap` and
    /// `cockpit` to the first row of their provider, and schema 6 can list
    /// more than one lane per provider.
    pub fn primary(&self, provider: &str) -> Option<&Provider> {
        let mut rows = self.providers.iter().filter(|p| p.provider == provider);
        let first = rows.clone().next();
        rows.find(|p| p.claude_swap.is_some() || p.cockpit.is_some()).or(first)
    }
}
