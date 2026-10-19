//! The Codex switch (M8) on a temporary Codex home built from the fixtures
//! in `fixtures/codex/`: every name, URL and key in them is made up, and no
//! test reads or writes the real `~/.codex`, Codex's login or CC Switch.
//!
//! What these tests prove about keeping the ChatGPT login (the plugins and
//! remote control of Codex App): a switch to the custom provider leaves
//! `auth.json` byte for byte, writes `requires_openai_auth = true` in the
//! provider's table when the provider has its own key and a ChatGPT login
//! is there (so Codex keeps treating the login as signed in), keeps the
//! `[mcp_servers]` list and every other line as they were, and refuses a
//! provider with no key of its own. Whether Codex App's plugins and remote
//! control then really work is Codex's behaviour, not this file's: it needs
//! a check on a Mac.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{fixture_text, temp_dir};
use quota_station_core::antigravity::fs::AppDir;
use quota_station_core::codex::config::{self, Current, KeyKind, Refusal, Target};
use quota_station_core::codex::files::{self, Record, WriteError};
use quota_station_core::codex::login::{self, Login};
use quota_station_core::codex::{self, log, probe, procs, resume, FileState, Outcome};
use time::OffsetDateTime;

const T0: i64 = 1_768_548_000;

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(T0 + offset).unwrap()
}

/// A Codex home with `config` (a fixture name) and `auth`, and an app folder.
fn setup(config: &str, auth: Option<&str>) -> (PathBuf, AppDir) {
    let root = temp_dir("codex");
    let home = root.join("codex-home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join(codex::CONFIG), fixture_text(&format!("codex/{config}"))).unwrap();
    if let Some(auth) = auth {
        std::fs::write(home.join(codex::AUTH), fixture_text(&format!("codex/{auth}"))).unwrap();
    }
    (home, AppDir::new(root.join("app")))
}

fn config_text(home: &Path) -> String {
    std::fs::read_to_string(home.join(codex::CONFIG)).unwrap()
}

fn hash(home: &Path) -> String {
    files::sha256(&std::fs::read(home.join(codex::CONFIG)).unwrap())
}

/// The lines that are not the switch's own keys.
fn other_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("model_provider ") || t.starts_with("requires_openai_auth "))
        })
        .collect()
}

fn requires(home: &Path, id: &str) -> Option<bool> {
    let doc: toml_edit::DocumentMut = config_text(home).parse().unwrap();
    doc.get("model_providers")?.get(id)?.get("requires_openai_auth")?.as_bool()
}

#[test]
fn the_file_is_read_without_keeping_a_key() {
    let parsed = config::parse(&fixture_text("codex/config-chatgpt.toml"), None).unwrap();
    assert_eq!(parsed.current(), Current::ChatGpt);
    let custom = parsed.custom.clone().unwrap();
    assert_eq!(custom.id, "custom");
    assert_eq!(custom.name, "relay");
    assert_eq!(custom.base_url.as_deref(), Some("https://relay.example.com/v1"));
    assert_eq!(custom.key, KeyKind::Bearer);
    assert_eq!(custom.requires_openai_auth, Some(true));
    let mcp: Vec<_> = parsed.mcp.iter().map(|m| (m.name.as_str(), m.kind, m.enabled)).collect();
    assert_eq!(
        mcp,
        [("files", "stdio", true), ("docs-search", "http", true), ("tracker", "http", false)]
    );
    assert!(parsed.traces.is_empty());
    let shown = format!("{parsed:?}");
    assert!(
        !shown.contains("test-key-not-real") && !shown.contains("test-token"),
        "no key in what is kept"
    );

    let custom = config::parse(&fixture_text("codex/config-custom.toml"), None).unwrap();
    assert_eq!(custom.current(), Current::Custom);

    let env =
        config::parse(&fixture_text("codex/config-envkey.toml"), None).unwrap().custom.unwrap();
    assert_eq!(
        (env.id.as_str(), env.key, env.env_key.as_deref()),
        ("relay", KeyKind::EnvKey, Some("RELAY_API_KEY"))
    );
    let header =
        config::parse(&fixture_text("codex/config-header.toml"), None).unwrap().custom.unwrap();
    assert_eq!(header.key, KeyKind::Headers);
    let none =
        config::parse(&fixture_text("codex/config-nokey.toml"), None).unwrap().custom.unwrap();
    assert_eq!(none.key, KeyKind::None);

    let dormant = config::parse(&fixture_text("codex/config-dormant.toml"), None).unwrap();
    assert_eq!(dormant.custom.as_ref().unwrap().key, KeyKind::Placeholder);
    assert_eq!(dormant.traces.len(), 2, "the model list and the placeholder: {:?}", dormant.traces);
}

