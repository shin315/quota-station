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
//!
//! Today's hooks:
//!
//! - the settings file ([`Settings::open`]): the extension's keys, kept in
//!   [`Settings::ext`], are read by [`Hooks::tidy_settings`] and settled
//!   for a Mac new to the app by [`Hooks::settle_settings`], and
//!   [`Hooks::settled_key`] names the key that tells a file was settled;
//! - which alerts run ([`Settings::alert_switches`]): [`Hooks::alert_switches`];
//! - the alerts ([`crate::alerts`]): the kinds the extension adds to the
//!   app's, with their switches in the settings file and the words of their
//!   buttons ([`Hooks::alert_kinds`]), and the card "Gửi thử" sends for one
//!   ([`Hooks::alert_sample`]); their alerts at each pass of the rules come
//!   from what the extension read for it ([`crate::alerts::Facts::ext`]);
//! - `quota-station claude …` ([`crate::claude::cli::run`]): the commands
//!   the extension adds to the app's ([`Hooks::claude_command`]) and the
//!   usage that names them ([`Hooks::claude_usage`]);
//! - the Claude pool: which of the accounts the app keeps count in it and
//!   the room it has, at the read ([`crate::quota::claude::own_now`]) and in
//!   alert 1's check ([`crate::alerts::pool_check`]), by the extension's
//!   policy ([`Hooks::claude_pool`]), and the words alert 1's card has for
//!   the pool so counted ([`Hooks::pool_answers`]);
//! - the names of the accounts the app shows where no name is shown, on
//!   the menu bar's title ([`crate::icon`]) and on the alerts' cards
//!   ([`crate::alerts`]), and alert 1's title: the extension's for the
//!   settings in force ([`Hooks::account_labels`]);
//! - the snapshot the windows and the menu bar draw: what the extension
//!   adds to it once the app has built it ([`Hooks::decorate_snapshot`]),
//!   in its own keys among the app's ([`Fields`]);
//! - the switch history ([`crate::history`]): the lines the extension adds
//!   to the app's, each with its source and reason ([`Hooks::history_lines`]),
//!   and its keys on the lines of the owner's manual switches
//!   ([`Hooks::manual_line`]);
//! - the owner's manual switch of the Claude account
//!   ([`crate::switching::switch_claude`], and `quota-station claude
//!   switch`): what the extension does around it ([`Hooks::manual_switch`]);
//! - whether the app refreshes a Claude login's token while it reads
//!   ([`Hooks::may_refresh_logins`]);
//! - the Claude accounts' file (`claude/accounts.json`): the keys the
//!   extension keeps on each account there, and what it shows of them in
//!   the state file and on `quota-station claude status`
//!   ([`Hooks::claude_account_keys`]); the read of the accounts carries
//!   them on each account's slot ([`crate::quota::Slot::ext`]);
//! - the import of claude-swap's accounts ([`crate::claude::import::import`]):
//!   what the extension brings over of its own, at the end of the import's
//!   outcome ([`Hooks::claude_imported`]).

use std::fmt;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use time::UtcOffset;

use crate::alerts::{Alert, ExtAnswer, ExtKind, Switches};
use crate::app_dir::AppDir;
use crate::claude::cli::Cli;
use crate::claude::store::AccountKeys;
use crate::claude::Paths;
use crate::history::ExtLine;
use crate::labels::Labels;
use crate::quota::claude::PoolPolicy;
use crate::quota::Response;
use crate::settings::Settings;
use crate::snapshot::Snapshot;
use crate::switching::AroundSwitch;

