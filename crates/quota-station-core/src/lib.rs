// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! Quota Station's core: reads the quota of Claude, Codex and Antigravity
//! itself ([`quota`]), builds the snapshot the menu bar and the popover
//! show, paces the reads, and switches Claude when the owner asks. No UI and
//! no AppKit, so all of it is tested with `cargo test`.
//!
//! It writes the app's own files (settings, through [`app_dir`]; the
//! Antigravity index and quota readings in `antigravity`, which only the
//! `antigravity-accounts` feature builds, CC BY-NC-SA 4.0: `NOTICE`; the switch [`history`] and what the
//! [`alerts`] sent and were answered), and the app's own Keychain items for the
//! Antigravity accounts it logs in and for MCP secrets. It keeps the Claude
//! accounts itself ([`claude`]): their logins in its own Keychain items, a
//! switch that writes Claude Code's login item and `~/.claude.json` under
//! Claude Code's locks, the usage reads, and the session folders
//! `quota-station claude run N` uses ([`switching`] is the manual switch
//! from the windows). The one it runs that writes anything elsewhere is
//! `claude mcp add-json`/`remove` ([`agents`]) from the MCP tab.
//! Everything else it runs only reads: `agy -p /quota`, `ps` and `lsof` for
//! the running Antigravity ([`quota::agy`]), `security` for Claude Code's
//! login on a Mac where the app keeps no Claude account ([`quota::claude`]),
//! `launchctl getenv NAME` for the variables that override a login
//! ([`agents::env_check`], which keeps whether a value is there, never the
//! value), and `plutil -p` on Antigravity's `Info.plist`. [`login_item`]
//! turns launching at login on or off through a backend the app gives it.
//!
//! [`codex`] switches Codex between the ChatGPT login and a custom provider
//! when the owner confirms: it writes only the provider choice in Codex's
//! `config.toml` (keeping a backup without the keys), never `auth.json` or
//! the MCP list, and runs `codex app-server daemon restart` only when the
//! owner asks.
//!
//! [`agents`] is the MCP tab (M9): after a preview the owner confirmed, it
//! writes the `[mcp_servers.<name>]` tables of Codex's `config.toml`, the
//! `mcpServers` of Antigravity's `mcp_config.json`, and Claude Code's list
//! through `claude mcp add-json`/`remove --scope user` (Claude Code writes
//! `~/.claude.json` itself). Each file is backed up first with its secrets
//! left out; secrets the app must keep go to its own Keychain items.
//!
//! [`stats`] (M16) reads Claude Code's and Codex's session logs on this
//! Mac for their token counts, model, time and project folder only, and
//! keeps those counts in its own index; no conversation text is kept.
//!
//! The advanced part, which Cài đặt › Nâng cao turns on: the rotation
//! ([`rotation`], its run in [`claude::rotate`] from its own LaunchAgent, or
//! the app's own timer on Windows, [`switcher`] reading its decision and
//! log, and the [`pause`] flag), `peers` and the copy of the rotation file
//! to the owner's other Macs over SSH, `agents::copy` (the agents' setup to
//! another Mac), `update` (`git pull --ff-only` and `scripts/install.sh` in
//! the clone set in settings), and alert 2. [`advanced`] holds what the
//! app's extension and these modules share (the advanced part's settings
//! keys). An app without that extension keeps those keys' values, so a
//! settings file passes through it. It builds on macOS and Windows; what
//! only a Mac does (`peers`, `agents::copy`, `update` and the LaunchAgent)
//! builds on Unix only.
//!
//! What the core asks of the operating system (the secret store, the
//! process list, opening a URL) goes through the seams in [`os`].
//!
//! [`ext`] holds the hooks an extension may give the core; without one
//! ([`ext::NoHooks`]) each does nothing. [`labels`] is what an extension
//! may call the accounts the app shows.

pub mod advanced;
pub mod agents;
pub mod alerts;
#[cfg(feature = "antigravity-accounts")]
pub mod antigravity;
pub mod app_dir;
pub mod claude;
pub mod cli_tools;
pub mod codex;
pub mod detail;
pub mod ext;
pub mod history;
pub mod icon;
pub mod labels;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod local_time;
pub mod login_item;
pub mod os;
pub mod pause;
#[cfg(unix)]
pub mod peers;
pub mod quota;
pub mod release;
pub mod rotation;
pub mod runner;
pub mod schedule;
pub mod sealed;
pub mod self_update;
pub mod sessions;
pub mod settings;
pub mod snapshot;
pub mod stats;
pub mod switcher;
pub mod switching;
pub mod time_util;
#[cfg(unix)]
pub mod update;
#[cfg(windows)]
pub mod win;

pub use icon::{icon, Icon, Tone};
pub use snapshot::Snapshot;

/// The core's own folder, which holds its tests' fixtures
/// (`tests/fixtures/`): the shared test helpers read them from here,
/// whichever crate's tests include them.
#[cfg(feature = "test-support")]
pub const TEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
