// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Finding Antigravity's OAuth client in an installed Antigravity, against
//! fake installs laid out as the macOS and Windows builds lay themselves out,
//! whose fake programs hold a fake client among other strings. The pins are
//! the fake client's; no test reads the Antigravity of the computer it runs
//! on, and nothing here is a real client.
#![cfg(feature = "antigravity-accounts")]

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::google::{token_body, FakeBrowser, FakeGoogle, Home, Script, CLIENT_ID};
use common::temp_dir;
use quota_station_core::antigravity::client::{self, ClientStatus};
use quota_station_core::antigravity::discover::{
    mac_places, windows_places, Miss, Pins, Sources, MIN_SIZE,
};
use quota_station_core::antigravity::login;

// Put together at run time, so that no literal here looks like a client to a
// secret scanner.
const DOMAIN: [&str; 2] = [".apps.", "googleusercontent.com"];
const SECRET_MARK: [&str; 2] = ["GOC", "SPX-"];

fn fake_id() -> String {
    ["424242424242-fakeantigravityclient", "fortests00", DOMAIN[0], DOMAIN[1]].concat()
}

fn fake_secret() -> String {
    [SECRET_MARK[0], SECRET_MARK[1], "fake_Secret-ForTheTests0001"].concat()
}

/// A secret of another build's.
fn other_secret(rest: &str) -> String {
    [SECRET_MARK[0], SECRET_MARK[1], rest].concat()
}

fn pins() -> Pins {
    Pins::of(&fake_id(), &fake_secret())
}

/// A fake program: `MIN_SIZE` bytes and more, with `strings` packed in the
/// middle as a program packs them, with no separator, beside other clients.
fn program(path: &Path, strings: &[&str]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = vec![0x90u8; MIN_SIZE as usize];
    let mut middle = b"go1.24 runtime: 128".to_vec();
    middle.extend_from_slice(
        ["111111111111-anotherclientanotherclient", "anoth", DOMAIN[0], DOMAIN[1]]
            .concat()
            .as_bytes(),
    );
    middle.extend_from_slice(other_secret("notTheSecretTheAppPinned0").as_bytes());
    for s in strings {
        middle.extend_from_slice(s.as_bytes());
        middle.extend_from_slice(b"7abc_");
    }
    let at = bytes.len() / 2;
    bytes.splice(at..at, middle);
    std::fs::write(path, bytes).unwrap();
}

fn holding_the_client(path: &Path) {
    program(path, &[&fake_id(), &fake_secret()]);
}

fn found(sources: &Sources) -> Result<String, Miss> {
    sources.find().map(|c| c.id().to_string())
}

#[test]
fn the_macos_desktop_app_is_found_in_its_bundle() {
    let root = temp_dir("agy-find-mac");
    let home = root.join("home");
    let app = home.join("Applications/Antigravity.app");
    holding_the_client(&app.join("Contents/Resources/bin/language_server"));
    // Only the places under this test's home: /Applications is the real one.
    let (folders, programs) = mac_places(&home, &[]);
    let folders = folders.into_iter().filter(|f| f.starts_with(&root)).collect();
    let programs = programs.into_iter().filter(|p| p.starts_with(&root)).collect();
    let sources = Sources::new(folders, programs, pins());
    assert_eq!(found(&sources), Ok(fake_id()));
}

#[test]
fn the_macos_places() {
    let home = Path::new("/Users/x");
    let path = [PathBuf::from("/Users/x/bin")];
    let (folders, programs) = mac_places(home, &path);
    assert_eq!(
        folders,
        [
            "/Applications/Antigravity.app",
            "/Applications/Antigravity IDE.app",
            "/Users/x/Applications/Antigravity.app",
            "/Users/x/Applications/Antigravity IDE.app",
        ]
        .map(PathBuf::from)
    );
    assert_eq!(
        programs,
        [
            "/Users/x/bin/agy",
            "/Users/x/.local/bin/agy",
            "/opt/homebrew/bin/agy",
            "/usr/local/bin/agy"
        ]
        .map(PathBuf::from)
    );
}

