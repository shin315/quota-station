// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The settings file with the advanced part's keys, read and written through
//! the hooks of the app's extension (the core's `settings` has the file
//! without an extension): Nâng cao, the other Macs, the clone and the
//! switch of alert 2. Every file is in a temporary folder.

use std::path::PathBuf;

use quota_station_core::advanced::alerts::leaving;
use quota_station_core::advanced::settings::{
    load, peers, Own, Peer, PeersError, SettingsHooks, PEERS_MAX,
};
use quota_station_core::alerts::{Kind, Switches};
use quota_station_core::ext::Hooks;
use quota_station_core::settings::{
    AutoBackup, CodexOptions, Language, Menubar, MenubarWindow, PoolAlert, Settings, SkillChecks,
    Theme, ThemePack,
};

/// The settings' part of the hooks the app passes with the app's
/// extension: the advanced part's.
fn hooks() -> &'static dyn Hooks {
    &SettingsHooks
}

#[test]
fn settings_round_trip_and_default_when_missing() {
    let dir = std::env::temp_dir().join(format!("qs-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("nested/settings.json");
    assert_eq!(Settings::load(&file), Settings::default());

    let s = Settings {
        machine_name: Some("Mac phòng khách".into()),
        hide_names: true,
        theme: Theme::Dark,
        theme_pack: ThemePack::ChieuLang,
        language: Language::En,
        menubar: Menubar::Codex,
        menubar_window: MenubarWindow::FableWeek,
        alerts: Switches {
            ext: serde_json::Map::from_iter([("leaving".to_string(), false.into())]),
            ..Switches::default()
        },
        codex_home: Some(PathBuf::from("/srv/example/codex")),
        codex: CodexOptions { plugins: false },
        auto_backup: AutoBackup { after_change: false, daily: true },
        skill_checks: SkillChecks { daily: false, notify: true },
        release_checks_off: true,
        cli_checks_off: true,
        backup_dir: Some(PathBuf::from("/srv/example/backups")),
        pool_alert: PoolAlert { dry_at_most: 5, room_at_least: 20 },
        setup_seen: true,
        ext: serde_json::Map::new(),
    };
    let s = {
        let mut s = s;
        let own = Own {
            peers: vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c")],
            source_dir: Some(PathBuf::from("/srv/example/quota-station")),
            advanced: true,
        };
        own.write(&mut s);
        s
    };
    s.save(&file).unwrap();
    assert_eq!(Settings::load(&file), s);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("machineName"));
    assert!(text.contains(r#""releaseChecksOff": true"#));
    assert!(text.contains(r#""cliChecksOff": true"#));
    assert!(text.contains("\"hideNames\": true"));
    assert!(text.contains("\"theme\": \"dark\""));
    assert!(text.contains("\"language\": \"en\""));
    assert!(text.contains("\"menubar\": \"codex\""));
    assert!(text.contains("\"menubarWindow\": \"fableWeek\""));
    assert!(text.contains("codexHome"));
    assert!(text.contains("\"afterChange\": false"));
    assert!(text.contains(r#""skillChecks": {"#));
    assert!(text.contains("backupDir"));
    assert!(text.contains(r#""dryAtMost": 5"#));
    for key in [r#""alias": "mac-c""#, "\"leaving\": false", "sourceDir", "\"advanced\": true"] {
        assert!(text.contains(key), "{key} in {text}");
    }

    std::fs::write(&file, "{ not json").unwrap();
    assert_eq!(Settings::load(&file), Settings::default());
}

#[test]
fn a_settings_file_from_m1_still_loads() {
    let dir = std::env::temp_dir().join(format!("qs-settings-m1-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, r#"{ "quotaAxiPath": "/opt/tools/bin/quota-axi" }"#).unwrap();
    let s = Settings::load(&file);
    // The quota-axi path of versions that ran it is left behind.
    assert_eq!(s.machine_name, None);
    assert!(!s.hide_names, "names show");
    assert_eq!(s.theme, Theme::Auto);
    assert_eq!(s.menubar, Menubar::Claude, "the menu bar follows Claude");
    assert_eq!(s.menubar_window, MenubarWindow::FiveHours, "the title shows five hours");
    // Alerts 1, 3 and 4 on.
    assert!([Kind::Pool, Kind::Login, Kind::Overwrite].iter().all(|k| s.alerts.on(k, hooks())));
    // Alert 2 on too.
    {
        assert!(s.alerts.on(&leaving(), hooks()));
        let own = Own::of(&s);
        assert_eq!(own.source_dir, None);
    }
    assert_eq!(s.codex_home, None, "Codex's own ~/.codex");
    assert!(s.ext.is_empty(), "and nothing is kept for an extension");
    let _ = std::fs::remove_dir_all(&dir);
}

fn peer(name: &str, alias: &str) -> Peer {
    Peer { name: name.into(), alias: alias.into() }
}

#[test]
fn the_single_ssh_alias_moves_into_the_list_of_other_macs() {
    let dir = std::env::temp_dir().join(format!("qs-settings-alias-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, r#"{ "machineName": "Mac A", "sshAlias": " mac-b " }"#).unwrap();
    let s = load(&file);
    assert_eq!(s.machine_name.as_deref(), Some("Mac A"));
    assert_eq!(Own::of(&s).peers, vec![peer("mac-b", "mac-b")]);

    // Saved again, only the list is written.
    s.save(&file).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains("sshAlias"), "{text}");
    assert_eq!(load(&file), s);

    // A list already there wins, and an alias ssh would read as an option is dropped.
    std::fs::write(
        &file,
        r#"{ "sshAlias": "mac-b", "peers": [{ "name": "Mac C", "alias": "mac-c" }] }"#,
    )
    .unwrap();
    assert_eq!(Own::of(&load(&file)).peers, vec![peer("Mac C", "mac-c")]);
    std::fs::write(&file, r#"{ "sshAlias": "-oProxyCommand=x" }"#).unwrap();
    assert!(Own::of(&load(&file)).peers.is_empty());
    std::fs::write(&file, r#"{ "sshAlias": "" }"#).unwrap();
    assert!(Own::of(&load(&file)).peers.is_empty());

    // A broken entry is dropped, never the whole file; the list is tidied as typed ones are.
    std::fs::write(
        &file,
        r#"{ "machineName": "Mac A", "peers": [{ "name": "Mac B" }, 7, { "alias": " mac-c " },
            { "name": "C2", "alias": "MAC-C" }, { "name": "X", "alias": "-oX=y" }] }"#,
    )
    .unwrap();
    let s = load(&file);
    assert_eq!(s.machine_name.as_deref(), Some("Mac A"));
    assert_eq!(Own::of(&s).peers, vec![peer("mac-c", "mac-c")]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn other_macs_typed_in_settings_are_tidied_and_checked() {
    let got = peers(&[peer("  Mac B ", " mac-b "), peer("", "mac-c")]).unwrap();
    assert_eq!(got, vec![peer("Mac B", "mac-b"), peer("mac-c", "mac-c")]);
    assert_eq!(peers(&[peer("Mac B", "")]), Err(PeersError::BadAlias(0)));
    assert_eq!(peers(&[peer("B", "mac-b"), peer("X", "-oX=y")]), Err(PeersError::BadAlias(1)));
    assert_eq!(peers(&[peer("B", "mac-b"), peer("B2", "MAC-B")]), Err(PeersError::SameAlias(1)));
    let many: Vec<Peer> = (0..=PEERS_MAX).map(|i| peer("M", &format!("m{i}"))).collect();
    assert_eq!(peers(&many), Err(PeersError::TooMany));
    assert_eq!(PeersError::SameAlias(1).code(), "sameAlias:1");
}

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qs-settings-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_new_install_starts_with_the_first_run_and_stays_new() {
    let dir = fresh_dir("new");
    let file = dir.join("settings.json");
    let s = Settings::open(&file, hooks());
    {
        assert!(!Own::of(&s).advanced, "Nâng cao is off on a new install");
    }
    assert!(!s.setup_seen, "the main window opens on the first run's steps");
    assert_eq!(s.pool_alert, PoolAlert { dry_at_most: 10, room_at_least: 15 });
    assert_eq!(s.backup_dir, None, "iCloud Drive until another folder is chosen");
    // The answer is written at once…
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"advanced\": false"));
    assert!(text.contains("\"setupSeen\": false"), "{text}");
    // …so the files the app writes next never make it look like an older install.
    std::fs::create_dir_all(dir.join("alerts")).unwrap();
    std::fs::write(dir.join("alerts/alerts.json"), "{}").unwrap();
    let again = Settings::open(&file, hooks());
    assert!(!Own::of(&again).advanced);
    assert!(!again.setup_seen, "until its owner finishes the steps");
    let mut seen = again;
    seen.setup_seen = true;
    seen.save(&file).unwrap();
    assert!(Settings::open(&file, hooks()).setup_seen);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn alert_two_waits_for_nang_cao() {
    // Alert 2 stays quiet while Nâng cao is off, whatever its switch says.
    let mut on = Settings::default();
    assert!(!Own::of(&on).advanced);
    on.alerts.set(&leaving(), true, hooks());
    let run = on.alert_switches(hooks());
    assert!(!run.on(&leaving(), hooks()));
    assert!(run.pool && run.login && run.overwrite);
    // Turned on in Cài đặt, it runs.
    Own::change(&mut on, |own| own.advanced = true);
    assert!(on.alert_switches(hooks()).on(&leaving(), hooks()));
}

#[test]
fn a_mac_that_had_the_app_keeps_everything_as_it_was() {
    // A settings file from before Nâng cao, with the owner's Macs.
    let dir = fresh_dir("had");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &file,
        r#"{ "peers": [{ "name": "Mac B", "alias": "mac-b" }],
             "alerts": { "leaving": true } }"#,
    )
    .unwrap();
    let s = Settings::open(&file, hooks());
    let own = Own::of(&s);
    assert!(own.advanced, "the tabs and alerts stay as they were");
    assert!(s.setup_seen, "and no first run");
    assert_eq!(s.alert_switches(hooks()), s.alerts);
    assert_eq!(own.peers, vec![peer("Mac B", "mac-b")]);
    assert_eq!(s.pool_alert, PoolAlert::default(), "alert 1 keeps 10 % and 15 %");
    assert_eq!(s.backup_dir, None, "the packs stay in iCloud Drive");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"advanced\": true") && text.contains("mac-b"), "{text}");
    // A file the version before this one wrote, Nâng cao settled: no first run either.
    let earlier = Settings::open(&file, hooks());
    std::fs::write(&file, r#"{ "advanced": false }"#).unwrap();
    assert!(Settings::open(&file, hooks()).setup_seen);
    earlier.save(&file).unwrap();
    // Turned off later in Cài đặt, it stays off.
    let mut off = s;
    Own::change(&mut off, |own| own.advanced = false);
    off.save(&file).unwrap();
    assert!(!Own::of(&Settings::open(&file, hooks())).advanced);
    let _ = std::fs::remove_dir_all(&dir);

    // No settings file, but the app's folder holds what it wrote: an older install too.
    let dir = fresh_dir("had-folder");
    std::fs::create_dir_all(dir.join("claude")).unwrap();
    std::fs::write(dir.join("claude/accounts.json"), "{}").unwrap();
    let s = Own::of(&Settings::open(&dir.join("settings.json"), hooks()));
    assert!(s.advanced);
    let _ = std::fs::remove_dir_all(&dir);

    // A settings file that cannot be read keeps everything on and is not written over.
    let dir = fresh_dir("had-broken");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), "{ half").unwrap();
    let s = Own::of(&Settings::open(&dir.join("settings.json"), hooks()));
    assert!(s.advanced);
    assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "{ half");
    let _ = std::fs::remove_dir_all(&dir);

    // A hidden file alone (Finder's) is not the app's.
    let dir = fresh_dir("hidden");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".DS_Store"), "").unwrap();
    assert!(!Own::of(&Settings::open(&dir.join("settings.json"), hooks())).advanced);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A settings file with everything Nâng cao keeps (Nâng cao on, the other
/// Macs, the clone, alert 2) loads and is written back unchanged.
#[test]
fn a_settings_file_with_nang_cao_on_is_written_back_as_it_was() {
    let dir = fresh_dir("either-build");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    let written = r#"{
  "machineName": "Mac A",
  "peers": [{ "name": "Mac B", "alias": "mac-b" }, { "name": "Mac C", "alias": "mac-c" }],
  "alerts": { "leaving": true },
  "sourceDir": "/srv/example/quota-station",
  "advanced": true
}"#;
    std::fs::write(&file, written).unwrap();
    let s = Settings::open(&file, hooks());
    let own = Own::of(&s);
    assert!(own.advanced);
    assert_eq!(own.peers, vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c")]);
    assert_eq!(own.source_dir, Some(PathBuf::from("/srv/example/quota-station")));
    assert!(s.alerts.on(&leaving(), hooks()));
    assert!(own.advanced_shown());
    let run = s.alert_switches(hooks());
    assert!(run.on(&leaving(), hooks()));
    assert!(run.pool && run.login && run.overwrite);

    // A change made in this build (the theme) keeps every key the other wrote.
    let changed = Settings { theme: Theme::Dark, ..s.clone() };
    changed.save(&file).unwrap();
    let back = Settings::open(&file, hooks());
    assert_eq!(back, changed);
    let text = std::fs::read_to_string(&file).unwrap();
    for key in [r#""alias": "mac-c""#, "sourceDir", r#""advanced": true"#] {
        assert!(text.contains(key), "{key} in {text}");
    }

    // The other way: a new install writes Nâng cao off and reads it so,
    // with everything else as it was.
    let dir = fresh_dir("either-build-new");
    let file = dir.join("settings.json");
    let new = Settings::open(&file, hooks());
    let own = Own::of(&new);
    assert!(!own.advanced && !own.advanced_shown());
    Settings { machine_name: Some("Mac D".into()), ..new }.save(&file).unwrap();
    let again = Settings::open(&file, hooks());
    assert_eq!(again.machine_name.as_deref(), Some("Mac D"));
    assert!(!Own::of(&again).advanced);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(fresh_dir("either-build"));
}

/// The `alerts` of a file `text` once a build with `hooks` opened it and
/// wrote it again; `None` when the file has no `alerts`.
fn alerts_written(tag: &str, text: &str, hooks: &dyn Hooks) -> Option<serde_json::Value> {
    let dir = fresh_dir(tag);
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, text).unwrap();
    Settings::open(&file, hooks).save(&file).unwrap();
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    written.get("alerts").cloned()
}

/// The switch of alert 2 reads and writes as the build before this socket
/// wrote it: all four keys while any switch is off its default, none (no
/// `alerts`) while every one is at it; a key missing reads as its default
/// (on).
#[test]
fn the_switch_of_alert_two_reads_and_writes_as_before() {
    use serde_json::json;
    let settled = |alerts: serde_json::Value| {
        json!({ "setupSeen": true, "advanced": true, "alerts": alerts }).to_string()
    };
    let all = json!({ "pool": true, "leaving": false, "login": true, "overwrite": true });
    assert_eq!(alerts_written("two-all", &settled(all.clone()), hooks()), Some(all));
    assert_eq!(
        alerts_written("two-app", &settled(json!({ "login": false })), hooks()),
        Some(json!({ "pool": true, "leaving": true, "login": false, "overwrite": true }))
    );
    let default = json!({ "pool": true, "leaving": true, "login": true, "overwrite": true });
    assert_eq!(alerts_written("two-default", &settled(default), hooks()), None);
    assert_eq!(
        alerts_written("two-none", r#"{ "setupSeen": true, "advanced": true }"#, hooks()),
        None
    );

    // Set in Cài đặt and back: the same keys again.
    let mut s = Settings::default();
    s.alerts.set(&leaving(), false, hooks());
    assert_eq!(
        serde_json::to_value(&s.alerts).unwrap(),
        json!({ "pool": true, "leaving": false, "login": true, "overwrite": true })
    );
    s.alerts.set(&leaving(), true, hooks());
    assert!(s.alerts.is_default(), "written as no `alerts` at all");
    // What Cài đặt › Thông báo shows: all four.
    assert_eq!(
        serde_json::to_value(s.alerts.shown(hooks())).unwrap(),
        json!({ "pool": true, "leaving": true, "login": true, "overwrite": true })
    );
}