/// The hooks an extension gives the core. Every method has a default that
/// does nothing; an extension overrides the ones it needs.
pub trait Hooks: Send + Sync {
    /// The settings key whose value, `true` or `false`, tells that
    /// [`Settings::open`] already settled whether this Mac is new to the
    /// app. `None`, the default, is the app's own (`setupSeen`). An
    /// extension that names its own key has its keys settled
    /// ([`Hooks::settle_settings`]) the first time a build with it opens a
    /// file without that key.
    fn settled_key(&self) -> Option<&'static str> {
        None
    }

    /// The extension's keys of the settings file as [`Settings::open`]
    /// read them, put in the shape the extension keeps them in. `found` is
    /// false when there was no file, or none the app could read, and
    /// `ext` is empty. Called on every open, before anything else reads
    /// them. The default leaves them as they were read.
    fn tidy_settings(&self, ext: &mut Map<String, Value>, found: bool) {
        let _ = (ext, found);
    }

    /// The extension's keys on the first open that settles the file:
    /// `had_app` tells a Mac that had the app before from a new one. What
    /// is set here is written with the file at once. The default sets
    /// nothing.
    fn settle_settings(&self, ext: &mut Map<String, Value>, had_app: bool) {
        let _ = (ext, had_app);
    }

    /// Which of the alerts switched on in `settings` run: `on` is the
    /// app's answer (the switches themselves). The default keeps it.
    fn alert_switches(&self, settings: &Settings, on: Switches) -> Switches {
        let _ = settings;
        on
    }

    /// The kinds of alert the extension adds to the app's own: each one's
    /// number, name, switch and the words of its buttons. Their switches
    /// are kept in the settings file's `alerts` beside the app's, and their
    /// alerts, which the extension draws at each pass of the rules
    /// ([`crate::alerts::ExtSource`]), go out as the app's own do. The
    /// default adds none.
    fn alert_kinds(&self) -> &'static [ExtKind] {
        &[]
    }

    /// The card "Gửi thử" sends for the extension's kind `name`, at `now`:
    /// its tone, title, body and buttons, with `claude` the names the cards
    /// give three Claude accounts. Its key, kind and time are the app's to
    /// set, and so is the mark that makes its buttons change nothing.
    /// `None`, the default, sends nothing.
    fn alert_sample(
        &self,
        name: &str,
        now: i64,
        offset: UtcOffset,
        claude: [&str; 3],
    ) -> Option<Alert> {
        let _ = (name, now, offset, claude);
        None
    }

    /// A command of `quota-station claude …` that is not one of the app's:
    /// `args` are the words after `claude`, the command first. The
    /// extension answers with the exit code, having written what it prints
    /// to `out` and `err`, or `None`, the default, when the command is not
    /// its own either: then the app prints the usage on `err` and exits 2.
    /// The app's own commands never reach the extension.
    fn claude_command(
        &self,
        cli: &Cli,
        args: &[String],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Option<i32> {
        let _ = (cli, args, out, err);
        None
    }

    /// The usage `quota-station claude` prints (for `help`, and on stderr
    /// for a command or an argument it does not know), when the
    /// extension's commands belong in it. `None`, the default, is the
    /// app's own ([`crate::claude::cli::USAGE`]).
    fn claude_usage(&self) -> Option<&'static str> {
        None
    }

    /// The policy of the Claude pool where the app keeps the accounts at
    /// `paths`, read from what is there now: which accounts count in the
    /// pool and the room it has ([`PoolPolicy`]). Asked once for each read
    /// of the accounts and each pass of the alert rules. `None`, the
    /// default: every account counts, each with what it has left.
    fn claude_pool(&self, paths: &Paths) -> Option<Box<dyn PoolPolicy>> {
        let _ = paths;
        None
    }

    /// What alert 1's card (the Claude pool) says under itself once one of
    /// these buttons is pressed, where the pool the extension counts
    /// ([`Hooks::claude_pool`]) has its own words, in place of the app's
    /// for that button. The button still does what it does. The default
    /// has none.
    fn pool_answers(&self) -> &'static [ExtAnswer] {
        &[]
    }

    /// The extension's names for the accounts the app shows where no name
    /// is shown, and alert 1's title, with `settings` as they are now
    /// ([`Labels`]). Asked wherever the app names them: each build of the
    /// menu bar's icon, each pass of the alert rules, a sample card and
    /// the line under a card after a switch. `None`, the default: the
    /// app's own names (`Tài khoản N`, `Antigravity N`).
    fn account_labels(&self, settings: &Settings) -> Option<Box<dyn Labels>> {
        let _ = settings;
        None
    }

    /// Adds the extension's part to `snapshot`, which the app has just
    /// built from its reports (`claude` is the Claude report it was built
    /// from): keys of the extension's own in the [`Fields`] of its
    /// sections, accounts and pools, and, where the extension counts a
    /// pool its own way, that pool ([`crate::snapshot::PoolKind::Ext`]).
    /// Called wherever the app builds the snapshot it shows. The default
    /// adds nothing.
    fn decorate_snapshot(&self, snapshot: &mut Snapshot, claude: Option<&Response>) {
        let _ = (snapshot, claude);
    }

    /// The lines the extension adds to the switch history, read now from
    /// what it keeps under `home` (the owner's home folder) or in `app`
    /// (the app's folder), in any order: each with its time, the account
    /// before and after, the name its source goes by and the reason it
    /// gives ([`ExtLine`]). The app shows them among its own, newest first
    /// (its own first at the same second); one that is a switch counts in
    /// the stats' timeline of the Claude accounts, and any of them explains
    /// a change of the account in use the readings show soon after it.
    /// Asked whenever the app gathers the history. The default adds none.
    fn history_lines(&self, home: &Path, app: &AppDir) -> Vec<ExtLine> {
        let _ = (home, app);
        Vec::new()
    }

    /// What the extension does around one manual switch of the Claude
    /// account the owner asks for ([`AroundSwitch`]): `asked` holds the
    /// arguments the window asked for the switch with (empty from a
    /// terminal), among them any of the extension's own. Asked before
    /// anything else about the switch, by the app's switch and by
    /// `quota-station claude switch`. An error refuses the switch at once,
    /// with that error for the window. `Ok(None)`, the default: nothing
    /// around the switch.
    fn manual_switch(
        &self,
        asked: &Map<String, Value>,
    ) -> Result<Option<Box<dyn AroundSwitch>>, String> {
        let _ = asked;
        Ok(None)
    }

    /// The extension's keys on the History tab's line of one of the
    /// owner's manual switches of the Claude account, before its `result`,
    /// from the keys the switch kept in the journal (`kept`,
    /// [`AroundSwitch::journal`]). The default shows none.
    fn manual_line(&self, kept: &Fields) -> Fields {
        let _ = kept;
        Fields::new()
    }

    /// Whether a read of the Claude accounts the app keeps may refresh a
    /// login's token now (the app's reader, and `quota-station claude
    /// status --refresh`). Asked at each such read. The default: it may.
    fn may_refresh_logins(&self) -> bool {
        true
    }

    /// What the extension keeps on each Claude account of the app's file,
    /// and shows of it where the app tells other programs about the
    /// accounts ([`AccountKeys`]). Asked wherever the app opens its Claude
    /// accounts with the extension ([`crate::claude::store::Store::with_hooks`]).
    /// `None`, the default: the file keeps only the app's keys.
    fn claude_account_keys(&self) -> Option<Arc<dyn AccountKeys>> {
        None
    }

    /// What the extension brings over of its own when the app imports
    /// claude-swap's accounts into the Claude accounts it keeps at `paths`
    /// ([`crate::claude::import::import`]), once the accounts and their
    /// readings are in: the keys it returns end the import's outcome
    /// ([`crate::claude::import::Imported::ext`]), in the order it put
    /// them. Asked at the end of every import that read claude-swap's
    /// accounts. The default brings nothing.
    fn claude_imported(&self, paths: &Paths) -> Fields {
        let _ = paths;
        Fields::new()
    }
}