#[test]
fn the_windows_desktop_app_and_cli_are_found_where_their_installers_put_them() {
    let root = temp_dir("agy-find-win");
    let local = root.join("AppData/Local");
    let home = root.join("Users/x");
    let (folders, programs) = windows_places(Some(&local), None, None, &home, &[]);
    // The desktop app's installer: `<Programs>\Antigravity\resources\bin`.
    let server = local.join("Programs/Antigravity/resources/bin/language_server.exe");
    holding_the_client(&server);
    let sources = Sources::new(folders.clone(), programs.clone(), pins());
    assert_eq!(found(&sources), Ok(fake_id()));

    // Without the desktop app, the CLI's installer: `%LOCALAPPDATA%\agy\bin`.
    std::fs::remove_file(&server).unwrap();
    holding_the_client(&local.join("agy/bin/agy.exe"));
    let sources = Sources::new(folders, programs, pins());
    assert_eq!(found(&sources), Ok(fake_id()));
}

#[test]
fn the_windows_places() {
    let local = PathBuf::from("C:/Users/x/AppData/Local");
    let pf = PathBuf::from("C:/Program Files");
    let home = PathBuf::from("C:/Users/x");
    let path = [PathBuf::from("C:/tools")];
    let (folders, programs) = windows_places(Some(&local), Some(&pf), None, &home, &path);
    assert_eq!(
        folders,
        [
            local.join("Programs").join("Antigravity"),
            local.join("Programs").join("Antigravity IDE"),
            pf.join("Antigravity"),
            pf.join("Antigravity IDE"),
        ]
    );
    assert_eq!(
        programs,
        [
            path[0].join("agy.exe"),
            local.join("agy").join("bin").join("agy.exe"),
            home.join(".local").join("bin").join("agy.exe"),
        ]
    );
    // No variable set: only the home's place.
    let (folders, programs) = windows_places(None, None, None, &home, &[]);
    assert!(folders.is_empty());
    assert_eq!(programs, [home.join(".local").join("bin").join("agy.exe")]);
}

#[test]
fn an_older_layout_and_a_cli_on_the_path_are_found_too() {
    let root = temp_dir("agy-find-other");
    // The language server under the IDE's extension, as older builds kept it.
    let ide = root.join("Antigravity IDE.app");
    holding_the_client(
        &ide.join("Contents/Resources/app/extensions/antigravity/bin/language_server_macos_arm"),
    );
    let sources = Sources::new(vec![ide], vec![], pins());
    assert_eq!(found(&sources), Ok(fake_id()));

    let bin = root.join("bin");
    holding_the_client(&bin.join("agy"));
    let (_, programs) = mac_places(&root.join("nohome"), &[bin]);
    let programs = programs.into_iter().filter(|p| p.starts_with(&root)).collect();
    let sources = Sources::new(vec![], programs, pins());
    assert_eq!(found(&sources), Ok(fake_id()));
}

#[test]
fn no_antigravity_and_an_unknown_build_say_so() {
    let root = temp_dir("agy-find-miss");
    let app = root.join("Antigravity.app");
    let sources = Sources::new(vec![app.clone()], vec![root.join("agy")], pins());
    assert_eq!(found(&sources), Err(Miss::NotInstalled));

    // Too small to be Antigravity's program, even with the client in it.
    std::fs::write(root.join("agy"), format!("{}{}", fake_id(), fake_secret())).unwrap();
    assert_eq!(found(&sources), Err(Miss::NotInstalled));

    // A build whose client is not the pinned one.
    program(
        &app.join("Contents/Resources/bin/language_server"),
        &[&other_secret("someOtherBuildsSecret0001")],
    );
    assert_eq!(found(&sources), Err(Miss::Unrecognized));

    // The id alone is not a client.
    program(&app.join("Contents/Resources/bin/language_server"), &[&fake_id()]);
    assert_eq!(found(&sources), Err(Miss::Unrecognized), "a changed file is read again");

    // Updated to a build the app knows: read again.
    std::thread::sleep(Duration::from_millis(20));
    holding_the_client(&app.join("Contents/Resources/bin/language_server"));
    assert_eq!(found(&sources), Ok(fake_id()));
}

