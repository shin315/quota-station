// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use quota_station_core::alerts::{Kind, Switches};
use quota_station_core::settings::{
    machine_name, peers, AutoBackup, CodexOptions, Menubar, MenubarWindow, Peer, PeersError,
    PoolAlert, Settings, SkillChecks, Theme, MACHINE_NAME_MAX, PEERS_MAX,
};

#[test]
fn settings_round_trip_and_default_when_missing() {
    let dir = std::env::temp_dir().join(format!("qs-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("nested/settings.json");
    assert_eq!(Settings::load(&file), Settings::default());

    let s = Settings {
        quota_axi_path: Some(PathBuf::from("/opt/tools/bin/quota-axi")),
        machine_name: Some("Mac phòng khách".into()),
        peers: vec![peer("Mac B", "mac-b"), peer("Mac C", "mac-c")],
        hide_names: true,
        theme: Theme::Dark,
        menubar: Menubar::Codex,
        menubar_window: MenubarWindow::FableWeek,
        alerts: Switches { leaving: false, firstmate: true, ..Switches::default() },
        firstmate_file: Some(PathBuf::from("/srv/example/quota-pool.last")),
        source_dir: Some(PathBuf::from("/srv/example/quota-station")),
        codex_home: Some(PathBuf::from("/srv/example/codex")),
        codex: CodexOptions { plugins: false },
        auto_backup: AutoBackup { after_change: false, daily: true },
        skill_checks: SkillChecks { daily: false, notify: true },
        backup_dir: Some(PathBuf::from("/srv/example/backups")),
        pool_alert: PoolAlert { dry_at_most: 5, room_at_least: 20 },
        advanced: true,
        firstmate: false,
    };
    s.save(&file).unwrap();
    assert_eq!(Settings::load(&file), s);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("quotaAxiPath"));
    assert!(text.contains("machineName"));
    assert!(text.contains(r#""alias": "mac-c""#));
    assert!(text.contains("\"hideNames\": true"));
    assert!(text.contains("\"theme\": \"dark\""));
    assert!(text.contains("\"menubar\": \"codex\""));
    assert!(text.contains("\"menubarWindow\": \"fableWeek\""));
    assert!(text.contains("\"leaving\": false"));
    assert!(text.contains("firstmateFile"));
    assert!(text.contains("sourceDir"));
    assert!(text.contains("codexHome"));
    assert!(text.contains("\"afterChange\": false"));
    assert!(text.contains(r#""skillChecks": {"#));
    assert!(text.contains("backupDir"));
    assert!(text.contains(r#""dryAtMost": 5"#));
    assert!(text.contains("\"advanced\": true"));
    assert!(text.contains("\"firstmate\": false"));

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
    assert_eq!(s.quota_axi_path, Some(PathBuf::from("/opt/tools/bin/quota-axi")));
    assert_eq!(s.machine_name, None);
    assert!(!s.hide_names, "names show");
    assert_eq!(s.theme, Theme::Auto);
    assert_eq!(s.menubar, Menubar::Claude, "the menu bar follows Claude");
    assert_eq!(s.menubar_window, MenubarWindow::FiveHours, "the title shows five hours");
    // Alerts 1 to 4 on, Firstmate's off until a Mac has Firstmate.
    assert!([Kind::Pool, Kind::Leaving, Kind::Login, Kind::Overwrite]
        .iter()
        .all(|k| s.alerts.on(*k)));
    assert!(!s.alerts.on(Kind::Firstmate));
    assert_eq!(s.firstmate_file, None);
    assert_eq!(s.source_dir, None);
    assert_eq!(s.codex_home, None, "Codex's own ~/.codex");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn names_show_by_default_even_where_emails_were_turned_on_or_left_off() {
    let dir = std::env::temp_dir().join(format!("qs-settings-names-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for text in [r#"{ "showEmail": true }"#, "{}", r#"{ "theme": "sepia" }"#] {
        std::fs::write(&file, text).unwrap();
        let s = Settings::load(&file);
        assert!(!s.hide_names, "{text}");
        assert_eq!(s.theme, Theme::Auto, "{text}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_menu_bar_follows_claude_unless_told_and_an_unknown_value_reads_as_claude() {
    let dir = std::env::temp_dir().join(format!("qs-settings-menubar-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for (text, want) in [
        ("{}", Menubar::Claude),
        (r#"{ "menubar": "gemini" }"#, Menubar::Gemini),
        (r#"{ "menubar": "codex" }"#, Menubar::Codex),
        (r#"{ "menubar": "cursor" }"#, Menubar::Claude),
        (r#"{ "menubar": 3 }"#, Menubar::Claude),
    ] {
        std::fs::write(&file, text).unwrap();
        assert_eq!(Settings::load(&file).menubar, want, "{text}");
    }
    // The default is not written: a file from before keeps its shape.
    Settings::default().save(&file).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("menubar"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_title_shows_five_hours_unless_told_and_an_unknown_value_reads_as_five_hours() {
    let dir = std::env::temp_dir().join(format!("qs-settings-window-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for (text, want) in [
        // A file from before the choice, following a service: five hours.
        (r#"{ "menubar": "codex" }"#, MenubarWindow::FiveHours),
        ("{}", MenubarWindow::FiveHours),
        (r#"{ "menubarWindow": "week" }"#, MenubarWindow::Week),
        (r#"{ "menubarWindow": "fableWeek" }"#, MenubarWindow::FableWeek),
        (r#"{ "menubarWindow": "tightest" }"#, MenubarWindow::Tightest),
        (r#"{ "menubarWindow": "month" }"#, MenubarWindow::FiveHours),
        (r#"{ "menubarWindow": 2 }"#, MenubarWindow::FiveHours),
    ] {
        std::fs::write(&file, text).unwrap();
        assert_eq!(Settings::load(&file).menubar_window, want, "{text}");
    }
    // The default is not written: a file from before keeps its shape.
    let s = Settings { menubar: Menubar::Gemini, ..Settings::default() };
    s.save(&file).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("menubarWindow"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn machine_names_are_trimmed_and_capped() {
    assert_eq!(machine_name("  Mac mini 2  "), Some("Mac mini 2".into()));
    assert_eq!(machine_name("   "), None);
    assert_eq!(machine_name(""), None);
    let long = "Máy ".repeat(20);
    let cut = machine_name(&long).unwrap();
    assert!(cut.chars().count() <= MACHINE_NAME_MAX);
    assert!(!cut.ends_with(' '));
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
    let s = Settings::load(&file);
    assert_eq!(s.machine_name.as_deref(), Some("Mac A"));
    assert_eq!(s.peers, vec![peer("mac-b", "mac-b")]);

    // Saved again, only the list is written.
    s.save(&file).unwrap();
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(!text.contains("sshAlias"), "{text}");
    assert_eq!(Settings::load(&file), s);

    // A list already there wins, and an alias ssh would read as an option is dropped.
    std::fs::write(
        &file,
        r#"{ "sshAlias": "mac-b", "peers": [{ "name": "Mac C", "alias": "mac-c" }] }"#,
    )
    .unwrap();
    assert_eq!(Settings::load(&file).peers, vec![peer("Mac C", "mac-c")]);
    std::fs::write(&file, r#"{ "sshAlias": "-oProxyCommand=x" }"#).unwrap();
    assert!(Settings::load(&file).peers.is_empty());
    std::fs::write(&file, r#"{ "sshAlias": "" }"#).unwrap();
    assert!(Settings::load(&file).peers.is_empty());

    // A broken entry is dropped, never the whole file; the list is tidied as typed ones are.
    std::fs::write(
        &file,
        r#"{ "machineName": "Mac A", "peers": [{ "name": "Mac B" }, 7, { "alias": " mac-c " },
            { "name": "C2", "alias": "MAC-C" }, { "name": "X", "alias": "-oX=y" }] }"#,
    )
    .unwrap();
    let s = Settings::load(&file);
    assert_eq!(s.machine_name.as_deref(), Some("Mac A"));
    assert_eq!(s.peers, vec![peer("mac-c", "mac-c")]);
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

#[test]
fn plugins_under_the_custom_api_are_on_unless_unticked_and_the_choice_persists() {
    assert!(Settings::default().codex.plugins, "ticked by default, as before there was a choice");
    assert!(serde_json::from_str::<Settings>("{}").unwrap().codex.plugins);
    assert!(
        serde_json::from_str::<Settings>(r#"{ "codex": "odd" }"#).unwrap().codex.plugins,
        "an unknown value reads as ticked"
    );
    let dir = std::env::temp_dir().join(format!("qs-settings-codex-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("settings.json");
    let off = Settings { codex: CodexOptions { plugins: false }, ..Settings::default() };
    off.save(&file).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains("\"plugins\": false"));
    assert!(!Settings::load(&file).codex.plugins, "unticked stays unticked");
    Settings::default().save(&file).unwrap();
    assert!(
        !std::fs::read_to_string(&file).unwrap().contains("codex"),
        "the default is not written"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qs-settings-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_new_install_starts_without_nang_cao_and_stays_new() {
    let dir = fresh_dir("new");
    let file = dir.join("settings.json");
    let s = Settings::open(&file);
    assert!(!s.advanced, "Nâng cao is off on a new install");
    assert!(!s.firstmate, "and Firstmate with it");
    assert_eq!(s.pool_alert, PoolAlert { dry_at_most: 10, room_at_least: 15 });
    assert_eq!(s.backup_dir, None, "iCloud Drive until another folder is chosen");
    // The answer is written at once…
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"advanced\": false"), "{text}");
    assert!(text.contains("\"firstmate\": false"), "{text}");
    // …so the files the app writes next never make it look like an older install.
    std::fs::create_dir_all(dir.join("alerts")).unwrap();
    std::fs::write(dir.join("alerts/alerts.json"), "{}").unwrap();
    let again = Settings::open(&file);
    assert!(!again.advanced && !again.firstmate);
    // Alerts 2 and 5 stay quiet while Nâng cao is off, whatever their switches say.
    let mut on = again;
    on.alerts.leaving = true;
    on.alerts.firstmate = true;
    let run = on.alert_switches();
    assert!(!run.leaving && !run.firstmate);
    assert!(run.pool && run.login && run.overwrite);
    assert!(!on.firstmate_shown());
    // Turned on in Cài đặt, both run; Firstmate's has its own switch inside.
    on.advanced = true;
    assert!(on.alert_switches().leaving);
    assert!(!on.alert_switches().firstmate);
    on.firstmate = true;
    assert!(on.alert_switches().firstmate && on.firstmate_shown());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_mac_that_had_the_app_keeps_everything_as_it_was() {
    // A settings file from before Nâng cao, with the owner's Macs and Firstmate.
    let dir = fresh_dir("had");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &file,
        r#"{ "peers": [{ "name": "Mac B", "alias": "mac-b" }],
             "firstmateFile": "/srv/example/quota-pool.last",
             "alerts": { "leaving": true, "firstmate": true } }"#,
    )
    .unwrap();
    let s = Settings::open(&file);
    assert!(s.advanced && s.firstmate, "the tabs and alerts stay as they were");
    assert_eq!(s.alert_switches(), s.alerts);
    assert!(s.firstmate_shown());
    assert_eq!(s.peers, vec![peer("Mac B", "mac-b")]);
    assert_eq!(s.firstmate_file, Some(PathBuf::from("/srv/example/quota-pool.last")));
    assert_eq!(s.pool_alert, PoolAlert::default(), "alert 1 keeps 10 % and 15 %");
    assert_eq!(s.backup_dir, None, "the packs stay in iCloud Drive");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"advanced\": true") && text.contains("mac-b"), "{text}");
    // Turned off later in Cài đặt, it stays off.
    let mut off = s;
    off.advanced = false;
    off.save(&file).unwrap();
    assert!(!Settings::open(&file).advanced);
    let _ = std::fs::remove_dir_all(&dir);

    // No settings file, but the app's folder holds what it wrote: an older install too.
    let dir = fresh_dir("had-folder");
    std::fs::create_dir_all(dir.join("claude")).unwrap();
    std::fs::write(dir.join("claude/rotation"), "team = 2\n").unwrap();
    let s = Settings::open(&dir.join("settings.json"));
    assert!(s.advanced && s.firstmate);
    let _ = std::fs::remove_dir_all(&dir);

    // A settings file that cannot be read keeps everything on and is not written over.
    let dir = fresh_dir("had-broken");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), "{ half").unwrap();
    let s = Settings::open(&dir.join("settings.json"));
    assert!(s.advanced && s.firstmate);
    assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "{ half");
    let _ = std::fs::remove_dir_all(&dir);

    // A hidden file alone (Finder's) is not the app's.
    let dir = fresh_dir("hidden");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".DS_Store"), "").unwrap();
    assert!(!Settings::open(&dir.join("settings.json")).advanced);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn alert_one_s_thresholds_are_read_loosely() {
    let dir = fresh_dir("pool");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for (text, want) in [
        (r#"{ "advanced": false, "poolAlert": { "dryAtMost": 20, "roomAtLeast": 30 } }"#, (20, 30)),
        (r#"{ "advanced": false, "poolAlert": { "dryAtMost": 30, "roomAtLeast": 20 } }"#, (10, 15)),
        (r#"{ "advanced": false, "poolAlert": { "dryAtMost": 0 } }"#, (10, 15)),
        (r#"{ "advanced": false, "poolAlert": "high" }"#, (10, 15)),
    ] {
        std::fs::write(&file, text).unwrap();
        let p = Settings::open(&file).pool_alert;
        assert_eq!((p.dry_at_most, p.room_at_least), want, "{text}");
    }
    assert_eq!(PoolAlert { dry_at_most: 15, room_at_least: 15 }.check(), Err("order"));
    assert_eq!(PoolAlert { dry_at_most: 10, room_at_least: 100 }.check(), Err("range"));
    let _ = std::fs::remove_dir_all(&dir);
}
