//! Quota Station's core: reads quota-axi, builds the snapshot the menu bar
//! and the popover show, paces the reads, and switches Claude when the owner
//! asks. No UI and no AppKit, so all of it is tested with `cargo test`.
//!
//! It writes the app's own files (settings, the Antigravity index and quota
//! readings in [`antigravity`], the switch [`history`] and what the
//! [`alerts`] sent and were answered), the switcher's pause flag, the
//! rotation file ([`rotation`]), which it also copies to the owner's other
//! Macs over SSH ([`peers`]), and the app's own Keychain items for the
//! Antigravity accounts it logs in and for MCP secrets. The one command it runs that switches
//! anything is `cswap switch N --json` ([`switching`]). The ones it runs
//! that write anything elsewhere are the [`peers`] script over `ssh`, which
//! replaces the rotation file on another Mac (keeping a backup there), and
//! `claude mcp add-json`/`remove` ([`agents`]) from the MCP tab.
//! Everything else it runs only reads: quota-axi, `cswap list --json`
//! (every Claude account's windows, email and organization), the switcher's
//! `--dry-run` (which reads claude-swap's accounts and switches nothing),
//! `ssh` reading the other Macs' rotation file, `launchctl list`,
//! `launchctl getenv NAME` for the variables that override a login
//! ([`agents::env_check`], which keeps whether a value is there, never the
//! value), and
//! `plutil -p` on Antigravity's `Info.plist`. It reads Firstmate's
//! `quota-pool.last` ([`firstmate`]) only at the path set in settings.
//!
//! [`update`] runs, only when the owner presses `Cập nhật từ mã nguồn`,
//! `git pull --ff-only` in the clone set in settings and that clone's
//! `scripts/install.sh`, which replaces the installed app. [`login_item`]
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

pub mod agents;
pub mod alerts;
pub mod antigravity;
pub mod axi;
pub mod codex;
pub mod cswap;
pub mod detail;
pub mod firstmate;
pub mod history;
pub mod icon;
pub mod local_time;
pub mod login_item;
pub mod pause;
pub mod peers;
pub mod rotation;
pub mod runner;
pub mod schedule;
pub mod sessions;
pub mod settings;
pub mod snapshot;
pub mod stats;
pub mod switcher;
pub mod switching;
mod time_util;
pub mod update;

pub use icon::{icon, Icon, Tone};
pub use snapshot::Snapshot;
