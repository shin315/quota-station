//! Quota Station's own list of Claude accounts and their secrets.
//!
//! `claude/accounts.json` (mode 600) holds each account's slot number,
//! alias, whether it is held out of the rotation, its kind, its ids, and
//! [`identity`]: a hash of its email and organization, so the app can tell
//! which account Claude Code is on without keeping the email. No email,
//! organization name or token is in the file.
//!
//! The secrets are two Keychain items per account, service [`SERVICE`]:
//! `slot-N`, the credential exactly as Claude Code stores it, and
//! `slot-N.account`, the account's `oauthAccount` object, which holds its
//! email and organization. Both are read only when needed and kept in
//! memory only.

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::keychain::{KeychainError, Security};
use super::Paths;

/// The app's own Keychain service for the Claude accounts.
pub const SERVICE: &str = "com.shin315.quota-station.claude";
const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// A Claude login, with a refresh token.
    #[default]
    Login,
    /// A long-lived token from `claude setup-token`: no refresh, and no
    /// email of its own.
    SetupToken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub slot: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// Held out of the rotation; still switched to by hand.
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub kind: Kind,
    /// `oauthAccount.accountUuid`; empty for a setup token.
    #[serde(default)]
    pub account_uuid: String,
    /// `oauthAccount.organizationUuid`; empty for none.
    #[serde(default)]
    pub organization_uuid: String,
    /// [`identity`] of the account's email and organization.
    pub identity: String,
    /// When it was added, RFC 3339.
    #[serde(default)]
    pub added: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct List {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub accounts: Vec<Account>,
}

impl List {
    pub fn get(&self, slot: u32) -> Option<&Account> {
        self.accounts.iter().find(|a| a.slot == slot)
    }

    pub fn get_mut(&mut self, slot: u32) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|a| a.slot == slot)
    }

    /// The account with this identity.
    pub fn find(&self, identity: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.identity == identity)
    }

    /// The next free slot number: one past the highest.
    pub fn next_slot(&self) -> u32 {
        self.accounts.iter().map(|a| a.slot).max().unwrap_or(0) + 1
    }

    /// Puts `account` in, replacing the one in its slot, in slot order.
    pub fn put(&mut self, account: Account) {
        self.accounts.retain(|a| a.slot != account.slot);
        self.accounts.push(account);
        self.accounts.sort_by_key(|a| a.slot);
    }
}

/// The hash that names an account without its email: SHA-256 of the email
/// and the organization id, as claude-swap tells accounts apart (one email
/// may be in two organizations), first 32 hex digits.
pub fn identity(email: &str, organization_uuid: &str) -> String {
    let mut h = Sha256::new();
    h.update(email.trim().as_bytes());
    h.update([0]);
    h.update(organization_uuid.trim().as_bytes());
    h.finalize().iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// The parts of an `oauthAccount` object the app reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Who {
    pub email: String,
    pub account_uuid: String,
    pub organization_uuid: String,
    pub organization_name: String,
}

impl Who {
    pub fn of(oauth: &Value) -> Who {
        let text = |key: &str| oauth.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        Who {
            email: text("emailAddress"),
            account_uuid: text("accountUuid"),
            organization_uuid: text("organizationUuid"),
            organization_name: text("organizationName"),
        }
    }

    pub fn identity(&self) -> String {
        identity(&self.email, &self.organization_uuid)
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    /// `accounts.json` is there but not a list the app wrote.
    Corrupt(String),
    Keychain(KeychainError),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "{e}"),
            StoreError::Corrupt(e) => write!(f, "accounts.json cannot be read: {e}"),
            StoreError::Keychain(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

impl From<KeychainError> for StoreError {
    fn from(e: KeychainError) -> Self {
        StoreError::Keychain(e)
    }
}

/// The account list and its Keychain items.
#[derive(Debug, Clone)]
pub struct Store {
    pub paths: Paths,
    pub security: Security,
}

fn credential_item(slot: u32) -> String {
    format!("slot-{slot}")
}

fn account_item(slot: u32) -> String {
    format!("slot-{slot}.account")
}

impl Store {
    pub fn new(paths: Paths, security: Security) -> Store {
        Store { paths, security }
    }

    /// The list; none yet is an empty list. A file that does not parse is
    /// an error, never an empty list: the app would otherwise forget every
    /// account at the next save.
    pub fn load(&self) -> Result<List, StoreError> {
        match super::read_optional(&self.paths.accounts())? {
            None => Ok(List { version: VERSION, accounts: Vec::new() }),
            Some(text) => {
                serde_json::from_str(&text).map_err(|e| StoreError::Corrupt(e.to_string()))
            }
        }
    }

    pub fn save(&self, list: &List) -> Result<(), StoreError> {
        let mut list = list.clone();
        list.version = VERSION;
        list.accounts.sort_by_key(|a| a.slot);
        let text = serde_json::to_string_pretty(&list).expect("the list serializes");
        super::write_private(&self.paths.accounts(), &(text + "\n"))?;
        Ok(())
    }

    /// The account's credential, as Claude Code stores it.
    pub fn credential(&self, slot: u32) -> Result<Option<String>, KeychainError> {
        self.security.get(SERVICE, &credential_item(slot))
    }

    pub fn set_credential(&self, slot: u32, credential: &str) -> Result<(), KeychainError> {
        self.security.set(SERVICE, &credential_item(slot), credential)
    }

    /// The account's `oauthAccount` object.
    pub fn oauth_account(&self, slot: u32) -> Result<Option<Value>, KeychainError> {
        let text = self.security.get(SERVICE, &account_item(slot))?;
        Ok(text.and_then(|t| serde_json::from_str(&t).ok()))
    }

    pub fn set_oauth_account(&self, slot: u32, oauth: &Value) -> Result<(), KeychainError> {
        self.security.set(SERVICE, &account_item(slot), &ascii_json(oauth))
    }

    /// Deletes both items of `slot`.
    pub fn forget(&self, slot: u32) -> Result<(), KeychainError> {
        self.security.delete(SERVICE, &credential_item(slot))?;
        self.security.delete(SERVICE, &account_item(slot))
    }
}

/// JSON with every non-ASCII character escaped: `security -w` prints a value
/// it finds unprintable as hex, and a name may carry accents.
pub fn ascii_json(value: &Value) -> String {
    let text = serde_json::to_string(value).expect("a JSON value serializes");
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

/// An alias as the owner may type it: up to 24 characters, no line breaks.
/// Empty clears it.
pub fn clean_alias(text: &str) -> Result<Option<String>, &'static str> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if text.chars().count() > 24 || text.chars().any(char::is_control) {
        return Err("badAlias");
    }
    Ok(Some(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identity_tells_one_email_in_two_organizations_apart() {
        let a = identity("lan@example.com", "org-1");
        assert_eq!(a.len(), 32);
        assert_ne!(a, identity("lan@example.com", "org-2"));
        assert_eq!(a, identity(" lan@example.com ", "org-1"));
    }

    #[test]
    fn the_stored_account_object_is_ascii_and_reads_back() {
        let oauth = serde_json::json!({"emailAddress": "ngọc@example.com", "displayName": "Ngọc"});
        let text = ascii_json(&oauth);
        assert!(text.is_ascii(), "{text}");
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), oauth);
    }
}
