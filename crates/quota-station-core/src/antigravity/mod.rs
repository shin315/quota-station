// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Antigravity accounts the app logs in itself (M4a), whose quota it reads
//! (M4b) and which it switches to (M4c). The app talks to Google as
//! Antigravity does: Antigravity's OAuth client, flow, scopes, parameters
//! and refresh rule, the same Cloud Code calls, headers and user agent, at
//! the pace [`pace`] sets: the account in use with the app's reads, the others
//! every 10 minutes, none more than once a minute.
//!
//! Where things live, all under the app's Application Support folder:
//!
//! - `oauth-client.json` at its root, when the owner put one there: the
//!   OAuth client, which otherwise comes from the installed Antigravity
//!   ([`client`], [`discover`]);
//! - `antigravity/` (mode 700): `accounts.json` and `status.json`, with no
//!   secrets ([`index`]), and each account's last quota reading under
//!   `cache/` ([`export`]), which the Antigravity row
//!   lists ([`listing`]); the binding, the switch log and the six-hour
//!   watch ([`switch`], [`watch`]);
//! - the tokens only in the app's own Keychain items ([`store`]; on
//!   Windows the app's own sealed files).
//!
//! Switching the account (M4c, [`switch`]) writes the login Antigravity.app
//! and agy share ([`credential`]; on Windows the credential in Credential
//! Manager), quits and reopens Antigravity.app when the owner asks
//! ([`desktop`]; on Windows `Antigravity.exe`), points the binding at the
//! new account, and watches six hours for the switch to be undone
//! ([`watch`]).
//!
//! Parts of [`oauth`], [`oauth_server`], [`token`], [`index`],
//! [`cloud_code`], [`version`], [`export`], [`credential`], [`desktop`] and
//! [`switch`] hold adapted code under CC BY-NC-SA 4.0; each file names its
//! source in its header, and `NOTICE` credits it and lists the changes.

pub mod cli;
pub mod client;
pub mod cloud_code;
pub mod credential;
pub mod desktop;
pub mod discover;
pub mod export;
pub mod index;
pub mod listing;
pub mod login;
pub mod oauth;
pub mod oauth_server;
pub mod pace;
pub mod quota;
pub mod rotate;
pub mod store;
pub mod switch;
pub mod token;
pub mod version;
pub mod watch;