/// The keys an extension adds to one of the app's JSON objects (the
/// snapshot's sections, accounts and pools, the Claude read's list of
/// accounts and its slots, the Claude accounts' file and state file, a
/// manual switch's outcome and its lines in the history, an import's
/// outcome), with their values. In the snapshot each key is written right
/// after the app's own key it was put after ([`Fields::insert_after`]), so
/// the object reads with the extension's keys where they belong, even
/// where the app leaves that key out; a key put after none
/// ([`Fields::insert`]), or after a key the object does not have, comes at
/// the end, in the order the keys were put. The Claude read's list, a
/// manual switch's outcome and an import's write them all at their end, in
/// that order; a manual switch's lines in the history before their
/// `result`; a slot of the Claude read after its `active`; an account of the Claude accounts'
/// file and of the state file after its `alias`. Read back from JSON (a
/// report), an object keeps here every key it does not know, written back
/// where its extension's keys go.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fields {
    entries: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
struct Field {
    after: Option<&'static str>,
    key: String,
    value: Value,
    /// The value as it was written, for one put that way
    /// ([`Fields::insert_written`]): written so, not from `value`.
    written: Option<Written>,
}

impl Field {
    fn write<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
        match &self.written {
            Some(written) => map.serialize_entry(&self.key, &written.0),
            None => map.serialize_entry(&self.key, &self.value),
        }
    }
}

/// A value's JSON text, as it was written.
#[derive(Debug, Clone)]
struct Written(Box<RawValue>);

impl PartialEq for Written {
    fn eq(&self, other: &Written) -> bool {
        self.0.get() == other.0.get()
    }
}

