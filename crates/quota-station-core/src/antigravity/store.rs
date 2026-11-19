// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The app's own Keychain items for Antigravity tokens: one generic password
//! per account, service [`SERVICE`], account the app's account id, value
//! the account's [`TokenData`] as JSON. Kept with macOS's `security` tool,
//! the way other account tools keep theirs, so a rebuilt app with
//! a new ad-hoc signature gets no Keychain prompt.
//!
//! Items are added without `-A`, so other programs need the owner's consent
//! to read them. The app never names another service: not the
//! `gemini`/`antigravity` item that Antigravity.app and agy share, not
//! another account tool's or Claude Code's.
//!
//! On Linux, which has no Keychain (and Linux in WSL no keyring the app
//! could count on), the items are files in the app's own `antigravity/`
//! folder ([`files_dir`]): `secrets/<service>/<account>.bin`, of mode 600
//! in folders of mode 700, each written whole to a temporary file and
//! renamed over the old one ([`Keychain::files`]).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::token::TokenData;
// macOS's store, and the tests' fake one on every system.
#[cfg(any(not(any(windows, target_os = "linux")), feature = "test-support"))]
use crate::agents::secrets::SecurityCli;
pub use crate::agents::secrets::{StoreError, TIMEOUT};
use crate::app_dir::AppDir;
use crate::os::SecretStore;
use crate::sealed::{Plain, SealedFiles};

pub const SERVICE: &str = "com.shin315.quota-station.antigravity";
/// The folder of the items on Linux, in the app's `antigravity/` folder.
pub const FILES_DIR: &str = "secrets";

/// Where the items are kept on Linux: `antigravity/secrets` in the app's
/// folder.
pub fn files_dir(app: &AppDir) -> PathBuf {
    super::index::dir(app).join(FILES_DIR)
}

/// The Antigravity tokens, in the app's [`SecretStore`].
#[derive(Debug, Clone)]
pub struct Keychain {
    store: Arc<dyn SecretStore>,
}

impl Keychain {
    /// The system's store: [`SecurityCli`] on macOS, the app's sealed
    /// files on Windows ([`crate::os::secret_store`]), the app's own files
    /// on Linux ([`Keychain::files`] in [`files_dir`]).
    pub fn system() -> Self {
        #[cfg(windows)]
        return Keychain::new(crate::os::secret_store());
        #[cfg(not(any(windows, target_os = "linux")))]
        return Keychain::new(SecurityCli::system());
        #[cfg(target_os = "linux")]
        {
            // The tests keep their items in a folder of their own; one that
            // reached for the real store stops here.
            if cfg!(feature = "test-support") {
                panic!("tests must use a fake store");
            }
            let home = crate::linux::home().expect("no home folder: HOME is not set");
            Keychain::files(&files_dir(&AppDir::new(crate::claude::app_dir(&home))))
        }
    }

    /// The items as files under `dir`, as Linux keeps them: mode 600 in
    /// folders of mode 700, each written whole and renamed over the old one.
    pub fn files(dir: &Path) -> Self {
        Keychain::new(SealedFiles::new(dir, Plain))
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
