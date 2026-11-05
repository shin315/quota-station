// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The advanced part's views in the model every window draws
//! (`ext::Extension::model`): the switcher (`switcher`), Cài đặt › Nâng cao
//! (`advanced`) and the rotation file and the other Macs (`rotation`),
//! `authorTools` in what the build has
//! (`ext::Extension::capabilities`), and its keys in the Claude accounts'
//! view (`ext::Extension::view_keys`: `rotation` and `legacy`) and, on a
//! Mac, in the app's own row (the update from source, [`super::update`]).
//!
//! The app draws its model under its own lock, so these read only what
//! [`ModelInput`] hands them and take only the advanced part's own lock
//! ([`AdvancedState::lock`]), each view on its own, as they did when they
//! were the app's.

use quota_station_core::advanced::held;
use quota_station_core::advanced::settings::Own;
use quota_station_core::claude::Paths;
use quota_station_core::local_time;
#[cfg(unix)]
use quota_station_core::peers::{self, PeerState};
use quota_station_core::rotation::{self, FileState, Rules, Stamp};
use quota_station_core::runner;
use quota_station_core::switcher::{Decision, LogLine};
use serde::Serialize;
use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::state::AdvancedState;
use crate::ext::{ModelInput, View};

/// The three views, by the keys the windows read them at.
pub fn model(input: &ModelInput<'_>) -> Map<String, Value> {
    let author = input.kept.get::<AdvancedState>().unwrap_or_default();
    let mut map = Map::new();
    map.insert("switcher".into(), json(&switcher(input, &author, OffsetDateTime::now_utc())));
    map.insert("advanced".into(), json(&advanced(input)));
    map.insert("rotation".into(), json(&rotation(input, &author)));
    map
}

/// `authorTools` in what the build has: the tools made for the author's
/// own Macs (Cài đặt › Nâng cao, the rules editor and the other Macs, the
/// copy to another Mac and the update from source). Without them
/// none of their commands is built and the windows show no switch, tab or
/// setting for them.
pub fn capabilities() -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("authorTools".into(), Value::Bool(true));
    map
}

/// Its keys in the app's views: in the Claude accounts', whether the
/// rotation's LaunchAgent is loaded (`rotation`, `null` when that cannot be
/// told) and an older switcher still loaded (`legacy`: nothing switches
/// while it is). In the app's own row, on a Mac only, the update from
/// source (`source`, `sourceStatus`, `bundled`, `updating`, `outcome`,
/// `error`, `log`: [`super::update::SourceView`]); none on Windows.
pub fn view_keys(view: View, input: &ModelInput<'_>) -> Map<String, Value> {
    let mut map = Map::new();
    match view {
        View::ClaudeAccounts => {
            let author = input.kept.get::<AdvancedState>().unwrap_or_default();
            let own = author.lock();
            map.insert("rotation".into(), json(&own.switcher.service));
            map.insert("legacy".into(), json(&own.switcher.legacy));
        }
        View::App =>
        {
            #[cfg(unix)]
            if let Value::Object(source) = json(&super::update::view(input)) {
                map.extend(source);
            }
        }
    }
    map
}

fn json(view: &impl Serialize) -> Value {
    serde_json::to_value(view).expect("a view is plain data")
}

fn claude_paths(input: &ModelInput<'_>) -> Paths {
    Paths::new(input.home, input.app_dir.path())
}

/// Cài đặt › Nâng cao: on, the tabs and switches made for the author's own
/// Macs show.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedView {
    pub on: bool,
}

pub fn advanced(input: &ModelInput<'_>) -> AdvancedView {
    AdvancedView { on: Own::of(input.settings).advanced_shown() }
}

