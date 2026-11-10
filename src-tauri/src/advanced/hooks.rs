// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The hooks the advanced part gives the core ([`crate::ext::Hooks`]),
//! which [`super::Advanced::hooks`] hands over: the core's own
//! ([`quota_station_core::advanced::hooks`]), which the command line
//! without the app runs with too.

pub use quota_station_core::advanced::hooks::AdvancedHooks;
