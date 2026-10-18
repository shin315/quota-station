// Ported from Cockpit Tools `src-tauri/src/modules/oauth.rs` (1.3.70, commit
// 244b957, CC BY-NC-SA 4.0); changes: one OAuth client, read from
// `oauth-client.json` instead of constants in the source
// (`resolve_oauth_client_key` dropped); blocking HTTP; typed errors that
// keep only Google's error code, never a response body, a token or the
// secret; no logging; the clock is passed in; the endpoints point at a local
// fake in the tests. The URLs, scopes, parameters, timeout and refresh rule
// are Cockpit's.

//! Google's side of an Antigravity login: the authorization URL, the code
//! exchange, the token refresh and userinfo.

use std::time::Duration;

use serde::Deserialize;

use super::client::{OAuthClient, CLIENT_KEY};
use super::token::TokenData;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Antigravity's scopes, in Cockpit's order.
pub const SCOPES: [&str; 7] = [
    "openid",
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/cclog",
    "https://www.googleapis.com/auth/experimentsandconfigs",
    "https://www.googleapis.com/auth/aicode",
];

/// Every request's timeout (Cockpit's `create_client(15)`).
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// A token is refreshed when it has less than this left, and only right
/// before it is used.
pub const REFRESH_MARGIN_SECS: i64 = 300;

/// Where the login talks to. The app always uses [`Endpoints::google`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    auth: String,
    token: String,
    userinfo: String,
}

impl Endpoints {
    pub fn google() -> Self {
        Endpoints { auth: AUTH_URL.into(), token: TOKEN_URL.into(), userinfo: USERINFO_URL.into() }
    }

    /// A local fake Google at `base`, with Google's paths.
    #[cfg(feature = "test-support")]
    pub fn fake(base: &str) -> Self {
        let base = base.trim_end_matches('/');
        Endpoints {
            auth: format!("{base}/o/oauth2/v2/auth"),
            token: format!("{base}/token"),
            userinfo: format!("{base}/oauth2/v2/userinfo"),
        }
    }
}

#[derive(Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(skip)]
    pub oauth_client_key: Option<String>,
}

/// Never prints a token.
impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("expires_in", &self.expires_in)
            .field("has_refresh_token", &self.refresh_token.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
pub struct UserInfo {
    #[serde(default)]
    pub id: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
}

/// Never prints the email or the name.
impl std::fmt::Debug for UserInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInfo").finish_non_exhaustive()
    }
}

