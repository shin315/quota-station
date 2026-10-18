// Ported from Cockpit Tools `src-tauri/src/modules/quota.rs` (1.3.70,
// commit 244b957, CC BY-NC-SA 4.0): the hosts and their choice
// (`select_cloud_code_base_url`), `QuotaCloudCodeContext`, the
// `loadCodeAssist` request (`build_cloud_code_metadata`,
// `build_load_code_assist_payload`, its headers, `extract_project_id`, the
// GCP ToS detection), the `fetchAvailableModels` and
// `retrieveUserQuotaSummary` requests and the payload Cockpit caches, the
// 403 handling, the retries a second apart, and
// `parse_google_api_error_with_email` with `format_google_validation_url`.
// Changes: blocking HTTP; no environment overrides, no autopush host, no
// `onboardUser`, no tier or credit parsing for a UI; `loadCodeAssist` is
// asked once, only when the caller needs it; retries only after a network
// error (a 429 or 5xx is not retried here: the caller backs off instead);
// typed errors that keep Google's reason code and verification link, never
// a response body or a token; no logging; the hosts point at a local fake in
// the tests.

//! The three Cloud Code calls Antigravity makes to learn an account's quota,
//! made the way Cockpit Tools makes them: the same hosts, paths, headers,
//! bodies and user agents.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use super::oauth::HTTP_TIMEOUT;
use super::token::TokenData;
use super::version::{self, Identity};

const DAILY_BASE_URL: &str = "https://daily-cloudcode-pa.googleapis.com";
const PROD_BASE_URL: &str = "https://cloudcode-pa.googleapis.com";
const LOAD_CODE_ASSIST_PATH: &str = "v1internal:loadCodeAssist";
const FETCH_AVAILABLE_MODELS_PATH: &str = "v1internal:fetchAvailableModels";
const RETRIEVE_USER_QUOTA_SUMMARY_PATH: &str = "v1internal:retrieveUserQuotaSummary";
/// The project Google names for consumer accounts; Cockpit never sends it.
const CONSUMER_PLACEHOLDER: &str = "aicode-consumers";

/// Tries of `fetchAvailableModels` after network errors (Cockpit's
/// `max_retries = 3`), a second apart.
pub const ATTEMPTS: usize = 3;
pub const RETRY_DELAY: Duration = Duration::from_secs(1);

/// The two Cloud Code hosts. The app always uses [`Hosts::google`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosts {
    daily: String,
    prod: String,
}

impl Hosts {
    pub fn google() -> Self {
        Hosts { daily: DAILY_BASE_URL.into(), prod: PROD_BASE_URL.into() }
    }

    /// A local fake at `base`: the daily host under `/daily`, prod under `/prod`.
    #[cfg(feature = "test-support")]
    pub fn fake(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Hosts { daily: format!("{base}/daily"), prod: format!("{base}/prod") }
    }

    /// Antigravity's choice: prod for a GCP ToS account with a project,
    /// daily for every other.
    pub fn base_url(&self, ctx: &Context) -> &str {
        if ctx.is_gcp_tos && valid_project_id(ctx.project_id.as_deref()).is_some() {
            return &self.prod;
        }
        &self.daily
    }
}

/// What picks the host and the project the calls name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    pub project_id: Option<String>,
    pub is_gcp_tos: bool,
}

impl Context {
    pub fn from_token(token: &TokenData) -> Self {
        Self {
            project_id: valid_project_id(token.project_id.as_deref()).map(str::to_string),
            is_gcp_tos: token.is_gcp_tos.unwrap_or(false),
        }
    }
}

/// A project the calls may name: not empty and not the consumer placeholder.
pub fn valid_project_id(project_id: Option<&str>) -> Option<&str> {
    project_id.map(str::trim).filter(|value| !value.is_empty() && *value != CONSUMER_PLACEHOLDER)
}

/// Which call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    LoadCodeAssist,
    Models,
    Summary,
}

/// What Google's error body said, without its message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiError {
    /// `VALIDATION_REQUIRED`, `SUBSCRIPTION_REQUIRED` or another reason code.
    pub reason: Option<String>,
    /// The page that verifies the account, with the account's email in it.
    pub validation_url: Option<String>,
}

