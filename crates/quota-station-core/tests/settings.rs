// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use quota_station_core::alerts::{Kind, Switches};
use quota_station_core::ext::Hooks;
use quota_station_core::settings::{
    machine_name, AutoBackup, CodexOptions, Language, Menubar, MenubarWindow, PoolAlert, Settings,
    SkillChecks, Theme, ThemePack, MACHINE_NAME_MAX,
};

/// The hooks the app passes without an extension: none.
fn hooks() -> &'static dyn Hooks {
    quota_station_core::ext::none()
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
        alerts: Switches { login: false, ..Switches::default() },
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
    assert!(text.contains("\"login\": false"));

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
    assert!([Kind::Pool, Kind::Login, Kind::Overwrite].iter().all(|k| s.alerts.on(*k)));
    assert_eq!(s.codex_home, None, "Codex's own ~/.codex");
    assert!(s.ext.is_empty(), "and nothing is kept for an extension");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn names_show_by_default_even_where_emails_were_turned_on_or_left_off() {
    let dir = std::env::temp_dir().join(format!("qs-settings-names-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for text in [r#"{ "showEmail": true }"#, "{}", r#"{ "theme": "sepia", "language": "fr" }"#] {
        std::fs::write(&file, text).unwrap();
        let s = Settings::load(&file);
        assert!(!s.hide_names, "{text}");
        assert_eq!(s.theme, Theme::Auto, "{text}");
        assert_eq!(s.language, Language::Auto, "{text}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_theme_pack_is_the_default_unless_chosen_and_an_unknown_one_reads_as_the_default() {
    let dir = std::env::temp_dir().join(format!("qs-settings-pack-{}", std::process::id()));
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for (text, pack) in [
        ("{}", ThemePack::Default),
        (r#"{ "themePack": "kinhNhip" }"#, ThemePack::KinhNhip),
        (r#"{ "themePack": "banDieuKhien" }"#, ThemePack::BanDieuKhien),
        (r#"{ "themePack": "aurora" }"#, ThemePack::Default),
        (r#"{ "themePack": 3 }"#, ThemePack::Default),
    ] {
        std::fs::write(&file, text).unwrap();
        assert_eq!(Settings::load(&file).theme_pack, pack, "{text}");
    }
    // The default is not written, so a file stays as an older version wrote it.
    Settings::default().save(&file).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("themePack"));
    let s = Settings { theme_pack: ThemePack::DaiSangTinh, ..Settings::default() };
    s.save(&file).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains(r#""themePack": "daiSangTinh""#));
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
    assert_eq!(machine_name("  Mac Studio 2  "), Some("Mac Studio 2".into()));
    assert_eq!(machine_name("   "), None);
    assert_eq!(machine_name(""), None);
    let long = "Máy ".repeat(20);
    let cut = machine_name(&long).unwrap();
    assert!(cut.chars().count() <= MACHINE_NAME_MAX);
    assert!(!cut.ends_with(' '));
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
fn a_new_install_starts_with_the_first_run_and_stays_new() {
    let dir = fresh_dir("new");
    let file = dir.join("settings.json");
    let s = Settings::open(&file, hooks());
    assert!(!s.setup_seen, "the main window opens on the first run's steps");
    assert_eq!(s.pool_alert, PoolAlert { dry_at_most: 10, room_at_least: 15 });
    assert_eq!(s.backup_dir, None, "iCloud Drive until another folder is chosen");
    // The answer is written at once…
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"setupSeen\": false"), "{text}");
    // …so the files the app writes next never make it look like an older install.
    std::fs::create_dir_all(dir.join("alerts")).unwrap();
    std::fs::write(dir.join("alerts/alerts.json"), "{}").unwrap();
    let again = Settings::open(&file, hooks());
    assert!(!again.setup_seen, "until its owner finishes the steps");
    let mut seen = again;
    seen.setup_seen = true;
    seen.save(&file).unwrap();
    assert!(Settings::open(&file, hooks()).setup_seen);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_mac_that_had_the_app_skips_the_first_run() {
    // A settings file from before the first run's steps.
    let dir = fresh_dir("had");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, r#"{ "machineName": "Mac B", "alerts": { "login": false } }"#).unwrap();
    let s = Settings::open(&file, hooks());
    assert!(s.setup_seen, "no first run");
    assert!(!s.alerts.login, "and the alerts stay as they were");
    assert_eq!(s.pool_alert, PoolAlert::default(), "alert 1 keeps 10 % and 15 %");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"setupSeen\": true") && text.contains("Mac B"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);

    // No settings file, but the app's folder holds what it wrote: an older install too.
    let dir = fresh_dir("had-folder");
    std::fs::create_dir_all(dir.join("alerts")).unwrap();
    std::fs::write(dir.join("alerts/alerts.json"), "{}").unwrap();
    assert!(Settings::open(&dir.join("settings.json"), hooks()).setup_seen);
    let _ = std::fs::remove_dir_all(&dir);

    // A settings file that cannot be read is not written over.
    let dir = fresh_dir("had-broken");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("settings.json"), "{ half").unwrap();
    assert!(Settings::open(&dir.join("settings.json"), hooks()).setup_seen);
    assert_eq!(std::fs::read_to_string(dir.join("settings.json")).unwrap(), "{ half");
    let _ = std::fs::remove_dir_all(&dir);

    // A hidden file alone (Finder's) is not the app's.
    let dir = fresh_dir("hidden");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".DS_Store"), "").unwrap();
    assert!(!Settings::open(&dir.join("settings.json"), hooks()).setup_seen);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn alert_one_s_thresholds_are_read_loosely() {
    let dir = fresh_dir("pool");
    let file = dir.join("settings.json");
    std::fs::create_dir_all(&dir).unwrap();
    for (text, want) in [
        (r#"{ "setupSeen": true, "poolAlert": { "dryAtMost": 20, "roomAtLeast": 30 } }"#, (20, 30)),
        (r#"{ "setupSeen": true, "poolAlert": { "dryAtMost": 30, "roomAtLeast": 20 } }"#, (10, 15)),
        (r#"{ "setupSeen": true, "poolAlert": { "dryAtMost": 0 } }"#, (10, 15)),
        (r#"{ "setupSeen": true, "poolAlert": "high" }"#, (10, 15)),
    ] {
        std::fs::write(&file, text).unwrap();
        let p = Settings::open(&file, hooks()).pool_alert;
        assert_eq!((p.dry_at_most, p.room_at_least), want, "{text}");
    }
    assert_eq!(PoolAlert { dry_at_most: 15, room_at_least: 15 }.check(), Err("order"));
    assert_eq!(PoolAlert { dry_at_most: 10, room_at_least: 100 }.check(), Err("range"));
    let _ = std::fs::remove_dir_all(&dir);
}
