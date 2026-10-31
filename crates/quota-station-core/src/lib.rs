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
// @private:begin
//! The private part is built only with the `author-tools` feature, which
//! a release build and the community edition leave out
//! (`community/README.md`): the rotation ([`rotation`], its run in
//! [`claude::rotate`] from its own LaunchAgent, [`switcher`] reading its
//! decision and log, and the [`pause`] flag), [`peers`] and the copy of the
//! rotation file to the owner's other Macs over SSH, `agents::copy` (the
//! agents' setup to another Mac), [`update`] (`git pull --ff-only` and
//! `scripts/install.sh` in the clone set in settings), [`firstmate`]
//! (Firstmate's `quota-pool.last`, read only at the path set in settings),
//! alerts 2 and 5, and the copy of the Claude accounts in claude-swap's
//! layout. Without it the settings keep their values, so a settings file
//! passes through either build. It is made for macOS and builds on Unix
//! only.
//!
// @private:end
//! What the core asks of the operating system (the secret store, the
//! process list, opening a URL) goes through the seams in [`os`].

#[cfg(all(feature = "author-tools", not(unix)))]
compile_error!("`author-tools` (the tools for the author's own Macs) builds on Unix only");

pub mod agents;
pub mod alerts;
#[cfg(feature = "antigravity-accounts")]
pub mod antigravity;
pub mod app_dir;
pub mod claude;
pub mod cli_tools;
pub mod codex;
pub mod detail;
#[cfg(feature = "author-tools")]
pub mod firstmate;
pub mod history;
pub mod icon;
pub mod local_time;
pub mod login_item;
pub mod os;
#[cfg(feature = "author-tools")]
pub mod pause;
#[cfg(feature = "author-tools")]
pub mod peers;
pub mod quota;
pub mod release;
#[cfg(feature = "author-tools")]
pub mod rotation;
pub mod runner;
pub mod schedule;
pub mod sealed;
pub mod sessions;
pub mod settings;
pub mod snapshot;
pub mod stats;
#[cfg(feature = "author-tools")]
pub mod switcher;
pub mod switching;
mod time_util;
#[cfg(feature = "author-tools")]
pub mod update;
#[cfg(windows)]
pub mod win;

pub use icon::{icon, Icon, Tone};
pub use snapshot::Snapshot;
