// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the app calls the accounts it shows where no name is shown: the
//! menu bar's title and what VoiceOver reads ([`crate::icon`]), and the
//! alerts' cards, their samples and the line under a card after a switch
//! ([`crate::alerts`]). The app's own names go by number (`Tài khoản N`,
//! `Antigravity N`); an extension may give its own ([`Labels`], which
//! [`crate::ext::Hooks::account_labels`] gives for the settings in force).
//! A name it does not give stays the app's.

use crate::snapshot::{Account, Section};

/// An extension's names for the accounts the app shows, for one set of
/// settings ([`crate::ext::Hooks::account_labels`]). Every method has a
/// default that gives nothing: the app's own name or words stay.
pub trait Labels: Send + Sync {
    /// The name of the Claude account `account`, one of `section`'s (the
    /// Claude accounts as the snapshot has them, with the keys of the
    /// extension's own their list had, [`Section::list_ext`]), where no
    /// name is shown: it has a number from 1 and no email the app shows.
    /// `None`, the default, is the app's `Tài khoản N`.
    fn claude(&self, section: &Section, account: &Account) -> Option<String> {
        let _ = (section, account);
        None
    }

    /// The name of Antigravity account `number` (from 1) where no name is
    /// shown. `None`, the default, is the app's `Antigravity N`.
    fn agy(&self, number: u32) -> Option<String> {
        let _ = number;
        None
    }

    /// The names the cards "Gửi thử" sends give three Claude accounts, in
    /// place of the app's `Tài khoản 1` to `3`. `None`, the default, keeps
    /// the app's.
    fn claude_samples(&self) -> Option<[&'static str; 3]> {
        None
    }

    /// The title of alert 1's card when the accounts so named run dry, in
    /// place of the app's `Hạn mức Claude sắp cạn`, on its sample too.
    /// `None`, the default, keeps the app's.
    fn pool_title(&self) -> Option<&'static str> {
        None
    }
}