impl ApiError {
    /// Google asks for a verification or a subscription before more use.
    pub fn needs_verification(&self) -> bool {
        self.validation_url.is_some()
            || matches!(
                self.reason.as_deref(),
                Some("VALIDATION_REQUIRED" | "SUBSCRIPTION_REQUIRED")
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudCodeError {
    /// No answer: offline, DNS, TLS, timeout.
    Network { call: Call, detail: String },
    /// An error status.
    Rejected { call: Call, status: u16, error: ApiError },
    /// A success status with a body that is not JSON.
    Parse { call: Call, detail: String },
}

impl CloudCodeError {
    pub fn call(&self) -> Call {
        match self {
            CloudCodeError::Network { call, .. }
            | CloudCodeError::Rejected { call, .. }
            | CloudCodeError::Parse { call, .. } => *call,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            CloudCodeError::Rejected { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Too many requests, or Google's side failing: the caller waits longer.
    pub fn throttled(&self) -> bool {
        self.status().is_some_and(|s| s == 429 || s >= 500)
    }

    /// `fetchAvailableModels` answered 403: Cockpit marks the account
    /// forbidden and stops reading it.
    pub fn forbidden(&self) -> bool {
        matches!(self, CloudCodeError::Rejected { call: Call::Models, status: 403, .. })
    }

    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            CloudCodeError::Rejected { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl std::fmt::Display for CloudCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = |call: &Call| match call {
            Call::LoadCodeAssist => "loadCodeAssist",
            Call::Models => "fetchAvailableModels",
            Call::Summary => "retrieveUserQuotaSummary",
        };
        match self {
            CloudCodeError::Network { call, detail } => {
                write!(f, "{}: no answer from Google: {detail}", name(call))
            }
            CloudCodeError::Rejected { call, status, error } => match &error.reason {
                Some(reason) => write!(f, "{}: Google refused ({status}): {reason}", name(call)),
                None => write!(f, "{}: Google refused ({status})", name(call)),
            },
            CloudCodeError::Parse { call, detail } => {
                write!(f, "{}: unexpected answer from Google: {detail}", name(call))
            }
        }
    }
}

impl std::error::Error for CloudCodeError {}

/// What `loadCodeAssist` says about an account.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectInfo {
    pub project_id: Option<String>,
    /// `None` when Google did not say.
    pub is_gcp_tos: Option<bool>,
}

/// The tokens, the version and the hosts one round of calls uses.
pub struct CloudCode<'a> {
    pub hosts: &'a Hosts,
    pub identity: &'a Identity,
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| e.without_url().to_string())
}

fn network(call: Call, e: reqwest::Error) -> CloudCodeError {
    CloudCodeError::Network { call, detail: e.without_url().to_string() }
}

fn rejected(call: Call, response: reqwest::blocking::Response, email: &str) -> CloudCodeError {
    let status = response.status().as_u16();
    let text = response.text().unwrap_or_default();
    CloudCodeError::Rejected { call, status, error: parse_google_api_error(&text, email) }
}

fn read_json(call: Call, response: reqwest::blocking::Response) -> Result<Value, CloudCodeError> {
    let text = response
        .text()
        .map_err(|e| CloudCodeError::Network { call, detail: e.without_url().to_string() })?;
    serde_json::from_str(&text).map_err(|e| CloudCodeError::Parse { call, detail: e.to_string() })
}

fn metadata(identity: &Identity, duet_project: Option<&str>) -> Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("ideName".to_string(), Value::String("antigravity".to_string()));
    metadata.insert("ideType".to_string(), Value::String("ANTIGRAVITY".to_string()));
    metadata.insert("ideVersion".to_string(), Value::String(identity.ide_version.clone()));
    metadata.insert("pluginVersion".to_string(), Value::String(plugin_version()));
    metadata.insert("platform".to_string(), Value::String(version::platform().to_string()));
    metadata.insert("updateChannel".to_string(), Value::String("stable".to_string()));
    metadata.insert("pluginType".to_string(), Value::String("GEMINI".to_string()));
    if let Some(project) = duet_project.map(str::trim).filter(|value| !value.is_empty()) {
        metadata.insert("duetProject".to_string(), Value::String(project.to_string()));
    }
    Value::Object(metadata)
}

/// Cockpit sends its own version here; the app sends its own.
fn plugin_version() -> String {
    let version = env!("CARGO_PKG_VERSION").trim();
    if version.is_empty() {
        return "unknown".to_string();
    }
    version.to_string()
}

/// The `loadCodeAssist` body.
pub fn load_code_assist_payload(identity: &Identity, project_id: Option<&str>) -> Value {
    let mut payload = json!({
        "metadata": metadata(identity, project_id),
        "mode": "FULL_ELIGIBILITY_CHECK"
    });
    if let Some(project_id) = project_id.filter(|id| !id.trim().is_empty()) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "cloudaicompanionProject".to_string(),
                serde_json::Value::String(project_id.to_string()),
            );
        }
    }
    payload
}