impl UserInfo {
    pub fn get_display_name(&self) -> Option<String> {
        if let Some(name) = &self.name {
            if !name.trim().is_empty() {
                return Some(name.clone());
            }
        }

        match (&self.given_name, &self.family_name) {
            (Some(given), Some(family)) => Some(format!("{} {}", given, family)),
            (Some(given), None) => Some(given.clone()),
            (None, Some(family)) => Some(family.clone()),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthError {
    /// The request did not get an answer: offline, DNS, TLS, timeout.
    Network(String),
    /// Google answered with an error status. `code` is its OAuth error code
    /// (`invalid_grant`, `invalid_client`…) when the body had one.
    Rejected { status: u16, code: Option<String> },
    /// A success status with a body that does not parse.
    Parse(String),
}

impl OAuthError {
    /// The refresh token was revoked or expired: only a new login helps.
    pub fn is_invalid_grant(&self) -> bool {
        matches!(self, OAuthError::Rejected { code: Some(c), .. } if c == "invalid_grant")
    }
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthError::Network(e) => write!(f, "no answer from Google: {e}"),
            OAuthError::Rejected { status, code: Some(c) } => {
                write!(f, "Google refused ({status}): {c}")
            }
            OAuthError::Rejected { status, code: None } => write!(f, "Google refused ({status})"),
            OAuthError::Parse(e) => write!(f, "unexpected answer from Google: {e}"),
        }
    }
}

impl std::error::Error for OAuthError {}

/// A client with the timeout; one that cannot be built (no TLS backend) is
/// a network failure, never a client without the timeout.
fn http() -> Result<reqwest::blocking::Client, OAuthError> {
    reqwest::blocking::Client::builder().timeout(HTTP_TIMEOUT).build().map_err(network)
}

/// reqwest's message without the URL it adds: the URL is one of the fixed
/// endpoints and says nothing new.
fn network(e: reqwest::Error) -> OAuthError {
    OAuthError::Network(e.without_url().to_string())
}

/// Google's OAuth error code from an error body, letters and `_` only.
fn error_code(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Body {
        error: serde_json::Value,
    }
    let value = serde_json::from_str::<Body>(body).ok()?.error;
    // OAuth endpoints send `"error": "invalid_grant"`; userinfo sends an
    // object with a `status` string.
    let code = match &value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(o) => o.get("status")?.as_str()?.to_string(),
        _ => return None,
    };
    let ok = !code.is_empty() && code.len() <= 64;
    (ok && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then_some(code)
}

fn read<T: serde::de::DeserializeOwned>(
    response: reqwest::blocking::Response,
) -> Result<T, OAuthError> {
    let status = response.status();
    if status.is_success() {
        response.json::<T>().map_err(|e| OAuthError::Parse(e.without_url().to_string()))
    } else {
        let body = response.text().unwrap_or_default();
        Err(OAuthError::Rejected { status: status.as_u16(), code: error_code(&body) })
    }
}

/// The URL that opens Google's consent screen.
pub fn get_auth_url(
    endpoints: &Endpoints,
    client: &OAuthClient,
    redirect_uri: &str,
    state: Option<&str>,
) -> String {
    let scopes = SCOPES.join(" ");

    let mut params = vec![
        ("client_id", client.id()),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("scope", &scopes),
        ("access_type", "offline"),
        ("prompt", "consent"),
    ];

    if let Some(state) = state.filter(|value| !value.trim().is_empty()) {
        params.push(("state", state));
    }

    let url = url::Url::parse_with_params(&endpoints.auth, &params).expect("a valid auth URL");
    url.to_string()
}

/// Trades the code from the callback for tokens.
pub fn exchange_code(
    endpoints: &Endpoints,
    client: &OAuthClient,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthError> {
    let params = [
        ("client_id", client.id()),
        ("client_secret", client.secret()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let response = http()?.post(&endpoints.token).form(&params).send().map_err(network)?;
    let mut token_res: TokenResponse = read(response)?;
    token_res.oauth_client_key = Some(CLIENT_KEY.to_string());
    Ok(token_res)
}

/// A new access token for `refresh_token`.
pub fn refresh_access_token(
    endpoints: &Endpoints,
    client: &OAuthClient,
    refresh_token: &str,
) -> Result<TokenResponse, OAuthError> {
    let params = [
        ("client_id", client.id()),
        ("client_secret", client.secret()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response = http()?.post(&endpoints.token).form(&params).send().map_err(network)?;
    let mut token_data: TokenResponse = read(response)?;
    token_data.oauth_client_key = Some(CLIENT_KEY.to_string());
    Ok(token_data)
}

pub fn get_user_info(endpoints: &Endpoints, access_token: &str) -> Result<UserInfo, OAuthError> {
    let response =
        http()?.get(&endpoints.userinfo).bearer_auth(access_token).send().map_err(network)?;
    read(response)
}

/// The token to use now: `current` while it has more than
/// [`REFRESH_MARGIN_SECS`] left, else a refreshed one that keeps the refresh
/// token, the email, the project and the GCP ToS mark.
pub fn ensure_fresh_token(
    endpoints: &Endpoints,
    client: &OAuthClient,
    current_token: &TokenData,
    now: i64,
) -> Result<TokenData, OAuthError> {
    if current_token.expiry_timestamp > now + REFRESH_MARGIN_SECS {
        return Ok(current_token.clone());
    }

    let response = refresh_access_token(endpoints, client, &current_token.refresh_token)?;

    let mut token = TokenData::new(
        response.access_token,
        current_token.refresh_token.clone(),
        response.expires_in,
        current_token.email.clone(),
        current_token.project_id.clone(),
        current_token.session_id.clone(),
        now,
    )
    .with_oauth_metadata(
        response.oauth_client_key,
        response.id_token.or_else(|| current_token.id_token.clone()),
    );
    token.is_gcp_tos = current_token.is_gcp_tos;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_short_oauth_error_code_is_kept_from_a_body() {
        assert_eq!(
            error_code(
                r#"{"error": "invalid_grant", "error_description": "Token has been expired or revoked."}"#
            ),
            Some("invalid_grant".into())
        );
        assert_eq!(
            error_code(r#"{"error": {"code": 401, "status": "UNAUTHENTICATED", "message": "x"}}"#),
            Some("UNAUTHENTICATED".into())
        );
        assert_eq!(error_code(r#"{"error": "has spaces and a fake token"}"#), None);
        assert_eq!(error_code("<html>"), None);
    }
}
