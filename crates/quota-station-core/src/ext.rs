// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's side of an extension: the hooks the core calls where an
//! extension may add to what it reads, keeps and shows. Each hook is a
//! method of [`Hooks`] whose default does nothing, so [`NoHooks`] (and any
//! extension that leaves a hook alone) leaves the core exactly as it is
//! without one.
//!
//! The core never finds an extension on its own: a caller that has one
//! passes `&dyn Hooks` in, as a parameter or as a field of the input it
//! already takes, and the core's own callers and tests pass [`NoHooks`].
//! The app's extension (`quota_station::ext::Extension`) hands its hooks
//! over with `Extension::hooks`.

/// The hooks an extension gives the core. Every method has a default that
/// does nothing; an extension overrides the ones it needs.
pub trait Hooks: Send + Sync {}

/// No extension: every hook does nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoHooks;

impl Hooks for NoHooks {}

/// The hooks of no extension, for a caller that has none to pass.
pub fn none() -> &'static dyn Hooks {
    &NoHooks
}
