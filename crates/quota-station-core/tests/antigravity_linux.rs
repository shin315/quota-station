// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! The Antigravity accounts carried from one computer to a Linux one and
//! switched there, as the command line runs on Linux: `export` from a
//! Mac's store, `import` into another app folder whose tokens are files
//! (`Keychain::files`), and `switch N`, `switch EMAIL` and `rotate` writing
//! the file agy keeps its login in (`SharedLogin::file`), with no desktop app
//! (`Desktop::none`). Every token and client is made up; nothing here
//! reaches Google, a Keychain or a real agy.
#![cfg(all(unix, feature = "antigravity-accounts"))]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use common::desktop::remember;
use common::google::{CLIENT_ID, CLIENT_SECRET};
use common::os::FakeProcesses;
use common::temp_dir;
use quota_station_core::antigravity::bundle::{Bundle, FORMAT};
use quota_station_core::antigravity::cli::{self, Cli};
use quota_station_core::antigravity::credential::{build_payload, SharedLogin, AGY_LOGIN_FILE};
use quota_station_core::antigravity::desktop::{Desktop, Installed};
use quota_station_core::antigravity::index::{self, AccountIndex, AccountSummary};
use quota_station_core::antigravity::listing;
use quota_station_core::antigravity::oauth::Endpoints;
use quota_station_core::antigravity::rotate::{self, Fresh, Schedule, World};
use quota_station_core::antigravity::store::{self, Keychain};
use quota_station_core::antigravity::switch::{self, How, Limits, NoProbe, Outcome, Request};
use quota_station_core::antigravity::token::TokenData;
use quota_station_core::app_dir::AppDir;
use quota_station_core::quota::{agy, Response};
use serde_json::Value;
use time::OffsetDateTime;

const EMAILS: [&str; 3] = ["one@example.com", "two@example.com", "three@example.com"];
const RESET: &str = "2030-01-15T13:00:00Z";
/// When the made-up tokens were got: they stay fresh until 2030.
const ISSUED: i64 = 1_900_000_000;

/// One switch at a time in the whole process, so one at a time here too.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn token(n: usize) -> TokenData {
    TokenData::new(
        format!("fake-access-{n}"),
        format!("fake-refresh-{n}"),
        3600,
        Some(EMAILS[n - 1].to_string()),
        Some(format!("project-{n}")),
        None,
        ISSUED,
    )
    .with_oauth_metadata(Some("antigravity_enterprise".into()), None)
}

fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// One computer: the app's folder, its tokens as files, its home.
struct Computer {
    home: PathBuf,
    app: AppDir,
    keychain: Keychain,
}

impl Computer {
    fn empty(tag: &str) -> Computer {
        let home = temp_dir(tag);
        let app = AppDir::new(home.join(".config/com.shin315.quota-station"));
        let keychain = Keychain::files(&store::files_dir(&app));
        Computer { home, app, keychain }
    }

