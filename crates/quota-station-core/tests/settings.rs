use std::path::PathBuf;

use quota_station_core::settings::Settings;

#[test]
fn settings_round_trip_and_default_when_missing() {
    let dir = std::env::temp_dir().join(format!("qs-settings-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let file = dir.join("nested/settings.json");
    assert_eq!(Settings::load(&file), Settings::default());

    let s = Settings { quota_axi_path: Some(PathBuf::from("/opt/tools/bin/quota-axi")) };
    s.save(&file).unwrap();
    assert_eq!(Settings::load(&file), s);
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("quotaAxiPath"));

    std::fs::write(&file, "{ not json").unwrap();
    assert_eq!(Settings::load(&file), Settings::default());
}