/// The rotation file and the other Macs (F06).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationView {
    /// `~/Library/Application Support/com.shin315.quota-station/claude/rotation`
    /// once the app keeps the accounts, else the old switcher's
    /// (`~/` + `quota_station_core::rotation::FILE`).
    pub path: String,
    /// `ok`, `missing`, `invalid` (the file's rules refuse it) or `unreadable`.
    pub status: &'static str,
    pub rules: Option<Rules>,
    /// Why the file is invalid, in the rule's words, or the read error.
    pub reason: Option<String>,
    /// Sent back with a save, so a file changed meanwhile is not overwritten.
    pub version: Option<String>,
    /// The `# updated = … by …` line of a valid file; `None` for a file the app
    /// never saved, and for a missing or invalid one.
    pub stamp: Option<Stamp>,
    pub ignored_keys: Vec<String>,
    /// What "Tạo từ mặc định" starts from ([`Rules::starter`]): the accounts
    /// in the rotation, each in turn. There is no built-in rotation.
    pub defaults: Rules,
    pub saving: bool,
    #[cfg(unix)]
    pub peers: Vec<PeerView>,
    /// Windows copies the file to no other Mac: always `[]`.
    #[cfg(not(unix))]
    pub peers: [(); 0],
    /// An `ssh` to copy the file with; never on Windows.
    pub ssh: bool,
}

#[cfg(unix)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub name: String,
    pub alias: String,
    /// `None`: nothing sent to it yet.
    pub state: Option<PeerState>,
    /// The state is about this Mac's file as it is now.
    pub current: bool,
    /// Its rules, when it holds another copy (newer, or edited there).
    pub their_rules: Option<Rules>,
    /// A copy is going to it now.
    pub pushing: bool,
}

/// The rotation file as it is on disk now, and each other Mac against it
/// (none on Windows, which copies it to no other Mac).
pub fn rotation(input: &ModelInput<'_>, author: &AdvancedState) -> RotationView {
    let file = claude_paths(input).rotation_file();
    let state = rotation::load(&file);
    let stamp = state.stamp().cloned();
    let version = state.version();
    let (status, rules, reason, ignored_keys) = match &state {
        FileState::Missing => ("missing", None, None, Vec::new()),
        FileState::Unreadable(e) => ("unreadable", None, Some(e.clone()), Vec::new()),
        FileState::Invalid { reason, .. } => ("invalid", None, Some(reason.clone()), Vec::new()),
        FileState::Ok { parsed, .. } => {
            ("ok", Some(parsed.rules.clone()), None, parsed.ignored_keys.clone())
        }
    };
    let own = author.lock();
    #[cfg(unix)]
    let peers = Own::of(input.settings)
        .peers
        .iter()
        .map(|p| {
            let record = own.sync.peers.get(&p.alias);
            PeerView {
                name: p.name.clone(),
                alias: p.alias.clone(),
                current: record.zip(version.as_ref()).is_some_and(|(r, v)| r.against == *v),
                state: record.map(|r| r.state.clone()),
                their_rules: record
                    .and_then(|r| r.their_text.as_deref())
                    .and_then(|t| rotation::parse(t).ok())
                    .map(|p| p.rules),
                pushing: own.pushing.contains(&p.alias),
            }
        })
        .collect();
    RotationView {
        path: runner::display_path(&file, input.home),
        status,
        rules,
        reason,
        version,
        stamp,
        ignored_keys,
        defaults: Rules::starter(
            &input
                .claude_accounts
                .iter()
                .filter(|a| held::enabled(a))
                .map(|a| a.slot)
                .collect::<Vec<_>>(),
        ),
        saving: own.rotation_saving,
        #[cfg(unix)]
        peers,
        #[cfg(not(unix))]
        peers: [],
        #[cfg(unix)]
        ssh: peers::find_ssh(std::env::var_os("PATH").as_deref()).is_some(),
        #[cfg(not(unix))]
        ssh: false,
    }
}