/// The `fetchAvailableModels` and `retrieveUserQuotaSummary` body.
pub fn quota_payload(ctx: &Context) -> Value {
    valid_project_id(ctx.project_id.as_deref())
        .map(|id| json!({ "project": id }))
        .unwrap_or_else(|| json!({}))
}

#[derive(Deserialize)]
struct LoadProjectResponse {
    #[serde(rename = "cloudaicompanionProject")]
    project: Option<Value>,
    #[serde(rename = "currentTier")]
    current_tier: Option<Tier>,
    #[serde(rename = "allowedTiers")]
    allowed_tiers: Option<Vec<AllowedTier>>,
}

#[derive(Deserialize)]
struct AllowedTier {
    #[serde(rename = "isDefault")]
    is_default: Option<bool>,
    #[serde(rename = "usesGcpTos")]
    uses_gcp_tos: Option<bool>,
}

#[derive(Deserialize)]
struct Tier {
    #[serde(rename = "usesGcpTos")]
    uses_gcp_tos: Option<bool>,
}

fn extract_project_id(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    if let Some(obj) = value.as_object() {
        if let Some(id_value) = obj.get("id") {
            if let Some(id) = id_value.as_str() {
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}

impl CloudCode<'_> {
    /// The account's project and whether it is a GCP ToS account. Asked
    /// once, as Cockpit asks it.
    pub fn load_code_assist(
        &self,
        access_token: &str,
        email: &str,
        ctx: &Context,
    ) -> Result<ProjectInfo, CloudCodeError> {
        let call = Call::LoadCodeAssist;
        let base_url = self.hosts.base_url(ctx);
        let preferred = ctx.project_id.as_deref().map(str::trim).filter(|v| !v.is_empty());
        let client = http().map_err(|detail| CloudCodeError::Network { call, detail })?;
        let response = client
            .post(format!("{}/{}", base_url, LOAD_CODE_ASSIST_PATH))
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::USER_AGENT, self.identity.load_code_assist_user_agent())
            .header("x-goog-api-client", version::X_GOOG_API_CLIENT)
            .header(reqwest::header::ACCEPT, "*/*")
            .header(reqwest::header::ACCEPT_ENCODING, "gzip, deflate, br")
            .json(&load_code_assist_payload(self.identity, preferred))
            .send()
            .map_err(|e| network(call, e))?;
        if !response.status().is_success() {
            return Err(rejected(call, response, email));
        }
        let data: LoadProjectResponse = serde_json::from_value(read_json(call, response)?)
            .map_err(|e| CloudCodeError::Parse { call, detail: e.to_string() })?;

        let is_gcp_tos = data.current_tier.as_ref().and_then(|t| t.uses_gcp_tos).or_else(|| {
            data.allowed_tiers.as_ref().and_then(|tiers| {
                tiers.iter().find(|t| t.is_default.unwrap_or(false)).and_then(|t| t.uses_gcp_tos)
            })
        });
        let project_id = data
            .project
            .as_ref()
            .and_then(extract_project_id)
            .filter(|id| valid_project_id(Some(id)).is_some());
        Ok(ProjectInfo { project_id, is_gcp_tos })
    }

    /// `fetchAvailableModels`, then `retrieveUserQuotaSummary`: the payload
    /// Cockpit caches, the models answer with the summary under
    /// `quota_summary`. Unlike Cockpit, a summary that did not come back is
    /// an error rather than a payload without it, so the caller keeps the
    /// last full reading.
    pub fn fetch_quota(
        &self,
        access_token: &str,
        email: &str,
        ctx: &Context,
    ) -> Result<Value, CloudCodeError> {
        let base_url = self.hosts.base_url(ctx);
        let payload = quota_payload(ctx);
        let user_agent = self.identity.cloud_code_user_agent();
        let post = |client: &reqwest::blocking::Client, path: &str| {
            client
                .post(format!("{}/{}", base_url, path))
                .bearer_auth(access_token)
                .header(reqwest::header::USER_AGENT, &user_agent)
                .header(reqwest::header::ACCEPT_ENCODING, "gzip")
                .json(&payload)
                .send()
        };

        let call = Call::Models;
        let client = http().map_err(|detail| CloudCodeError::Network { call, detail })?;
        let mut attempt = 1;
        let response = loop {
            match post(&client, FETCH_AVAILABLE_MODELS_PATH) {
                Ok(response) => break response,
                Err(_) if attempt < ATTEMPTS => {
                    attempt += 1;
                    std::thread::sleep(RETRY_DELAY);
                }
                Err(e) => return Err(network(call, e)),
            }
        };
        if !response.status().is_success() {
            return Err(rejected(call, response, email));
        }
        let mut models = read_json(call, response)?;

        let call = Call::Summary;
        let response =
            post(&client, RETRIEVE_USER_QUOTA_SUMMARY_PATH).map_err(|e| network(call, e))?;
        if !response.status().is_success() {
            return Err(rejected(call, response, email));
        }
        let summary = read_json(call, response)?;
        match models.as_object_mut() {
            Some(obj) => {
                obj.insert("quota_summary".to_string(), summary);
                Ok(models)
            }
            None => Err(CloudCodeError::Parse {
                call: Call::Models,
                detail: "not a JSON object".into(),
            }),
        }
    }
}

/// Google's reason code and verification link from an error body, as
/// Cockpit reads them. The message itself is only looked at, never kept.
pub fn parse_google_api_error(text: &str, email: &str) -> ApiError {
    let mut message = text.to_string();
    let mut reason: Option<String> = None;
    let mut validation_url: Option<String> = None;

    if let Ok(val) = serde_json::from_str::<Value>(text) {
        if let Some(err_obj) = val.get("error") {
            if let Some(msg) = err_obj.get("message").and_then(|v| v.as_str()) {
                message = msg.to_string();
            }
            if let Some(details) = err_obj.get("details").and_then(|v| v.as_array()) {
                for detail in details {
                    if reason.is_none() {
                        if let Some(r) = detail.get("reason").and_then(|v| v.as_str()) {
                            reason = Some(r.to_string());
                        }
                    }
                    if validation_url.is_none() {
                        if let Some(metadata) = detail.get("metadata") {
                            if let Some(url) =
                                metadata.get("validation_url").and_then(|v| v.as_str())
                            {
                                validation_url = Some(url.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    if reason.is_none() {
        if message.contains("Verify your account") {
            reason = Some("VALIDATION_REQUIRED".to_string());
        } else if message.contains("Subscription required") {
            reason = Some("SUBSCRIPTION_REQUIRED".to_string());
        }
    }
    // Only a short code is kept: it reaches the windows.
    let reason = reason.filter(|r| {
        !r.is_empty() && r.len() <= 64 && r.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    });

    ApiError {
        reason,
        validation_url: validation_url.map(|url| format_validation_url(&url, email)),
    }
}

/// A Google verification page with the account's email filled in, so it
/// opens for the right account, as Cockpit does.
pub fn format_validation_url(raw_url: &str, email: &str) -> String {
    let email = email.trim();
    if raw_url.trim().is_empty() || email.is_empty() {
        return raw_url.to_string();
    }

    match url::Url::parse(raw_url) {
        Ok(mut parsed) => {
            let host = match parsed.host_str() {
                Some(h) => h.to_ascii_lowercase(),
                None => return raw_url.to_string(),
            };
            let is_google_host = host == "accounts.google.com"
                || host == "google.com"
                || host.ends_with(".google.com");
            if !is_google_host {
                return raw_url.to_string();
            }

            let mut pairs: Vec<(String, String)> = parsed.query_pairs().into_owned().collect();
            let has_valid_authuser = pairs.iter().any(|(k, v)| {
                k == "authuser"
                    && !v.trim().is_empty()
                    && !v.trim().chars().all(|c| c.is_ascii_digit())
            });
            let has_login_hint =
                pairs.iter().any(|(k, v)| k == "login_hint" && !v.trim().is_empty());
            let has_email = pairs.iter().any(|(k, v)| k == "Email" && !v.trim().is_empty());

            if !has_valid_authuser {
                pairs.retain(|(k, _)| k != "authuser");
                pairs.push(("authuser".to_string(), email.to_string()));
            }
            if !has_login_hint {
                pairs.retain(|(k, _)| k != "login_hint");
                pairs.push(("login_hint".to_string(), email.to_string()));
            }
            if !has_email {
                pairs.retain(|(k, _)| k != "Email");
                pairs.push(("Email".to_string(), email.to_string()));
            }

            parsed.query_pairs_mut().clear().extend_pairs(pairs);
            parsed.to_string()
        }
        Err(_) => raw_url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Identity {
        Identity { ide_version: "2.24.0".into(), google_client_version: "10.3.0".into() }
    }

    #[test]
    fn the_host_is_prod_only_for_a_gcp_tos_account_with_a_real_project() {
        let hosts = Hosts::google();
        let ctx = |project: Option<&str>, gcp| Context {
            project_id: project.map(String::from),
            is_gcp_tos: gcp,
        };
        assert_eq!(hosts.base_url(&ctx(Some("p-1"), true)), PROD_BASE_URL);
        for project in [None, Some(""), Some(" aicode-consumers ")] {
            assert_eq!(hosts.base_url(&ctx(project, true)), DAILY_BASE_URL, "{project:?}");
        }
        assert_eq!(hosts.base_url(&ctx(Some("p-1"), false)), DAILY_BASE_URL);
    }

    #[test]
    fn a_token_context_drops_the_consumer_placeholder() {
        let mut token =
            TokenData::new("a".into(), "r".into(), 3600, None, Some(" p-1 ".into()), None, 0);
        assert_eq!(
            Context::from_token(&token),
            Context { project_id: Some("p-1".into()), is_gcp_tos: false }
        );
        token.project_id = Some("aicode-consumers".into());
        token.is_gcp_tos = Some(true);
        assert_eq!(Context::from_token(&token), Context { project_id: None, is_gcp_tos: true });
    }

    #[test]
    fn the_bodies_are_cockpits() {
        let body = load_code_assist_payload(&identity(), Some("p-1"));
        // serde_json keeps keys sorted, as in Cockpit's build.
        let expected = format!(
            concat!(
                r#"{{"cloudaicompanionProject":"p-1","metadata":{{"duetProject":"p-1","#,
                r#""ideName":"antigravity","ideType":"ANTIGRAVITY","ideVersion":"2.24.0","#,
                r#""platform":"{}","pluginType":"GEMINI","pluginVersion":"{}","#,
                r#""updateChannel":"stable"}},"mode":"FULL_ELIGIBILITY_CHECK"}}"#
            ),
            version::platform(),
            env!("CARGO_PKG_VERSION"),
        );
        assert_eq!(body.to_string(), expected);
        let none = load_code_assist_payload(&identity(), None);
        assert!(none.get("cloudaicompanionProject").is_none());
        assert!(none["metadata"].get("duetProject").is_none());

        let ctx = |p: Option<&str>| Context { project_id: p.map(String::from), is_gcp_tos: false };
        assert_eq!(quota_payload(&ctx(Some("p-1"))).to_string(), r#"{"project":"p-1"}"#);
        assert_eq!(quota_payload(&ctx(None)).to_string(), "{}");
        assert_eq!(quota_payload(&ctx(Some("aicode-consumers"))).to_string(), "{}");
    }

    #[test]
    fn an_error_body_gives_its_reason_and_link_only() {
        let body = r#"{"error":{"code":403,"message":"Verify your account to continue. secret-ish text","details":[{"reason":"VALIDATION_REQUIRED","metadata":{"validation_url":"https://accounts.google.com/signin/continue?authuser=0"}}]}}"#;
        let error = parse_google_api_error(body, "someone@example.com");
        assert_eq!(error.reason.as_deref(), Some("VALIDATION_REQUIRED"));
        assert!(error.needs_verification());
        let url = url::Url::parse(error.validation_url.as_deref().unwrap()).unwrap();
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("authuser".into(), "someone@example.com".into())));
        assert!(pairs.contains(&("login_hint".into(), "someone@example.com".into())));
        assert!(!format!("{error:?}").contains("secret-ish"));

        let plain = parse_google_api_error(r#"{"error":{"message":"Subscription required"}}"#, "x");
        assert_eq!(plain.reason.as_deref(), Some("SUBSCRIPTION_REQUIRED"));
        let other = parse_google_api_error("<html>rate limited</html>", "x");
        assert_eq!(other, ApiError::default());
        assert!(!other.needs_verification());
        let odd = parse_google_api_error(
            r#"{"error":{"details":[{"reason":"has spaces, not a code"}]}}"#,
            "x",
        );
        assert_eq!(odd.reason, None);
    }

    #[test]
    fn only_google_pages_get_the_email() {
        let raw = "https://example.com/verify?code=123";
        assert_eq!(format_validation_url(raw, "someone@example.com"), raw);
        let fake = "https://evil-google.com/signin?authuser";
        assert_eq!(format_validation_url(fake, "someone@example.com"), fake);
        let kept = "https://accounts.google.com/x?authuser=existing%40example.com";
        let url = url::Url::parse(&format_validation_url(kept, "someone@example.com")).unwrap();
        assert!(url.query_pairs().any(|(k, v)| k == "authuser" && v == "existing@example.com"));
    }
}
