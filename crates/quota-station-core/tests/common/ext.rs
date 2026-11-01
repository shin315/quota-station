// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The core's test hooks (`quota_station_core::ext::Hooks`): each core
//! socket's tests fill its hook here and check that the core calls it, and
//! that `NoHooks` leaves the core as it is.

use quota_station_core::ext::Hooks;

/// Hooks a test can see the core call.
#[derive(Debug, Default)]
pub struct TestHooks;

impl Hooks for TestHooks {}
