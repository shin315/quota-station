// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The app's own Keychain items for Antigravity tokens: one generic password
//! per account, service [`SERVICE`], account the app's account id, value
//! the account's [`TokenData`] as JSON. Kept with macOS's `security` tool,
//! the way claude-swap and Cockpit Tools keep theirs, so a rebuilt app with
//! a new ad-hoc signature gets no Keychain prompt.
//!
//! Items are added without `-A`, so other programs need the owner's consent
//! to read them. The app never names another service: not the
//! `gemini`/`antigravity` item that Antigravity.app and agy share, not
//! claude-swap's or Claude Code's.

use std::sync::Arc;

use super::token::TokenData;
use crate::agents::secrets::SecurityCli;
pub use crate::agents::secrets::{StoreError, TIMEOUT};
use crate::os::SecretStore;

pub const SERVICE: &str = "com.shin315.quota-station.antigravity";

/// The Antigravity tokens, in the app's [`SecretStore`].
#[derive(Debug, Clone)]
pub struct Keychain {
    store: Arc<dyn SecretStore>,
}

impl Keychain {
    /// The system's store: [`SecurityCli`] on macOS, the app's sealed
    /// files on Windows ([`crate::os::secret_store`]).
    pub fn system() -> Self {
        #[cfg(windows)]
        let store = crate::os::secret_store();
        #[cfg(not(windows))]
        let store = SecurityCli::system();
        Keychain::new(store)
    }

    pub fn new(store: impl SecretStore + 'static) -> Self {
        Keychain { store: Arc::new(store) }
    }

    /// A fake `security` for the tests.
    #[cfg(feature = "test-support")]
    pub fn fake(program: impl Into<std::path::PathBuf>) -> Self {
        Keychain::new(SecurityCli::fake(program))
    }

    /// Adds or replaces the account's item. On a failure the item that was
    /// there stays as it was.
    pub fn save(&self, account: &str, token: &TokenData) -> Result<(), StoreError> {
        Ok(self.store.set(SERVICE, account, &ascii_json(token))?)
    }

    pub fn read(&self, account: &str) -> Result<TokenData, StoreError> {
        let text = self.store.get(SERVICE, account)?.ok_or(StoreError::NotFound)?;
        serde_json::from_str(text.trim_end_matches('\n')).map_err(|_| StoreError::Corrupt)
    }

    /// Removes the account's item; one that is not there counts as removed.
    pub fn delete(&self, account: &str) -> Result<(), StoreError> {
        Ok(self.store.delete(SERVICE, account)?)
    }
}

/// The token as JSON with every non-ASCII character escaped: `security -w`
/// prints a value it finds unprintable as hex, and a name or email may
/// carry accents.
fn ascii_json(token: &TokenData) -> String {
    let text = serde_json::to_string(token).expect("a token serializes");
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stored_value_is_ascii_and_reads_back_the_same() {
        let mut token = TokenData::new(
            "a".into(),
            "r".into(),
            3600,
            Some("ngọc@example.com".into()),
            None,
            None,
            100,
        );
        token.id_token = Some("x😀".into());
        let text = ascii_json(&token);
        assert!(text.is_ascii(), "{text}");
        let back: TokenData = serde_json::from_str(&text).unwrap();
        assert_eq!(back, token);
    }
}
