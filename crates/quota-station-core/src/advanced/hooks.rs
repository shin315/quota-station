// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The hooks the advanced part gives the core ([`crate::ext::Hooks`]): the
//! app's extension for Cài đặt › Nâng cao hands them over, and the command
//! line without the app (`quota-station-cli`) runs with them.
//!
//! Filled so far: the settings file (the advanced part's keys, how a Mac new to
//! the app starts Nâng cao, and alert 2 held while Nâng cao is off), from
//! `super::settings`; the advanced part's commands of
//! `quota-station claude …` (the rotation, its LaunchAgent, and an account held
//! out of the rotation or put back), from `super::cli`;
//! and its kind of alert (2, an account leaving the rotation) with the card
//! "Gửi thử" sends for it, from `super::alerts`; and the
//! rotation as the policy of the Claude pool, with alert 1's words for it, from
//! `super::pool`; and the accounts' names by their place
//! in the rotation while Nâng cao shows, with alert 1's title for it, from
//! `super::labels`; and the rotation's part of the
//! snapshot (each account's place in it, the leaves it, its personal hours and
//! its room), from `super::snapshot`; and the switcher's
//! switches and primes as lines of the switch history, with the hold on the
//! lines of the owner's switches, from `super::history`;
//! and around the owner's manual switch the older switcher's refusal, the hold
//! that pauses the rotation and the rotation's switch back, with no token
//! refreshed while an older switcher is loaded, from
//! `super::switch`; and the accounts held out of the
//! rotation, kept on the Claude accounts' file and shown in the state file and
//! on `quota-station claude status`, from `super::held`;
//! and the old switcher's rotation file and priming brought over on an import
//! from another account switcher, from `super::import`.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::advanced::alerts::{self, AlertHooks};
use crate::advanced::cli::{self, CliHooks};
use crate::advanced::held::HeldHooks;
use crate::advanced::history::HistoryHooks;
use crate::advanced::import::ImportHooks;
use crate::advanced::labels::LabelHooks;
use crate::advanced::pool::PoolHooks;
use crate::advanced::settings::SettingsHooks;
use crate::advanced::snapshot::SnapshotHooks;
use crate::advanced::switch::SwitchHooks;
use crate::alerts::{Alert, ExtAnswer, ExtKind, Switches};
use crate::app_dir::AppDir;
use crate::claude::cli::Cli;
use crate::claude::store::AccountKeys;
use crate::claude::Paths;
use crate::ext::Fields;
use crate::history::ExtLine;
use crate::labels::Labels;
use crate::quota::claude::PoolPolicy;
use crate::quota::Response;
use crate::settings::Settings;
use crate::switching::AroundSwitch;
use crate::Snapshot;
use serde_json::{Map, Value};
use time::UtcOffset;

use crate::ext::Hooks;

/// The advanced part's hooks.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdvancedHooks;

impl Hooks for AdvancedHooks {
    fn settled_key(&self) -> Option<&'static str> {
        SettingsHooks.settled_key()
    }

    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        SettingsHooks.tidy_settings(ext, found);
    }

    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        SettingsHooks.settle_settings(ext, had_app);
    }

    fn alert_switches(&self, settings: &Settings, on: Switches) -> Switches {
        SettingsHooks.alert_switches(settings, on)
    }

    fn alert_kinds(&self) -> &'static [ExtKind] {
        &alerts::KINDS
    }

    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        AlertHooks.alert_sample(name, now, offset, claude)
    }

    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        CliHooks::this_program().claude_command(cli, args, out, err)
    }

    fn claude_usage(&self) -> Option<&'static str> {
        Some(cli::USAGE)
    }

    fn claude_pool(&self, paths: &Paths) -> Option<Box<dyn PoolPolicy>> {
        PoolHooks.claude_pool(paths)
    }

    fn pool_answers(&self) -> &'static [ExtAnswer] {
        PoolHooks.pool_answers()
    }

    fn account_labels(&self, settings: &Settings) -> Option<Box<dyn Labels>> {
        LabelHooks.account_labels(settings)
    }

    fn decorate_snapshot(&self, snapshot: &mut Snapshot, claude: Option<&Response>) {
        SnapshotHooks.decorate_snapshot(snapshot, claude);
    }

    fn history_lines(&self, home: &Path, app: &AppDir) -> Vec<ExtLine> {
        HistoryHooks.history_lines(home, app)
    }

    fn manual_switch(
        &self,
        asked: &Map<String, Value>,
    ) -> Result<Option<Box<dyn AroundSwitch>>, String> {
        SwitchHooks::system().manual_switch(asked)
    }

    fn manual_line(&self, kept: &Fields) -> Fields {
        HistoryHooks.manual_line(kept)
    }

    fn may_refresh_logins(&self) -> bool {
        SwitchHooks::system().may_refresh_logins()
    }

    fn claude_account_keys(&self) -> Option<Arc<dyn AccountKeys>> {
        HeldHooks.claude_account_keys()
    }

    fn claude_imported(&self, paths: &Paths) -> Fields {
        ImportHooks.claude_imported(paths)
    }
}
