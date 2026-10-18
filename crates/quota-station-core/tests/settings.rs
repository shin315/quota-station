use std::path::PathBuf;

use quota_station_core::settings::{machine_name, Settings, MACHINE_NAME_MAX};

#[test]
fn settings_round_trip_and_default_when_missing() {
    let dir = std::env::temp_dir().join(format!("qs-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("nested/settings.json");
    assert_eq!(Settings::load(&file), Settings::default());

    let s = Settings {
        quota_axi_path: Some(PathBuf::from("/opt/tools/bin/quota-axi")),
        machine_name: Some("Mac phòng khách".into()),
    };
    s.save(&file).unwrap();
    assert_eq!(Settings::load(&file), s);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("quotaAxiPath"));
    assert!(text.contains("machineName"));

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
