//! Quota Station's core: reads quota-axi, builds the snapshot the menu bar
//! and the popover show, and paces the reads. No UI and no AppKit, so all of
//! it is tested with `cargo test`.
//!
//! Read-only by design: nothing here switches an account or writes outside
//! the app's own settings file.

pub mod axi;
pub mod icon;
pub mod runner;
pub mod schedule;
pub mod settings;
pub mod snapshot;
mod time_util;

pub use icon::{icon, Icon, Tone};
pub use snapshot::Snapshot;
