// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-FileCopyrightText: 2026 Kun Chen (quota-axi)
// SPDX-License-Identifier: MIT

// The reuse and stale rules are quota-axi's (`src/cache.ts` and
// `servableStaleWindows`, `staleFromCache` in `src/providers/common.ts`,
// MIT, https://github.com/kunchenguid/quota-axi, 0.1.66): the last good
// reading answers while younger than the age asked for and while the login
// it was read with is the same; a failed read shows it as `stale`, keeping
// only the windows whose reset is still ahead, or, without a reset, whose
// cycle cannot have ended yet; a confirmed sign-out forgets it.

//! Each provider's last good reading, in `quota/<provider>.json` in the
//! app's folder: windows and times only, and a fingerprint of the login it
//! was read with (a hash, never the login).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::{parse_iso, semantics, Provider, ProviderId, Window};
use crate::app_dir::AppDir;

pub const DIR: &str = "quota";
const VERSION: u32 = 1;
/// A reading younger than this answers a timed read (quota-axi's
/// `--max-age 120s`, which the app passed).
pub const MAX_AGE_SECONDS: i64 = 120;
const FIVE_HOURS: f64 = 5.0 * 3600.0;
const WEEK: f64 = 7.0 * 86400.0;
const SHORTEST_MONTH: f64 = 28.0 * 86400.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Stored {
    version: u32,
    /// The login the reading was taken with, as [`Cache::save`] was told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    login: Option<String>,
    row: Provider,
}

/// The cache of one provider in the app's folder, or of one of its
/// accounts.
#[derive(Debug, Clone)]
pub struct Cache {
    app: AppDir,
    provider: ProviderId,
    /// `codex-2`: one Codex account's own file.
    name: Option<String>,
}

impl Cache {
    pub fn new(app: &AppDir, provider: ProviderId) -> Cache {
        Cache { app: app.clone(), provider, name: None }
    }

    /// One account's cache, `<provider>-<account>.json`.
    pub fn account(app: &AppDir, provider: ProviderId, account: u32) -> Cache {
        Cache { app: app.clone(), provider, name: Some(format!("{}-{account}", provider.as_str())) }
    }

    fn path(&self) -> std::path::PathBuf {
        let name = self.name.as_deref().unwrap_or(self.provider.as_str());
        self.app.path().join(DIR).join(format!("{name}.json"))
    }

    fn stored(&self) -> Option<Stored> {
        let text = self.app.read_to_string(&self.path()).ok()?;
        serde_json::from_str::<Stored>(&text).ok().filter(|s| s.version == VERSION)
    }

    /// The last good reading, as stored.
    pub fn last(&self) -> Option<Provider> {
        self.stored().map(|s| s.row)
    }

    /// Keeps `row`, a fresh reading taken with `login`.
    pub fn save(&self, row: &Provider, login: Option<&str>) {
        let stored = Stored { version: VERSION, login: login.map(String::from), row: row.clone() };
        if let Ok(text) = serde_json::to_string_pretty(&stored) {
            let _ = self.app.write_private(&self.path(), &(text + "\n"));
        }
    }

    /// Forgets the last reading: the login it was taken with is gone.
    pub fn retire(&self) {
        let _ = self.app.remove(&self.path());
    }

    /// The last reading when it was taken with `login` under `max_age`
    /// seconds before `now`: the read needs to ask nothing.
    pub fn reusable(
        &self,
        login: Option<&str>,
        max_age: i64,
        now: OffsetDateTime,
    ) -> Option<Provider> {
        if max_age <= 0 {
            return None;
        }
        let stored = self.stored()?;
        if stored.login.as_deref() != login || stored.row.state.status != "fresh" {
            return None;
        }
        let at = parse_iso(stored.row.state.refreshed_at.as_deref()?)?;
        let age = (now - at).whole_milliseconds();
        let mut row = stored.row;
        (age >= 0 && age < i128::from(max_age) * 1000).then(|| {
            semantics::apply(&mut row, now);
            row
        })
    }

    /// The last reading shown as stale for a read that failed with `error`,
    /// when any of its windows is still true at `now`.
    pub fn stale(&self, error: &str, now: OffsetDateTime) -> Option<Provider> {
        stale_from(&self.last()?, error, now)
    }
}