#[cfg(unix)]
#[test]
fn a_symlinked_folder_in_the_install_is_not_followed() {
    let root = temp_dir("agy-find-link");
    let elsewhere = root.join("elsewhere");
    holding_the_client(&elsewhere.join("language_server"));
    let app = root.join("Antigravity.app/Contents/Resources");
    std::fs::create_dir_all(&app).unwrap();
    std::os::unix::fs::symlink(&elsewhere, app.join("bin")).unwrap();
    let sources = Sources::new(vec![root.join("Antigravity.app")], vec![], pins());
    assert_eq!(found(&sources), Err(Miss::NotInstalled));
}

#[test]
fn the_file_comes_first_and_neither_ever_prints_the_client() {
    let home = Home::new("agy-find-file");
    let install = home.home.join("Antigravity.app");
    holding_the_client(&install.join("Contents/Resources/bin/language_server"));
    let sources = Sources::new(vec![install], vec![], pins());

    // oauth-client.json is there: it is the client.
    assert_eq!(client::load_with(&home.app, Some(&sources)).unwrap().id(), CLIENT_ID);
    // One that is not usable is said so, not passed over.
    home.write_client("not json", 0o600);
    assert_eq!(client::load_with(&home.app, Some(&sources)).err(), Some(ClientStatus::Invalid));
    // None: the installed Antigravity's.
    std::fs::remove_file(home.app.path().join("oauth-client.json")).unwrap();
    let loaded = client::load_with(&home.app, Some(&sources)).unwrap();
    assert_eq!(loaded.id(), fake_id());
    // Nowhere to look: missing, as before.
    assert_eq!(client::load_with(&home.app, None).err(), Some(ClientStatus::Missing));

    for shown in [format!("{loaded:?}"), format!("{sources:?}"), format!("{:?}", pins())] {
        assert!(!shown.contains(&fake_id()) && !shown.contains(&fake_secret()), "{shown}");
    }
    let empty = Sources::new(vec![home.home.join("none")], vec![], pins());
    assert_eq!(client::load_with(&home.app, Some(&empty)).err(), Some(ClientStatus::Missing));
    let other =
        Sources::new(vec![home.home.join("Antigravity.app")], vec![], Pins::of(&fake_id(), "x"));
    assert_eq!(client::load_with(&home.app, Some(&other)).err(), Some(ClientStatus::Unrecognized));
}

/// The owner's way: no setup, "Thêm tài khoản" three times, each a new
/// account, with the client of the Antigravity installed on this computer.
#[test]
fn three_accounts_are_added_with_the_installed_antigravitys_client() {
    let home = Home::new("agy-find-login");
    std::fs::remove_file(home.app.path().join("oauth-client.json")).unwrap();
    let local = home.home.join("AppData/Local");
    holding_the_client(&local.join("Programs/Antigravity/resources/bin/language_server.exe"));
    let (folders, programs) = windows_places(Some(&local), None, None, &home.home, &[]);
    client::use_installed(Sources::new(folders, programs, pins()));
    assert_eq!(client::status(&home.app), ClientStatus::Ok);

    let google = FakeGoogle::start(Script::default());
    let endpoints = google.endpoints();
    let deps = home.deps(&endpoints);
    for n in 1..=3usize {
        google.set(|s| {
            s.userinfo.1 = format!(r#"{{"id":"100000{n}","email":"ultra.{n}@example.com"}}"#);
            s.exchange.1 =
                token_body(&format!("fake-access-{n}"), Some(&format!("fake-refresh-{n}")));
        });
        let browser = FakeBrowser::following();
        let done = login::prepare(&deps)
            .and_then(|flow| {
                login::complete(&deps, flow, &browser, Duration::from_secs(20), &|| 1_768_548_000)
            })
            .unwrap();
        assert_eq!((done.number, done.added), (n, true));
        let opened = browser.opened();
        let url = url::Url::parse(&opened[0]).unwrap();
        let client_id = url.query_pairs().find(|(k, _)| k == "client_id").unwrap().1.into_owned();
        assert_eq!(client_id, fake_id());
        assert!(!opened[0].contains(&fake_secret()), "the secret never goes to the browser");
    }
    let exchanges = google.to("/token");
    assert_eq!(exchanges.len(), 3);
    for call in exchanges {
        let form = call.form();
        assert!(form.contains(&("client_id".into(), fake_id())));
        assert!(form.contains(&("client_secret".into(), fake_secret())));
    }
}
