// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's names for the accounts the app shows, which the app
//! knows only through the hooks ([`crate::ext::Hooks::account_labels`]):
//! with Cài đặt › Nâng cao on, a Claude account without a name shown goes
//! by its place in the rotation (Cá nhân, then Team A, B… in slot order;
//! `Tài khoản N` for one in no rotation) and an Antigravity account as
//! Ultra N, and alert 1 speaks of the rotation's quota. With Nâng cao off
//! the app's own names stay.
//!
//! [`LabelHooks`] gives the core these; [`Roles`] is the names themselves.

use crate::ext::Hooks;
use crate::labels::Labels;
use crate::settings::Settings;
use crate::snapshot::{Account, Section};

use super::settings::Own;
use super::snapshot::{role_in, rotation_of, Role};

const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The advanced part's hooks for the accounts' names: [`Roles`] while
/// Nâng cao shows, none otherwise. The app's extension
/// gives the core these.
#[derive(Debug, Clone, Copy, Default)]
pub struct LabelHooks;

impl Hooks for LabelHooks {
    fn account_labels(&self, settings: &Settings) -> Option<Box<dyn Labels>> {
        Own::of(settings).advanced_shown().then(|| Box::new(Roles) as Box<dyn Labels>)
    }
}

/// The accounts by their place in the rotation.
#[derive(Debug, Clone, Copy, Default)]
pub struct Roles;

impl Labels for Roles {
    fn claude(&self, section: &Section, account: &Account) -> Option<String> {
        let rotation = rotation_of(section);
        let role = |n: u32| role_in(rotation.as_ref(), n);
        match role(account.number) {
            Role::Personal => Some("Cá nhân".into()),
            Role::Team => {
                let mut team: Vec<u32> = section
                    .accounts
                    .iter()
                    .map(|a| a.number)
                    .filter(|n| role(*n) == Role::Team)
                    .collect();
                team.sort_unstable();
                let place = team.iter().position(|n| *n == account.number).unwrap_or(0);
                Some(match LETTERS.get(place) {
                    Some(l) => format!("Team {}", *l as char),
                    None => format!("Team {}", account.number),
                })
            }
            Role::Unknown => None,
        }
    }

    fn agy(&self, number: u32) -> Option<String> {
        Some(format!("Ultra {number}"))
    }

    fn claude_samples(&self) -> Option<[&'static str; 3]> {
        Some(["Team A", "Team B", "Cá nhân"])
    }

    fn pool_title(&self) -> Option<&'static str> {
        Some("Hạn mức Claude để tự chuyển sắp cạn")
    }
}
