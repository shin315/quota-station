use std::path::PathBuf;

use quota_station_core::alerts::{Kind, Switches};
use quota_station_core::settings::{
    machine_name, peers, Menubar, MenubarWindow, Peer, PeersError, Settings, Theme,
    MACHINE_NAME_MAX, PEERS_MAX,
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