/// The rotation as the windows show it (F07).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitcherView {
    /// The app keeps the Claude accounts on this Mac, so its rotation can run.
    pub installed: bool,
    pub pause: Option<PauseView>,
    pub dry_run: Option<DryRunView>,
    pub dry_run_error: Option<&'static str>,
    /// `false`: a dry run while the flag held did not say paused, so the
    /// rotation on this Mac does not read the pause flag.
    pub guard: Option<bool>,
    pub last_log: Option<LogLine>,
    /// The app's rotation LaunchAgent is loaded.
    pub service: Option<bool>,
    /// An older switcher still loaded: its label.
    pub legacy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseView {
    /// RFC 3339; `None` pauses until the owner resumes.
    pub until: Option<String>,
    pub reason: Option<String>,
    /// The flag held when this model was built.
    pub holds: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunView {
    pub decision: Decision,
    pub raw: String,
    /// When the dry run ran, RFC 3339.
    pub at: String,
}

pub fn switcher(
    input: &ModelInput<'_>,
    author: &AdvancedState,
    now: OffsetDateTime,
) -> SwitcherView {
    let own = author.lock();
    let sw = &own.switcher;
    let stamp = local_time::stamp(now);
    let rfc = |t: OffsetDateTime| t.format(&Rfc3339).ok();
    SwitcherView {
        installed: claude_paths(input).in_use(),
        pause: sw.pause.as_ref().map(|f| PauseView {
            until: f.until.as_deref().and_then(local_time::instant).and_then(rfc),
            reason: f.reason.clone(),
            holds: f.holds(&stamp),
        }),
        dry_run: sw.dry_run.as_ref().map(|r| DryRunView {
            decision: r.run.decision.clone(),
            raw: r.run.raw.clone(),
            at: rfc(r.at).unwrap_or_default(),
        }),
        dry_run_error: sw.dry_run_error,
        guard: sw.guard(),
        last_log: sw.last_log.clone(),
        service: sw.service,
        legacy: sw.legacy.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use quota_station_core::app_dir::AppDir;
    use quota_station_core::claude::state::AccountOut;
    use quota_station_core::pause;
    use quota_station_core::settings::Settings;

    use super::*;
    use crate::ext::Kept;

    /// What the app hands the advanced part to draw from, owned by the test.
    struct Drawn {
        kept: Kept,
        settings: Settings,
        home: PathBuf,
        app_dir: AppDir,
        accounts: Vec<AccountOut>,
    }

    impl Drawn {
        fn new(home: PathBuf, author: Arc<AdvancedState>) -> Drawn {
            let mut kept = Kept::default();
            kept.keep(author);
            let app_dir = AppDir::new(home.join("app"));
            Drawn { kept, settings: Settings::default(), home, app_dir, accounts: Vec::new() }
        }

        fn input(&self) -> ModelInput<'_> {
            ModelInput {
                kept: &self.kept,
                settings: &self.settings,
                home: &self.home,
                app_dir: &self.app_dir,
                claude_accounts: &self.accounts,
            }
        }

        fn author(&self) -> Arc<AdvancedState> {
            self.kept.get::<AdvancedState>().expect("kept")
        }

        fn switcher(&self) -> SwitcherView {
            switcher(&self.input(), &self.author(), OffsetDateTime::now_utc())
        }

        #[cfg(unix)]
        fn rotation(&self) -> RotationView {
            rotation(&self.input(), &self.author())
        }
    }

    /// The Claude accounts' view carries the rotation's LaunchAgent and an
    /// older switcher as the advanced part last saw them; `null` before it
    /// looked. The app's own row gets the update from source's keys on a
    /// Mac, and none on Windows.
    #[test]
    fn the_claude_accounts_carry_the_launch_agent_and_an_older_switcher() {
        let author = Arc::new(AdvancedState::default());
        let drawn = Drawn::new(PathBuf::new(), author.clone());
        let keys = |view| Value::Object(view_keys(view, &drawn.input()));
        assert_eq!(
            keys(View::ClaudeAccounts),
            serde_json::json!({ "rotation": null, "legacy": null })
        );
        author.lock().switcher.service = Some(true);
        author.lock().switcher.legacy = Some("com.cswap.auto".into());
        assert_eq!(
            keys(View::ClaudeAccounts),
            serde_json::json!({ "rotation": true, "legacy": "com.cswap.auto" })
        );
        #[cfg(unix)]
        assert_eq!(
            keys(View::App),
            serde_json::json!({
                "source": null,
                "sourceStatus": "notSet",
                "bundled": false,
                "updating": null,
                "outcome": null,
                "error": null,
                "log": "~/app/update/install.log",
            })
        );
        #[cfg(not(unix))]
        assert_eq!(keys(View::App), serde_json::json!({}));
    }

    #[cfg(unix)]
    fn sample() -> Rules {
        Rules { team: vec![2, 3, 4], personal: Some(1), ..Rules::starter(&[]) }
    }

    #[test]
    fn a_pause_counts_until_a_dry_run_shows_the_switcher_ignores_it() {
        use crate::advanced::state::DryRunRecord;
        use quota_station_core::switcher::{parse_decision, DryRun};
        let now = OffsetDateTime::now_utc();
        let later = local_time::minute(now + time::Duration::hours(1));
        let author = Arc::new(AdvancedState::default());
        let drawn = Drawn::new(PathBuf::new(), author.clone());
        assert!(!author.lock().switcher.paused(now));

        author.lock().switcher.pause =
            Some(pause::Flag { until: Some(later.clone()), reason: Some("lý do".into()) });
        assert!(author.lock().switcher.paused(now));
        assert_eq!(author.lock().switcher.guard(), None, "no dry run yet");
        let view = drawn.switcher();
        let pause = view.pause.unwrap();
        assert!(pause.holds);
        let until = OffsetDateTime::parse(&pause.until.unwrap(), &Rfc3339).unwrap();
        assert_eq!(local_time::minute(until), later);

        let record = |line: &str, flag_held| DryRunRecord {
            run: DryRun { decision: parse_decision(line), raw: line.into() },
            at: now,
            flag_held,
        };
        author.lock().switcher.dry_run = Some(record("stay", true));
        let own = author.lock();
        assert_eq!(own.switcher.guard(), Some(false), "the switcher decided as if not paused");
        assert!(!own.switcher.paused(now));
        drop(own);
        author.lock().switcher.dry_run =
            Some(record(&format!("paused until {later}: lý do"), true));
        assert_eq!(author.lock().switcher.guard(), Some(true));
        assert!(author.lock().switcher.paused(now));
        // A dry run from before the pause says nothing about the guard.
        author.lock().switcher.dry_run = Some(record("stay", false));
        assert_eq!(author.lock().switcher.guard(), None);

        let expired = local_time::minute(now - time::Duration::minutes(5));
        author.lock().switcher.pause = Some(pause::Flag { until: Some(expired), reason: None });
        assert!(!author.lock().switcher.paused(now));
        assert!(!drawn.switcher().pause.unwrap().holds);
    }

    #[cfg(unix)]
    #[test]
    fn each_other_mac_is_shown_against_the_file_on_disk_now() {
        use quota_station_core::advanced::settings::Peer;
        use quota_station_core::peers::PeerRecord;
        let dir = std::env::temp_dir().join(format!("qs-state-rotation-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let author = Arc::new(AdvancedState::default());
        let mut drawn = Drawn::new(dir.clone(), author.clone());
        assert_eq!(drawn.rotation().status, "missing");

        let at = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
        let stamp = Stamp::new(at, "Mac A");
        let text = rotation::render(&sample(), &stamp, None);
        let file = rotation::path(&dir);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, &text).unwrap();
        let peer = |name: &str, alias: &str| Peer { name: name.into(), alias: alias.into() };
        Own::change(&mut drawn.settings, |own| {
            own.peers =
                vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c"), peer("Mac D", "mac-d")];
        });
        let newer = Stamp::new(at + time::Duration::minutes(9), "Mac B");
        let mut theirs = sample();
        theirs.full = 88.0;
        let mut own = author.lock();
        own.sync.peers.insert(
            "mac-b".into(),
            PeerRecord {
                state: PeerState::Newer { their: newer.clone() },
                against: rotation::version(&text),
                their_text: Some(rotation::render(&theirs, &newer, None)),
            },
        );
        own.sync.peers.insert(
            "mac-c".into(),
            PeerRecord {
                state: PeerState::Synced { at: "2026-01-16T07:00:00Z".into() },
                against: "2026-01-16T13:00:00+07:00".into(),
                their_text: None,
            },
        );
        drop(own);

        let view = drawn.rotation();
        assert_eq!(view.status, "ok");
        assert_eq!(view.rules, Some(sample()));
        assert_eq!(view.stamp, Some(stamp));
        assert_eq!(view.version, Some(rotation::version(&text)));
        assert_eq!(
            view.path, "~/.config/cswap-prefer-team/rotation",
            "before the app keeps the accounts"
        );
        let [b, c, d] = &view.peers[..] else { panic!("three Macs") };
        assert!(b.current);
        assert_eq!(b.their_rules.as_ref().map(|r| r.full), Some(88.0));
        assert!(!c.current, "Mac C has an older file of this Mac");
        assert!(d.state.is_none() && !d.current, "Mac D was never sent anything");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
