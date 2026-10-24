// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: jlcodes (Cockpit Tools)
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

// Ported from Cockpit Tools `src-tauri/src/models/token.rs` (1.3.70, commit
// 244b957, CC BY-NC-SA 4.0); changes: the clock is passed in, comments in
// English. The fields and their JSON names are Cockpit's, so a token the app
// keeps reads the same as one Cockpit keeps.

//! One account's tokens, as the app keeps them in its own Keychain item.

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    /// Unix seconds when `access_token` expires.
    pub expiry_timestamp: i64,
    pub token_type: String,
    pub email: Option<String>,
    /// The Google Cloud project the quota calls name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Picks the Cloud Code host as Antigravity does: GCP ToS accounts use prod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_gcp_tos: Option<bool>,
    /// The OAuth client that got or last refreshed this token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// Google's account id from userinfo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Never prints a token or the email.
impl std::fmt::Debug for TokenData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenData")
            .field("expiry_timestamp", &self.expiry_timestamp)
            .field("oauth_client_key", &self.oauth_client_key)
            .finish_non_exhaustive()
    }
}

impl TokenData {
    pub fn new(
        access_token: String,
        refresh_token: String,
        expires_in: i64,
        email: Option<String>,
        project_id: Option<String>,
        session_id: Option<String>,
        now: i64,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_in,
            expiry_timestamp: now + expires_in,
            token_type: "Bearer".to_string(),
            email,
            project_id,
            is_gcp_tos: None,
            oauth_client_key: None,
            id_token: None,
            session_id,
        }
    }

    pub fn with_oauth_metadata(
        mut self,
        oauth_client_key: Option<String>,
        id_token: Option<String>,
    ) -> Self {
        self.oauth_client_key = oauth_client_key;
        self.id_token = id_token;
        self
    }
}