#[test]
fn a_file_with_several_tables_goes_back_to_the_one_last_used() {
    let text = "[model_providers.alpha]\nenv_key = \"A\"\n\n[model_providers.custom]\nenv_key = \"C\"\n\n[model_providers.ollama]\nbase_url = \"http://localhost\"\n";
    assert_eq!(
        config::parse(text, None).unwrap().custom.unwrap().id,
        "custom",
        "CC Switch's id first"
    );
    assert_eq!(
        config::parse(text, Some("alpha")).unwrap().custom.unwrap().id,
        "alpha",
        "then the remembered one"
    );
    let only = "[model_providers.ollama]\nbase_url = \"http://localhost\"\n[model_providers.mine]\nenv_key = \"M\"\n";
    assert_eq!(
        config::parse(only, Some("gone")).unwrap().custom.unwrap().id,
        "mine",
        "never a built-in id"
    );
    let other = "model_provider = \"ollama\"\n[model_providers.mine]\nenv_key = \"M\"\n";
    assert_eq!(config::parse(other, None).unwrap().current(), Current::Other("ollama".into()));
}

#[test]
fn requires_openai_auth_follows_the_login_only_for_a_provider_with_its_own_key() {
    // CC Switch's own table of cases.
    for (kind, login, want) in [
        (KeyKind::Bearer, true, true),
        (KeyKind::Bearer, false, false),
        (KeyKind::EnvKey, true, true),
        (KeyKind::EnvKey, false, false),
        (KeyKind::Headers, true, false),
        (KeyKind::Headers, false, false),
        (KeyKind::None, true, false),
        (KeyKind::Placeholder, true, false),
    ] {
        assert_eq!(config::requires_openai_auth(kind, login), want, "{kind:?} with login {login}");
    }
}

#[test]
fn switching_to_the_custom_provider_keeps_the_chatgpt_login_and_everything_else() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    let before = config_text(&home);
    let auth_before = std::fs::read(home.join(codex::AUTH)).unwrap();
    let auth_mtime = std::fs::metadata(home.join(codex::AUTH)).unwrap().modified().unwrap();

    let reading = codex::read(&home, &Record::default());
    assert_eq!(reading.login, Login::ChatGpt);
    let out = codex::switch(&home, &app, Target::Custom, reading.hash.as_deref(), at(0));
    let Outcome::Switched { from, to, backup, .. } = out else { panic!("{out:?}") };
    assert_eq!((from.as_str(), to.as_str()), ("openai", "custom"));

    let after = config_text(&home);
    assert!(after.starts_with("model_provider = \"custom\"\n"), "{after}");
    assert_eq!(requires(&home, "custom"), Some(true), "Codex keeps the ChatGPT login signed in");
    assert_eq!(other_lines(&before), other_lines(&after), "every other line as it was");
    let mcp = |t: &str| {
        let doc: toml_edit::DocumentMut = t.parse().unwrap();
        doc.get("mcp_servers").unwrap().to_string()
    };
    assert_eq!(mcp(&before), mcp(&after), "the MCP list is not written");
    assert_eq!(
        std::fs::read(home.join(codex::AUTH)).unwrap(),
        auth_before,
        "auth.json byte for byte"
    );
    assert_eq!(std::fs::metadata(home.join(codex::AUTH)).unwrap().modified().unwrap(), auth_mtime);

    let backup = app.path().join(files::BACKUP_DIR).join(backup);
    let kept = std::fs::read_to_string(&backup).unwrap();
    assert_eq!(
        kept,
        before
            .replace("\"test-key-not-real\"", "\"<redacted>\"")
            .replace("\"test-token-not-real\"", "\"<redacted>\""),
        "the file before is kept, its keys left out"
    );
    assert!(!kept.contains("not-real"), "no key in the backup: {kept}");
    assert_eq!(std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(
        std::fs::metadata(home.join(codex::CONFIG)).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let record = files::load(&app);
    assert_eq!(record.written_hash.as_deref(), Some(hash(&home).as_str()));
    assert_eq!(record.written_provider.as_deref(), Some("custom"));
    assert_eq!(record.last_custom.as_deref(), Some("custom"));
    assert!(!record.changed_outside(&hash(&home)));
    let lines = log::read(&app);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        (lines[0].from.as_str(), lines[0].to.as_str(), lines[0].result.as_str()),
        ("openai", "custom", "switched")
    );
}