/// `cached` as a stale reading of a read that failed with `error`, with the
/// windows still servable at `now`; `None` when none is.
pub fn stale_from(cached: &Provider, error: &str, now: OffsetDateTime) -> Option<Provider> {
    let windows = servable(cached, now);
    if windows.is_empty() {
        return None;
    }
    let mut row = cached.clone();
    row.windows = windows;
    row.state.status = "stale".into();
    row.state.error = Some(error.to_string());
    semantics::apply(&mut row, now);
    Some(row)
}

/// The windows of `cached` still true at `now`: a reset ahead, or no reset
/// and a reading younger than the window's cycle.
pub fn servable(cached: &Provider, now: OffsetDateTime) -> Vec<Window> {
    let read = cached.state.refreshed_at.as_deref().and_then(parse_iso);
    if read.is_some_and(|r| r > now) {
        return Vec::new();
    }
    let age = read.map(|r| (now - r).as_seconds_f64());
    cached
        .windows
        .iter()
        .filter(|w| match w.resets_at.as_deref().and_then(parse_iso) {
            Some(reset) => reset > now,
            None => {
                let cycle =
                    w.window_seconds.filter(|s| *s > 0.0).unwrap_or(match w.kind.as_deref() {
                        Some("session") => FIVE_HOURS,
                        Some("weekly") => WEEK,
                        Some("monthly") => SHORTEST_MONTH,
                        _ => 0.0,
                    });
                cycle > 0.0 && age.is_some_and(|a| a < cycle)
            }
        })
        .cloned()
        .collect()
}

/// A fingerprint of a login's text, to tell two logins apart without
/// keeping either: the first 16 hex digits of its SHA-256.
pub fn fingerprint(text: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(text.as_bytes()).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> OffsetDateTime {
        parse_iso(s).unwrap()
    }

    fn reading() -> Provider {
        let mut ahead =
            Window::used("five_hour", "session", 40.0, Some("2026-01-16T09:00:00Z".into()));
        ahead.window_seconds = Some(FIVE_HOURS);
        let gone = Window::used("weekly", "weekly", 10.0, Some("2026-01-16T07:00:00Z".into()));
        let resetless = Window::used("seven_day", "weekly", 5.0, None);
        Provider::fresh(ProviderId::Codex, vec![ahead, gone, resetless], at("2026-01-16T07:10:00Z"))
    }

    #[test]
    fn a_stale_reading_keeps_only_windows_still_true() {
        let row =
            stale_from(&reading(), "Codex quota request timed out", at("2026-01-16T07:20:00Z"))
                .unwrap();
        assert_eq!(row.state.status, "stale");
        assert_eq!(row.state.refreshed_at.as_deref(), Some("2026-01-16T07:10:00.000Z"));
        let ids: Vec<&str> = row.windows.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["five_hour", "seven_day"], "the passed reset is dropped");
        // A week on, the resetless weekly window may have reset unseen.
        assert!(stale_from(&reading(), "x", at("2026-01-23T08:00:00Z")).is_none());
        // A clock that went back cannot age anything.
        assert!(stale_from(&reading(), "x", at("2026-01-16T07:00:00Z")).is_none());
    }

    #[test]
    fn a_reading_is_reused_only_young_and_for_the_same_login() {
        let dir = std::env::temp_dir().join(format!("qs-quota-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cache = Cache::new(&AppDir::new(&dir), ProviderId::Codex);
        assert!(cache.reusable(Some("a"), 120, at("2026-01-16T07:10:30Z")).is_none());
        cache.save(&reading(), Some("a"));
        assert!(cache.reusable(Some("a"), 120, at("2026-01-16T07:11:59Z")).is_some());
        assert!(cache.reusable(Some("a"), 120, at("2026-01-16T07:12:00Z")).is_none(), "too old");
        assert!(
            cache.reusable(Some("b"), 120, at("2026-01-16T07:10:30Z")).is_none(),
            "another login"
        );
        assert!(cache.reusable(Some("a"), 0, at("2026-01-16T07:10:30Z")).is_none(), "a fresh read");
        cache.retire();
        assert!(cache.last().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
