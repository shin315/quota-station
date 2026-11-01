// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's hooks for an extension (`quota_station_core::ext`).

mod common;

use common::ext::TestHooks;
use quota_station_core::ext::{self, Hooks, NoHooks};

/// What every caller holds: hooks behind a shared reference, from any
/// thread.
fn shared(hooks: &dyn Hooks) -> &dyn Hooks {
    hooks
}

#[test]
fn hooks_pass_as_a_shared_reference_between_threads() {
    fn send_sync<T: Send + Sync + ?Sized>(_: &T) {}
    send_sync(shared(&NoHooks));
    send_sync(shared(&TestHooks));
    send_sync(ext::none());
}