#[test]
fn switching_back_removes_only_the_provider_line_and_keeps_the_table() {
    let (home, app) = setup("config-custom.toml", Some("auth-chatgpt.json"));
    let before = config_text(&home);
    let reading = codex::read(&home, &Record::default());
    let out = codex::switch(&home, &app, Target::ChatGpt, reading.hash.as_deref(), at(0));
    assert!(matches!(out, Outcome::Switched { .. }), "{out:?}");
    let after = config_text(&home);
    assert!(!after.contains("model_provider ="));
    assert!(
        after.contains("# Chosen by hand: the relay.\nmodel = \"gpt-5.5\""),
        "the comment above the line stays: {after}"
    );
    assert!(
        after.contains("[model_providers.custom]")
            && after.contains("experimental_bearer_token = \"test-key-not-real\"")
    );
    assert_eq!(
        requires(&home, "custom"),
        Some(true),
        "the table is kept as it was, for resuming old sessions"
    );
    assert_eq!(other_lines(&before), other_lines(&after));
    assert_eq!(
        codex::read(&home, &files::load(&app)).parsed().unwrap().current(),
        Current::ChatGpt
    );
}

#[test]
fn a_switch_there_and_back_gives_the_same_bytes() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    let before = std::fs::read(home.join(codex::CONFIG)).unwrap();
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::ChatGpt, Some(&h), at(60)),
        Outcome::Switched { .. }
    ));
    assert_eq!(std::fs::read(home.join(codex::CONFIG)).unwrap(), before);
    assert_eq!(files::backups(&app.path().join(files::BACKUP_DIR)).len(), 2);
}

#[test]
fn requires_openai_auth_is_written_by_the_rule() {
    // An environment key with no login: off, or Codex would stay on its login page.
    let (home, app) = setup("config-envkey.toml", None);
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    assert_eq!(requires(&home, "relay"), Some(false));
    assert!(config_text(&home).contains(
        "env_key = \"RELAY_API_KEY\"\nwire_api = \"responses\"\nrequires_openai_auth = false\n"
    ));

    // The same with a ChatGPT login: on.
    let (home, app) = setup("config-envkey.toml", Some("auth-chatgpt.json"));
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    assert_eq!(requires(&home, "relay"), Some(true));

    // Only an API key in auth.json is no login.
    let (home, app) = setup("config-envkey.toml", Some("auth-apikey.json"));
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    assert_eq!(requires(&home, "relay"), Some(false));

    // An Authorization header: always off, so the ChatGPT login is never sent there.
    let (home, app) = setup("config-header.toml", Some("auth-chatgpt.json"));
    let before = config_text(&home);
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    assert_eq!(requires(&home, "custom"), Some(false));
    assert_eq!(other_lines(&before), other_lines(&config_text(&home)));
}

