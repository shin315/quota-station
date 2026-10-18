use std::path::PathBuf;

use quota_station_core::settings::{
    machine_name, peers, Peer, PeersError, Settings, MACHINE_NAME_MAX, PEERS_MAX,
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
    };
    s.save(&file).unwrap();
    assert_eq!(Settings::load(&file), s);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("quotaAxiPath"));
    assert!(text.contains("machineName"));
    assert!(text.contains(r#""alias": "mac-c""#));

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
