//! Quota Station's core: reads quota-axi, builds the snapshot the menu bar
//! and the popover show, paces the reads, and switches Claude when the owner
//! asks. No UI and no AppKit, so all of it is tested with `cargo test`.
//!
//! It writes only the app's own settings file and the switcher's pause flag.
//! The one command it runs that changes anything is `cswap switch N --json`
//! ([`switching`]). Everything else it runs only reads: quota-axi, the
//! switcher's `--dry-run` (which reads claude-swap's accounts and switches
//! nothing) and `launchctl list`.

pub mod axi;
pub mod cswap;
pub mod icon;
pub mod local_time;
pub mod pause;
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
