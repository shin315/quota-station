//! Quota Station's core: reads quota-axi, builds the snapshot the menu bar
//! and the popover show, paces the reads, and switches Claude when the owner
//! asks. No UI and no AppKit, so all of it is tested with `cargo test`.
//!
//! It writes the app's own files (settings, the Antigravity index and quota
//! readings in [`antigravity`], the switch [`history`] and what the
//! [`alerts`] sent and were answered), the switcher's pause flag, the
//! rotation file ([`rotation`]), which it also copies to the owner's other
//! Macs over SSH ([`peers`]), and the app's own Keychain items for the
//! Antigravity accounts it logs in. The one command it runs that switches
//! anything is `cswap switch N --json` ([`switching`]). The one it runs that
//! writes anything elsewhere is the [`peers`] script over `ssh`, which
//! replaces the rotation file on another Mac (keeping a backup there).
//! Everything else it runs only reads: quota-axi, the switcher's
//! `--dry-run` (which reads claude-swap's accounts and switches nothing),
//! `ssh` reading the other Macs' rotation file, `launchctl list`, and
//! `plutil -p` on Antigravity's `Info.plist`. It reads Firstmate's
//! `quota-pool.last` ([`firstmate`]) only at the path set in settings.

pub mod alerts;
pub mod antigravity;
pub mod axi;
pub mod cswap;
pub mod firstmate;
pub mod history;
pub mod icon;
pub mod local_time;
pub mod pause;
pub mod peers;
pub mod rotation;
pub mod runner;
pub mod schedule;
pub mod sessions;
pub mod settings;
pub mod snapshot;
pub mod switcher;
pub mod switching;
mod time_util;

pub use icon::{icon, Icon, Tone};
pub use snapshot::Snapshot;
