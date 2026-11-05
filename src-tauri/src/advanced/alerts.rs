// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! What the advanced part reads for its alerts after each read
//! ([`crate::ext::Extension::alert_read`]): the rotation file, for
//! alert 2 when the report has no rotation, and the rotation's last log
//! line, for alert 9 (a switch of "Tự chuyển khi hết quota"). Both are
//! drawn by `quota_station_core::advanced::alerts` ([`Leaving`]).

use quota_station_core::advanced::alerts::Source as Leaving;
use quota_station_core::advanced::pool;
use quota_station_core::alerts::{Alert, ExtPass, ExtSource};
use quota_station_core::claude::Paths;
use quota_station_core::rotation::Rules;
use quota_station_core::switcher::{self, LogLine};

use crate::ext::{AlertInput, AlertRead, Kept};

/// What was read for one pass of the alert rules.
pub struct Read {
    /// The rotation file's rules, when it read.
    rules: Option<Rules>,
    /// The rotation's last log line, when there is one.
    last_log: Option<LogLine>,
}

/// Reads the rotation file this Mac follows and the rotation's last log line.
pub fn read(input: &AlertInput<'_>) -> Read {
    let paths = Paths::new(input.home, input.app_dir.path());
    let last_log = paths.in_use().then(|| switcher::last_log_line(&paths.log())).flatten();
    Read { rules: pool::rules(&paths), last_log }
}

impl ExtSource for Read {
    fn alerts(&self, pass: &mut ExtPass<'_>) -> Vec<Alert> {
        Leaving { rules: self.rules.as_ref(), last_log: self.last_log.as_ref() }.alerts(pass)
    }
}

impl AlertRead for Read {
    fn source(&self) -> &dyn ExtSource {
        self
    }

    /// Nothing is kept of what was read.
    fn done(self: Box<Self>, kept: &Kept) {
        let _ = kept;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use quota_station_core::advanced::alerts::{leaving, KINDS};
    use quota_station_core::advanced::labels::Roles;
    use quota_station_core::advanced::pool::Policy;
    use quota_station_core::advanced::settings::Own;
    use quota_station_core::alerts::{self, Facts, Memory};
    use quota_station_core::app_dir::AppDir;
    use quota_station_core::quota::Response;
    use quota_station_core::rotation::{self, Expire, Stamp};
    use quota_station_core::settings::{PoolAlert, Settings};
    use time::format_description::well_known::Rfc3339;
    use time::{OffsetDateTime, UtcOffset};

    use super::*;
    use crate::advanced::hooks::AdvancedHooks;
    use crate::ext::Hooks;

    fn temp(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qs-author-alerts-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The advanced part's hooks name alert 2, as the core's advanced
    /// module keeps it.
    #[test]
    fn the_advanced_parts_hooks_name_alert_two() {
        assert_eq!(AdvancedHooks.alert_kinds(), &KINDS);
    }

    /// Team 2 to 4, personal 1 from 8 to 23, 2 leaving at `ends`.
    fn rules(ends: OffsetDateTime) -> Rules {
        Rules {
            team: vec![2, 3, 4],
            personal: Some(1),
            personal_from: 8,
            personal_to: 23,
            full: 90.0,
            fable_full: 95.0,
            expire: vec![Expire { slot: 2, at: ends.format(&Rfc3339).unwrap() }],
        }
    }

    /// The accounts the app keeps without the rotation's keys: 2 in use and
    /// nearly out, 3 with little left, 5 outside the rotation with room.
    fn report() -> Response {
        let slot = |n: u32, left: f64| {
            serde_json::json!({
                "slot": n, "active": n == 2, "status": "fresh", "effectivePercentRemaining": left
            })
        };
        serde_json::from_value(serde_json::json!({
            "generatedAt": "2026-01-16T07:20:00.000Z",
            "providers": [{
                "provider": "claude", "windows": [], "state": { "status": "fresh" },
                "claudeAccounts": {
                    "activeSlot": 2, "slots": [slot(2, 5.0), slot(3, 12.0), slot(5, 90.0)]
                }
            }]
        }))
        .unwrap()
    }

    /// The rotation file is read for alert 2, which comes from it when the
    /// report has no rotation; without a rotation file no rules are read.
    #[test]
    fn the_rotation_file_is_read_for_alert_two() {
        let dir = temp("rotation-two");
        let (home, app) = (dir.join("home"), AppDir::new(dir.join("app")));
        let settings = Settings::default();
        let input = AlertInput { settings: &settings, home: &home, app_dir: &app };
        assert!(read(&input).rules.is_none());

        let paths = Paths::new(&home, app.path());
        let now = OffsetDateTime::from_unix_timestamp(1_768_548_000).unwrap();
        let rules = rules(now + time::Duration::minutes(20));
        std::fs::create_dir_all(&paths.dir).unwrap();
        let stamp = Stamp::new(now, "Mac A");
        std::fs::write(paths.rotation(), rotation::render(&rules, &stamp, None)).unwrap();
        let mut settings = Settings::default();
        // Nâng cao holds alert 2 while it is off.
        Own::change(&mut settings, |own| own.advanced = true);
        let input = AlertInput { settings: &settings, home: &home, app_dir: &app };
        let read: Box<dyn AlertRead> = Box::new(read(&input));
        let policy = Policy::read(&paths);
        assert_eq!(policy.rules.as_ref(), Some(&rules));

        let claude = report();
        let facts = Facts {
            now,
            offset: UtcOffset::from_hms(7, 0, 0).unwrap(),
            claude: Some(&claude),
            pool_policy: Some(&policy),
            #[cfg(feature = "antigravity-accounts")]
            agy: Vec::new(),
            #[cfg(feature = "antigravity-accounts")]
            overwrite: None,
            ext: Some(read.source()),
            pool: PoolAlert::default(),
            labels: Some(&Roles),
        };
        let mut memory = Memory::default();
        let switches = settings.alert_switches(&AdvancedHooks);
        alerts::evaluate(&mut memory, &facts, &switches, &AdvancedHooks);
        let kinds: Vec<_> = memory.recent.iter().rev().map(|r| r.alert.kind.clone()).collect();
        assert_eq!(kinds, [alerts::Kind::Pool, leaving()], "5 is outside the rotation");
        read.done(&Kept::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The advanced part's hooks give the core its policy of the pool, read
    /// from the rotation file, and alert 1's words for it.
    #[test]
    fn the_private_parts_hooks_give_the_rotation_as_the_pools_policy() {
        let dir = temp("hooks");
        let paths = Paths::new(dir.join("home"), &dir.join("app"));
        assert!(AdvancedHooks.claude_pool(&paths).is_some(), "even without a rotation file");
        assert_eq!(AdvancedHooks.pool_answers(), &quota_station_core::advanced::pool::ANSWERS);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