#[test]
fn a_provider_without_its_own_key_is_refused_and_nothing_is_written() {
    for (fixture, refusal) in [
        ("config-nokey.toml", Refusal::NoKey),
        ("config-dormant.toml", Refusal::Placeholder),
        (
            "config-profile.toml",
            Refusal::Profile { profile: "fast".into(), key: "model_provider".into() },
        ),
    ] {
        let (home, app) = setup(fixture, Some("auth-chatgpt.json"));
        let before = std::fs::read(home.join(codex::CONFIG)).unwrap();
        let h = hash(&home);
        let out = codex::switch(&home, &app, Target::Custom, Some(&h), at(0));
        assert_eq!(out, Outcome::Refused { target: Target::Custom, refusal }, "{fixture}");
        assert_eq!(std::fs::read(home.join(codex::CONFIG)).unwrap(), before);
        assert!(files::backups(&app.path().join(files::BACKUP_DIR)).is_empty());
    }
    let inline = "model = \"x\"\nmodel_providers = { custom = { env_key = \"K\" } }\n";
    let parsed = config::parse(inline, None).unwrap();
    assert_eq!(config::plan(&parsed, Target::Custom, true), Err(Refusal::Shape));
}

#[test]
fn a_file_changed_after_the_sheet_opened_is_not_overwritten() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    let seen = hash(&home);
    let mut text = config_text(&home);
    text.push_str("\n# edited meanwhile\n");
    std::fs::write(home.join(codex::CONFIG), &text).unwrap();
    let out = codex::switch(&home, &app, Target::Custom, Some(&seen), at(0));
    assert_eq!(out, Outcome::Failed { target: Target::Custom, error: WriteError::Changed });
    assert_eq!(config_text(&home), text);
}

#[test]
fn a_write_stops_when_the_file_changes_between_the_plan_and_the_rename() {
    let root = temp_dir("codex-race");
    let file = root.join("config.toml");
    std::fs::write(&file, "a = 1\n").unwrap();
    let base = b"a = 1\n";
    let h = files::sha256(base);
    // Another program writes after the app read the file.
    std::fs::write(&file, "a = 2\n").unwrap();
    let err = files::write(
        &file,
        base,
        &h,
        "a = 1\nmodel_provider = \"x\"\n",
        &root.join("backups"),
        at(0),
    )
    .unwrap_err();
    assert_eq!(err, WriteError::Changed);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "a = 2\n",
        "the other program's write stays"
    );
    let left: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(left.len(), 2, "no temporary file left: {left:?}");
    let backups = files::backups(&root.join("backups"));
    assert_eq!(std::fs::read(&backups[0]).unwrap(), base, "the planned-on file is in the backups");
}

#[test]
fn a_symlinked_config_stays_a_symlink() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    let real = home.parent().unwrap().join("dotfiles-config.toml");
    std::fs::rename(home.join(codex::CONFIG), &real).unwrap();
    std::os::unix::fs::symlink(&real, home.join(codex::CONFIG)).unwrap();
    let h = hash(&home);
    assert!(matches!(
        codex::switch(&home, &app, Target::Custom, Some(&h), at(0)),
        Outcome::Switched { .. }
    ));
    assert!(std::fs::symlink_metadata(home.join(codex::CONFIG)).unwrap().file_type().is_symlink());
    assert!(std::fs::read_to_string(&real).unwrap().starts_with("model_provider = \"custom\""));
}

#[test]
fn the_newest_ten_backups_stay() {
    let dir = temp_dir("codex-backups");
    for i in 0..13 {
        files::backup(format!("v{i}").as_bytes(), &dir, at(i * 60)).unwrap();
    }
    let kept = files::backups(&dir);
    assert_eq!(kept.len(), files::BACKUPS_KEPT);
    assert_eq!(std::fs::read_to_string(&kept[0]).unwrap(), "v12");
    assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
}