    /// The three accounts logged in, account 2 in use, the second held out
    /// of the rotation, and the OAuth client in `oauth-client.json`.
    fn mac(tag: &str) -> Computer {
        let mac = Computer::empty(tag);
        let mut index = AccountIndex::new();
        for (n, email) in (1..).zip(EMAILS) {
            index.accounts.push(AccountSummary {
                id: index::account_id(email),
                email: email.to_string(),
                name: Some(format!("Name {n}")),
                created_at: 100 + n as i64,
                last_used: 200,
                rotation_paused: n == 2,
            });
            mac.keychain.save(&index::account_id(email), &token(n)).unwrap();
        }
        index.current_account_id = Some(index::account_id(EMAILS[1]));
        index::save(&mac.app, &index).unwrap();
        let client = mac.app.path().join("oauth-client.json");
        std::fs::write(
            &client,
            format!(r#"{{"client_id":"{CLIENT_ID}","client_secret":"{CLIENT_SECRET}"}}"#),
        )
        .unwrap();
        std::fs::set_permissions(&client, std::fs::Permissions::from_mode(0o600)).unwrap();
        mac
    }

    fn agy_login(&self) -> PathBuf {
        self.home.join(AGY_LOGIN_FILE)
    }

    fn cli(&self, world: Box<dyn World>) -> Cli {
        Cli {
            app: self.app.clone(),
            home: self.home.clone(),
            exe: self.home.join(".local/bin/quota-station"),
            world,
            schedule: Schedule { launchctl: self.home.join("no-launchctl") },
            keychain: self.keychain.clone(),
        }
    }

    /// The commands, with the switch as the command line runs it on Linux.
    fn linux(&self) -> Cli {
        self.cli(Box::new(Linux {
            keychain: self.keychain.clone(),
            shared: SharedLogin::file(self.agy_login()),
        }))
    }

    fn call(&self, args: &[&str]) -> (i32, String, String) {
        call(&self.linux(), args)
    }
}

fn call(cli: &Cli, args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = cli::run(cli, &args, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

/// The command line's world on Linux, on this test's files: the readings
/// the app keeps (agy is not there to ask), the switch with agy's login as
/// a file and no desktop app. No wait after the switch.
struct Linux {
    keychain: Keychain,
    shared: SharedLogin,
}

impl World for Linux {
    fn read(&self, app: &AppDir, _fresh: Fresh, now: OffsetDateTime) -> Response {
        listing::read(&no_agy(), app, 120, now)
    }

    fn switch(&self, app: &AppDir, id: &str, from: Option<String>, how: Option<How>) -> Outcome {
        let (desktop, tools) = (Desktop::none(), no_agy());
        // Never asked: every token here is fresh until 2030.
        let endpoints = Endpoints::google();
        let deps = switch::Deps {
            app,
            endpoints: &endpoints,
            keychain: &self.keychain,
            probe: Arc::new(NoProbe),
            shared: &self.shared,
            desktop: &desktop,
            agy: &tools,
        };
        let request = Request { id: id.into(), how: how.unwrap_or(How::Plain), from };
        let limits = Limits { settle: Duration::ZERO, ..Limits::default() };
        switch::switch_antigravity(&deps, &request, limits, &now).outcome
    }
}

fn no_agy() -> agy::Tools {
    agy::Tools { agy: None, processes: Arc::new(FakeProcesses::none()) }
}

/// What agy reads from its login file.
fn agy_login(pc: &Computer) -> Value {
    serde_json::from_str(&std::fs::read_to_string(pc.agy_login()).unwrap()).unwrap()
}

/// Exports `mac` to a new folder and imports it into a new Linux computer.
fn carried(tag: &str) -> (Computer, Computer) {
    let mac = Computer::mac(&format!("{tag}-mac"));
    let pc = Computer::empty(&format!("{tag}-pc"));
    let dir = mac.home.join("bundle");
    let (code, _, err) = mac.call(&["export", dir.to_str().unwrap()]);
    assert_eq!((code, err.as_str()), (0, ""));
    let (code, _, err) = pc.call(&["import", dir.to_str().unwrap()]);
    assert_eq!((code, err.as_str()), (0, ""));
    (mac, pc)
}

#[test]
fn an_export_is_a_private_folder_of_the_accounts_their_tokens_and_the_client() {
    let mac = Computer::mac("agy-export");
    let dir = mac.home.join("bundle");
    let (code, out, err) = mac.call(&["export", dir.to_str().unwrap()]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "exported account 1  one@example.com\nexported account 2  two@example.com\n\
         exported account 3  three@example.com\n"
    );
    assert!(!out.contains("fake-") && !out.contains(CLIENT_SECRET), "{out}");
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&dir.join("tokens")), 0o700);
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest, serde_json::json!({"format": FORMAT, "version": 1}));
    for name in ["manifest.json", "accounts.json", "oauth-client.json"] {
        assert_eq!(mode(&dir.join(name)), 0o600, "{name}");
    }
    let index: AccountIndex =
        serde_json::from_str(&std::fs::read_to_string(dir.join("accounts.json")).unwrap()).unwrap();
    assert_eq!(index.current_account_id, None, "which account agy is on stays here");
    for (n, email) in (1..).zip(EMAILS) {
        let file = dir.join(format!("tokens/{}.json", index::account_id(email)));
        assert_eq!(mode(&file), 0o600);
        let kept: TokenData =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(kept, token(n));
    }
    // An export never goes over another folder's files.
    let (code, _, err) = mac.call(&["export", dir.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(err.contains("not empty"), "{err}");
}

#[test]
fn an_import_keeps_the_numbers_tokens_holds_and_client_in_private_files() {
    let (mac, pc) = carried("agy-import");
    let here = index::load(&pc.app).unwrap();
    let there = index::load(&mac.app).unwrap();
    assert_eq!(here.accounts.len(), 3);
    for (h, t) in here.accounts.iter().zip(&there.accounts) {
        assert_eq!((&h.id, &h.email, &h.name), (&t.id, &t.email, &t.name));
        assert_eq!(h.rotation_paused, t.rotation_paused);
    }
    assert_eq!(here.current_account_id, None);
    for (n, email) in (1..).zip(EMAILS) {
        let id = index::account_id(email);
        assert_eq!(pc.keychain.read(&id).unwrap(), token(n));
        let file = store::files_dir(&pc.app)
            .join("com.shin315.quota-station.antigravity")
            .join(format!("{id}.bin"));
        assert_eq!(mode(&file), 0o600);
    }
    assert_eq!(mode(&store::files_dir(&pc.app)), 0o700);
    assert_eq!(mode(&index::dir(&pc.app)), 0o700);
    let client = pc.app.path().join("oauth-client.json");
    assert_eq!(mode(&client), 0o600);
    assert_eq!(quota_station_core::antigravity::client::load(&pc.app).unwrap().id(), CLIENT_ID);
}

#[test]
fn a_tar_stream_carries_the_same_bundle() {
    let mac = Computer::mac("agy-tar-mac");
    let pc = Computer::empty("agy-tar-pc");
    // To a pipe: the tar on stdout, what it holds on stderr.
    let (code, tar, err) = {
        let args = ["export".to_string(), "-".to_string()];
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = cli::run(&mac.linux(), &args, &mut out, &mut err);
        (code, out, String::from_utf8(err).unwrap())
    };
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("exported account 3  three@example.com"), "{err}");
    assert_eq!(tar.len() % 512, 0);
    let bundle = Bundle::read_tar(&mut tar.as_slice()).unwrap();
    assert_eq!(bundle.accounts().len(), 3);
    // To a tar file, which only its owner reads, and from it.
    let file = mac.home.join("accounts.tar");
    let (code, _, err) = mac.call(&["export", file.to_str().unwrap()]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(mode(&file), 0o600);
    assert_eq!(std::fs::read(&file).unwrap(), tar);
    let (code, out, err) = pc.call(&["import", file.to_str().unwrap()]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "imported account 1  one@example.com\nimported account 2  two@example.com\n\
         imported account 3  three@example.com\n"
    );
    assert_eq!(pc.keychain.read(&index::account_id(EMAILS[2])).unwrap(), token(3));
}

/// Breaks a copy of a bundle in the folder it is given.
type Breaks<'a> = dyn Fn(&Path) + 'a;

#[test]
fn a_bundle_that_does_not_check_out_is_refused_and_nothing_is_written() {
    let mac = Computer::mac("agy-bad");
    let dir = mac.home.join("bundle");
    assert_eq!(mac.call(&["export", dir.to_str().unwrap()]).0, 0);
    let token2 = format!("tokens/{}.json", index::account_id(EMAILS[1]));
    let write = |d: &Path, name: &str, text: &str| std::fs::write(d.join(name), text).unwrap();
    let broken: [(&str, &Breaks<'_>); 5] = [
        ("names x", &|d| write(d, "manifest.json", r#"{"format":"x","version":1}"#)),
        ("no tokens/", &|d| std::fs::remove_file(d.join(&token2)).unwrap()),
        ("another account's", &|d| write(d, &token2, &serde_json::to_string(&token(1)).unwrap())),
        ("oauth-client.json is not usable", &|d| {
            write(d, "oauth-client.json", r#"{"client_id":"<id>","client_secret":"<secret>"}"#)
        }),
        ("unexpected file", &|d| write(d, "notes.txt", "hi")),
    ];
    for (i, (why, breaks)) in broken.iter().enumerate() {
        let copy = mac.home.join(format!("broken-{i}"));
        std::fs::create_dir_all(copy.join("tokens")).unwrap();
        for entry in ["manifest.json", "accounts.json", "oauth-client.json"] {
            std::fs::copy(dir.join(entry), copy.join(entry)).unwrap();
        }
        for email in EMAILS {
            let name = format!("tokens/{}.json", index::account_id(email));
            std::fs::copy(dir.join(&name), copy.join(&name)).unwrap();
        }
        breaks(&copy);
        let pc = Computer::empty(&format!("agy-bad-pc-{i}"));
        let (code, out, err) = pc.call(&["import", copy.to_str().unwrap()]);
        assert_eq!((code, out.as_str()), (1, ""), "{why}");
        assert!(err.contains(why), "{why}: {err}");
        assert!(!index::index_path(&pc.app).exists(), "{why}");
        assert!(!store::files_dir(&pc.app).exists(), "{why}");
        assert!(!pc.app.path().join("oauth-client.json").exists(), "{why}");
    }
}

#[test]
fn a_tar_with_a_link_in_it_is_refused() {
    let mut tar = vec![0u8; 512];
    tar[..14].copy_from_slice(b"manifest.json\0");
    tar[100..108].copy_from_slice(b"0000777\0");
    tar[124..136].copy_from_slice(b"00000000000\0");
    tar[156] = b'2';
    tar.extend_from_slice(&[0; 1024]);
    let err = Bundle::read_tar(&mut tar.as_slice()).unwrap_err().to_string();
    assert!(err.contains("manifest.json is not a plain file"), "{err}");
}

#[test]
fn an_import_beside_accounts_already_here_keeps_their_numbers() {
    let mac = Computer::mac("agy-merge-mac");
    let pc = Computer::empty("agy-merge-pc");
    let mut here = AccountIndex::new();
    here.accounts.push(AccountSummary {
        id: index::account_id("own@example.com"),
        email: "own@example.com".into(),
        name: None,
        created_at: 1,
        last_used: 1,
        rotation_paused: false,
    });
    here.current_account_id = Some(index::account_id("own@example.com"));
    index::save(&pc.app, &here).unwrap();
    let dir = mac.home.join("bundle");
    assert_eq!(mac.call(&["export", dir.to_str().unwrap()]).0, 0);
    let (code, out, _) = pc.call(&["import", dir.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.starts_with("imported account 2  one@example.com\n"), "{out}");
    let index = index::load(&pc.app).unwrap();
    assert_eq!(index.accounts[0].email, "own@example.com");
    assert_eq!(index.current_account_id, here.current_account_id);
    // Imported again, nothing is added twice.
    assert_eq!(pc.call(&["import", dir.to_str().unwrap()]).0, 0);
    assert_eq!(index::load(&pc.app).unwrap().accounts.len(), 4);
}

#[test]
fn agys_login_file_is_replaced_whole_and_only_its_owner_reads_it() {
    let pc = Computer::empty("agy-file");
    let shared = SharedLogin::file(pc.agy_login());
    shared.write(&token(1)).unwrap();
    assert_eq!(std::fs::read_to_string(pc.agy_login()).unwrap(), build_payload(&token(1)));
    assert_eq!(mode(&pc.agy_login()), 0o600);
    assert_eq!(mode(pc.agy_login().parent().unwrap()), 0o700);
    std::fs::set_permissions(pc.agy_login(), std::fs::Permissions::from_mode(0o644)).unwrap();
    shared.write(&token(2)).unwrap();
    let login = agy_login(&pc);
    assert_eq!(login["token"]["refresh_token"], "fake-refresh-2");
    assert_eq!(login["token"]["token_type"], "Bearer");
    assert_eq!(login["auth_method"], "consumer");
    assert_eq!(mode(&pc.agy_login()), 0o600);
    let left: Vec<_> = std::fs::read_dir(pc.agy_login().parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["antigravity-oauth-token"], "no temporary file is left");
}

#[test]
fn without_a_desktop_app_nothing_is_quit_or_opened() {
    let desktop = Desktop::none();
    assert_eq!(desktop.installed(), Installed::Missing);
    assert_eq!(desktop.running(), Ok(false));
    assert_eq!(desktop.quit(Duration::ZERO), Ok(()));
    assert_eq!(desktop.open(), Ok(()));
}

#[test]
fn switch_by_number_or_email_writes_agys_login() {
    let _serial = serial();
    let (_mac, pc) = carried("agy-switch");
    let (code, out, err) = pc.call(&["switch", "3"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("account 3"), "{out}");
    assert!(err.contains("restart them"), "{err}");
    assert_eq!(agy_login(&pc)["token"]["refresh_token"], "fake-refresh-3");
    let index = index::load(&pc.app).unwrap();
    assert_eq!(index.current_account_id, Some(index::account_id(EMAILS[2])));

    let (code, out, err) = pc.call(&["switch", "ONE@example.com"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(agy_login(&pc)["token"]["refresh_token"], "fake-refresh-1");
    assert_eq!(pc.call(&["switch", "1"]).1, "already on account 1\n");

    let (code, _, err) = pc.call(&["switch", "nobody@example.com"]);
    assert_eq!(code, 1);
    assert!(err.contains("no account nobody@example.com"), "{err}");
    assert_eq!(agy_login(&pc)["token"]["refresh_token"], "fake-refresh-1");
    let log = switch::read_log(&pc.app);
    assert_eq!(log.len(), 2);
    assert!(log.iter().all(|l| l.result == "switched"), "{log:?}");
}

#[test]
fn rotate_moves_agy_to_the_account_with_the_most_left() {
    let _serial = serial();
    let (_mac, pc) = carried("agy-rotate");
    assert_eq!(pc.call(&["switch", "1"]).0, 0);
    let mut settings = rotate::load_settings(&pc.app);
    settings.cooldown_minutes = 0;
    rotate::save_settings(&pc.app, &settings).unwrap();
    let ms = now() * 1000;
    remember(&pc.app, EMAILS[0], 97.0, RESET, ms);
    // Account 2 has the most left, but is held out of the rotation.
    remember(&pc.app, EMAILS[1], 0.0, RESET, ms);
    remember(&pc.app, EMAILS[2], 40.0, RESET, ms);
    let (code, out, err) = pc.call(&["rotate", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(agy_login(&pc)["token"]["refresh_token"], "fake-refresh-1");
    let (code, out, err) = pc.call(&["rotate"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("account 3"), "{out}");
    assert_eq!(agy_login(&pc)["token"]["refresh_token"], "fake-refresh-3");
    let (code, out, _) = pc.call(&["status"]);
    assert_eq!(code, 0);
    assert!(out.lines().any(|l| l.starts_with("*  3")), "{out}");
}

#[test]
fn the_rotation_cannot_be_turned_on_without_a_launch_agent() {
    let line = rotate::cron_line(Path::new("/home/it's me/.local/bin/quota-station"));
    assert_eq!(
        line,
        r"*/3 * * * * '/home/it'\''s me/.local/bin/quota-station' antigravity rotate >/dev/null 2>&1"
    );
}
