//! Antigravity accounts the app logs in itself (M4a) and whose quota it
//! reads (M4b), taking over what Cockpit Tools does. The app talks to Google
//! as Cockpit does: the same OAuth client, flow, scopes, parameters and
//! refresh rule, the same Cloud Code calls, headers and user agent, and
//! never more often.
//!
//! Where things live, all under the app's Application Support folder:
//!
//! - `oauth-client.json` at its root: the OAuth client, put there by the
//!   owner ([`client`]);
//! - `antigravity/` (mode 700): `accounts.json` and `status.json`, with no
//!   secrets ([`index`]), and each account's last quota reading under
//!   `cache/`, in Cockpit's layout, where quota-axi reads it ([`export`]);
//! - the tokens only in the app's own Keychain items ([`store`]).
//!
//! Parts of [`oauth`], [`oauth_server`], [`token`], [`index`],
//! [`cloud_code`], [`version`] and [`export`] are ported from Cockpit Tools
//! 1.3.70 (CC BY-NC-SA 4.0); each file says what and what changed, and
//! `NOTICE` credits it. Switching (M4c) comes later.

pub mod client;
pub mod cloud_code;
pub mod export;
pub mod fs;
pub mod index;
pub mod login;
pub mod oauth;
pub mod oauth_server;
pub mod pace;
pub mod quota;
pub mod store;
pub mod token;
pub mod version;
