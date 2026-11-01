// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Antigravity accounts the app logs in itself (M4a), whose quota it reads
//! (M4b) and which it switches to (M4c), taking over what Cockpit Tools
//! does. The app talks to Google as Cockpit does: the same OAuth client,
//! flow, scopes, parameters and refresh rule, the same Cloud Code calls,
//! headers and user agent, and never more often.
//!
//! Where things live, all under the app's Application Support folder:
//!
//! - `oauth-client.json` at its root: the OAuth client, put there by the
//!   owner ([`client`]);
//! - `antigravity/` (mode 700): `accounts.json` and `status.json`, with no
//!   secrets ([`index`]), and each account's last quota reading under
//!   `cache/`, in Cockpit's layout ([`export`]), which the Antigravity row
//!   lists ([`listing`]); the binding, the switch log and the six-hour
//!   watch ([`switch`], [`watch`]);
//! - the tokens only in the app's own Keychain items ([`store`]).
//!
//! Switching the account (M4c, [`switch`]) writes the login Antigravity.app
//! and agy share ([`credential`]), quits and reopens Antigravity.app when the
//! owner asks ([`desktop`]), points the binding at the new account,
//! and watches six hours for the switch to be undone ([`watch`]).
//!
//! Parts of [`oauth`], [`oauth_server`], [`token`], [`index`],
//! [`cloud_code`], [`version`], [`export`], [`credential`], [`desktop`] and
//! [`switch`] are ported from Cockpit Tools 1.3.70 (CC BY-NC-SA 4.0); each
//! file says what and what changed, and `NOTICE` credits it.

pub mod client;
pub mod cloud_code;
pub mod credential;
pub mod desktop;
pub mod export;
pub mod index;
pub mod listing;
pub mod login;
pub mod oauth;
pub mod oauth_server;
pub mod pace;
pub mod quota;
pub mod store;
pub mod switch;
pub mod token;
pub mod version;
pub mod watch;
