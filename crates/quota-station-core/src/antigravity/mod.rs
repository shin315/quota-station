//! Antigravity accounts the app logs in itself (M4a), the first step of
//! taking over what Cockpit Tools does. The app talks to Google as Cockpit
//! does: the same OAuth client, flow, scopes, parameters and refresh rule.
//!
//! Where things live, all under the app's Application Support folder:
//!
//! - `oauth-client.json` at its root: the OAuth client, put there by the
//!   owner ([`client`]);
//! - `antigravity/` (mode 700): `accounts.json` and `status.json`, with no
//!   secrets ([`index`]);
//! - the tokens only in the app's own Keychain items ([`store`]).
//!
//! Parts of [`oauth`], [`oauth_server`], [`token`] and [`index`] are ported
//! from Cockpit Tools 1.3.70 (CC BY-NC-SA 4.0); each file says what and what
//! changed, and `NOTICE` credits it. Reading quota (M4b) and switching
//! (M4c) come later.

pub mod client;
pub mod fs;
pub mod index;
pub mod login;
pub mod oauth;
pub mod oauth_server;
pub mod store;
pub mod token;