impl Fields {
    /// No keys.
    pub fn new() -> Self {
        Fields::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The value of `key`, when it is here.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|f| f.key == key).map(|f| &f.value)
    }

    /// Puts `key`, written right after the object's own key `after`; a
    /// key already here leaves its place for the new one.
    pub fn insert_after(&mut self, after: &'static str, key: impl Into<String>, value: Value) {
        self.put(Some(after), key.into(), value);
    }

    /// Puts `key`, written at the end of the object; a key already here
    /// leaves its place for the new one.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) {
        self.put(None, key.into(), value);
    }

    /// Puts `key` with `value` written as serde writes it, at the end of
    /// the object: an object keeps its keys in the order it writes them,
    /// where a [`Value`] would sort them. A key already here leaves its
    /// place for the new one.
    pub fn insert_written(
        &mut self,
        key: impl Into<String>,
        value: &impl Serialize,
    ) -> serde_json::Result<()> {
        let written = serde_json::value::to_raw_value(value)?;
        let value = serde_json::from_str(written.get())?;
        let key = key.into();
        self.remove(&key);
        self.entries.push(Field { after: None, key, value, written: Some(Written(written)) });
        Ok(())
    }

    /// Takes `key` out, with its value.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        let at = self.entries.iter().position(|f| f.key == key)?;
        Some(self.entries.remove(at).value)
    }

    /// Every key with its value, in the order they were put.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|f| (f.key.as_str(), &f.value))
    }

    fn put(&mut self, after: Option<&'static str>, key: String, value: Value) {
        self.remove(&key);
        self.entries.push(Field { after, key, value, written: None });
    }
}

/// The keys in the order they were put, as one object (or, flattened, at
/// the end of the object that has them).
impl Serialize for Fields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for f in &self.entries {
            f.write(&mut map)?;
        }
        map.end()
    }
}

/// Every key of an object, each put at the end.
impl<'de> Deserialize<'de> for Fields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Keys;
        impl<'de> Visitor<'de> for Keys {
            type Value = Fields;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Fields, A::Error> {
                let mut fields = Fields::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    fields.insert(key, value);
                }
                Ok(fields)
            }
        }
        deserializer.deserialize_map(Keys)
    }
}

/// Writes one of the app's objects with an extension's [`Fields`] among
/// its own keys: each own key in turn ([`Placed::entry`]), each followed by
/// the extension's keys put after it, and the rest of them at the end
/// ([`Placed::end`]).
pub(crate) struct Placed<'a, M> {
    map: M,
    fields: &'a Fields,
    seen: Vec<&'static str>,
}

impl<'a, M: SerializeMap> Placed<'a, M> {
    pub(crate) fn start<S: Serializer<SerializeMap = M>>(
        serializer: S,
        fields: &'a Fields,
    ) -> Result<Self, S::Error> {
        Ok(Placed { map: serializer.serialize_map(None)?, fields, seen: Vec::new() })
    }

    /// The app's own `key`, then the extension's keys put after it.
    pub(crate) fn entry<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), M::Error> {
        self.map.serialize_entry(key, value)?;
        self.after(key)
    }

    /// The app's own `key` when it has a value (left out otherwise), then
    /// the extension's keys put after it, either way.
    pub(crate) fn optional<T: Serialize>(
        &mut self,
        key: &'static str,
        value: Option<&T>,
    ) -> Result<(), M::Error> {
        if let Some(value) = value {
            self.map.serialize_entry(key, value)?;
        }
        self.after(key)
    }

    fn after(&mut self, key: &'static str) -> Result<(), M::Error> {
        self.seen.push(key);
        for f in self.fields.entries.iter().filter(|f| f.after == Some(key)) {
            f.write(&mut self.map)?;
        }
        Ok(())
    }

    /// The extension's keys put after none of the object's own, and the end.
    pub(crate) fn end(mut self) -> Result<M::Ok, M::Error> {
        let seen = std::mem::take(&mut self.seen);
        for f in &self.fields.entries {
            if !f.after.is_some_and(|a| seen.contains(&a)) {
                f.write(&mut self.map)?;
            }
        }
        self.map.end()
    }
}

/// No extension: every hook does nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoHooks;

impl Hooks for NoHooks {}

/// The hooks of no extension, for a caller that has none to pass.
pub fn none() -> &'static dyn Hooks {
    &NoHooks
}