#[test]
fn backups_kept_whole_before_m9_lose_their_keys() {
    let dir = temp_dir("codex-old-backups");
    let whole = fixture_text("codex/config-chatgpt.toml");
    let old = files::backup(whole.as_bytes(), &dir, at(0)).unwrap();
    let stamp = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    std::fs::File::options().write(true).open(&old).unwrap().set_modified(stamp).unwrap();
    let garbage = files::backup(b"not = [toml", &dir, at(60)).unwrap();

    assert_eq!(files::scrub_backups(&dir), 2);
    let kept = std::fs::read_to_string(&old).unwrap();
    assert!(!kept.contains("not-real"), "{kept}");
    assert!(kept.contains("experimental_bearer_token = \"<redacted>\""), "{kept}");
    assert_eq!(kept.lines().count(), whole.lines().count(), "every other line stays");
    assert_eq!(std::fs::metadata(&old).unwrap().modified().unwrap(), stamp, "its time is kept");
    assert_eq!(std::fs::metadata(&old).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(!garbage.exists(), "one that does not read as TOML might hold a key: removed");
    assert_eq!(files::scrub_backups(&dir), 0, "a second run changes nothing");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no temporary file left");
}

#[test]
fn a_change_outside_the_app_is_seen_and_can_be_kept() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    assert!(
        !files::load(&app).changed_outside(&hash(&home)),
        "nothing to compare before the first write"
    );
    let h = hash(&home);
    codex::switch(&home, &app, Target::Custom, Some(&h), at(0));
    // CC Switch, or a hand, switches back.
    let text = config_text(&home).replacen("model_provider = \"custom\"\n", "", 1);
    std::fs::write(home.join(codex::CONFIG), text).unwrap();
    let reading = codex::read(&home, &files::load(&app));
    let now = reading.hash.clone().unwrap();
    assert!(files::load(&app).changed_outside(&now));
    codex::keep(&app, &now, reading.parsed()).unwrap();
    assert!(!files::load(&app).changed_outside(&now), "kept by the owner");
    std::fs::write(home.join(codex::CONFIG), "model = \"other\"\n").unwrap();
    assert!(files::load(&app).changed_outside(&hash(&home)), "a later change is seen again");
}

#[test]
fn the_login_is_judged_by_its_shape_and_the_store() {
    let (home, _) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    assert_eq!(login::check(&home, config::AuthStore::File), Login::ChatGpt);
    assert_eq!(
        login::check(&home, config::AuthStore::Keyring),
        Login::Keychain,
        "the Keychain is not opened"
    );
    assert_eq!(login::check(&home, config::AuthStore::Ephemeral), Login::Ephemeral);
    let (home, _) = setup("config-chatgpt.toml", Some("auth-apikey.json"));
    assert_eq!(login::check(&home, config::AuthStore::File), Login::ApiKey);
    let (home, _) = setup("config-chatgpt.toml", None);
    assert_eq!(login::check(&home, config::AuthStore::File), Login::Missing);
    std::fs::write(home.join(codex::AUTH), "{not json").unwrap();
    assert_eq!(login::check(&home, config::AuthStore::File), Login::Unreadable);
    assert!(Login::Keychain.on_disk() && !Login::ApiKey.on_disk() && !Login::Missing.on_disk());
}

#[test]
fn a_missing_or_broken_config_is_read_as_such() {
    let root = temp_dir("codex-missing");
    let reading = codex::read(&root, &Record::default());
    assert!(reading.present && reading.file == FileState::Missing);
    std::fs::write(root.join(codex::CONFIG), "model = [\n").unwrap();
    assert!(matches!(codex::read(&root, &Record::default()).file, FileState::Invalid(_)));
    let app = AppDir::new(root.join("app"));
    let out = codex::switch(&root, &app, Target::ChatGpt, None, at(0));
    assert!(matches!(out, Outcome::Invalid { .. }), "{out:?}");
    assert_eq!(std::fs::read_to_string(root.join(codex::CONFIG)).unwrap(), "model = [\n");
}

#[test]
fn the_process_table_shows_sessions_the_app_the_daemon_and_cc_switch() {
    let p = procs::classify(&fixture_text("codex/ps.txt"));
    assert_eq!(
        p.sessions, 3,
        "the native codex once, resume and exec; not mcp, not the node wrapper"
    );
    assert!(p.app && p.daemon && p.cc_switch);
    assert_eq!(procs::classify(""), procs::Procs::default());
    assert_eq!(
        procs::restart_daemon(None, Path::new("/nonexistent"), false),
        procs::Restart::NotRunning,
        "nothing runs when no daemon does"
    );
}

#[test]
fn the_daemon_restart_runs_codex_own_command_for_this_codex_home() {
    let root = temp_dir("codex-restart");
    let args = root.join("args");
    let codex = common::script(
        &root.join("bin/codex"),
        &format!("echo \"$@ $CODEX_HOME\" > '{}'\n", args.display()),
    );
    assert_eq!(
        procs::restart_daemon(Some(&codex), &root.join("home"), true),
        procs::Restart::Restarted
    );
    let got = std::fs::read_to_string(&args).unwrap();
    assert_eq!(got.trim(), format!("app-server daemon restart {}", root.join("home").display()));
    let failing = common::script(&root.join("bin/codex-fail"), "echo nope >&2\nexit 3\n");
    assert!(matches!(
        procs::restart_daemon(Some(&failing), &root, true),
        procs::Restart::Failed { .. }
    ));
}

#[test]
fn the_provider_counts_as_answering_on_any_http_answer() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = [0u8; 2048];
        let n = s.read(&mut buf).unwrap();
        s.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    });
    let answer = probe::check_with(
        &format!("http://127.0.0.1:{port}/v1/"),
        Duration::from_secs(2),
        Duration::ZERO,
    );
    assert!(matches!(answer, probe::Answer::Up { .. }), "{answer:?}");
    let request = seen.join().unwrap();
    assert!(request.starts_with("GET /v1/models "), "{request}");
    assert!(!request.to_ascii_lowercase().contains("authorization"), "no key is sent");

    // A port nobody listens on.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let answer = probe::check_with(
        &format!("http://127.0.0.1:{closed}/v1"),
        Duration::from_millis(500),
        Duration::ZERO,
    );
    assert!(matches!(answer, probe::Answer::Down { .. }));
    assert_eq!(probe::check("file:///etc"), probe::Answer::BadUrl);
}

#[test]
fn resumable_sessions_are_counted_by_their_provider() {
    let home = temp_dir("codex-sessions");
    let day = home.join("sessions/2026/01/16");
    std::fs::create_dir_all(&day).unwrap();
    let meta = |p: &str| {
        format!("{{\"timestamp\":\"2026-01-16T07:00:00Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"x\",\"cwd\":\"/w\",\"instructions\":\"{}\",\"model_provider\":\"{p}\"}}}}\n{{\"type\":\"turn\"}}\n", "a".repeat(4000))
    };
    for (i, p) in ["openai", "openai", "custom", "openai"].iter().enumerate() {
        std::fs::write(day.join(format!("rollout-2026-01-16T07-00-0{i}-x.jsonl")), meta(p))
            .unwrap();
    }
    std::fs::write(day.join("rollout-old.jsonl"), "{\"type\":\"session_meta\"}\n").unwrap();
    std::fs::write(day.join("notes.txt"), "model_provider").unwrap();
    let counts = resume::count(&home);
    assert_eq!(counts.get("openai"), Some(&3));
    assert_eq!(counts.get("custom"), Some(&1));
    assert_eq!(counts.get(""), Some(&1), "one without a provider");
    assert_eq!(counts.values().sum::<u32>(), 5);
}

#[test]
fn the_log_marks_a_restart_on_the_last_switch() {
    let (home, app) = setup("config-chatgpt.toml", Some("auth-chatgpt.json"));
    let h = hash(&home);
    codex::switch(&home, &app, Target::Custom, Some(&h), at(0));
    log::mark_restarted(&app, true).unwrap();
    assert_eq!(log::read(&app)[0].restarted, Some(true));
}
