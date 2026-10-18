//! `scripts/install.sh` (M7) on a clean temporary folder: a source tree
//! holding only the script and the files it checks, a fake `cargo` whose
//! `tauri build` writes a small real bundle (a copy of `/usr/bin/true`), and
//! an Applications folder in the same temporary folder. `codesign`, `ditto`,
//! `xattr` and `plutil` are the real ones, on those files only. `open`,
//! `security`, `npm` and `npx` are fakes that record their arguments, and so
//! are `launchctl`, `osascript` and `sfltool`, which must never run: the
//! script adds no login item. Nothing here opens an app or reads the
//! Keychain.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{script, temp_dir};

const APP: &str = "Quota Station.app";

struct Sandbox {
    root: PathBuf,
    /// The source tree: `scripts/install.sh` and what it checks for.
    src: PathBuf,
    bin: PathBuf,
    apps: PathBuf,
    target: PathBuf,
    log: PathBuf,
}

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>BUNDLE_ID</string>
<key>CFBundleShortVersionString</key><string>VERSION</string>
<key>CFBundleExecutable</key><string>quota-station</string>
<key>CFBundlePackageType</key><string>APPL</string>
</dict></plist>
"#;

/// Writes a small bundle that `codesign` can sign: an Info.plist and a copy
/// of `/usr/bin/true` as its executable.
fn fake_bundle(app: &Path, bundle_id: &str, version: &str) {
    std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    std::fs::copy("/usr/bin/true", app.join("Contents/MacOS/quota-station")).unwrap();
    let plist = INFO_PLIST.replace("BUNDLE_ID", bundle_id).replace("VERSION", version);
    std::fs::write(app.join("Contents/Info.plist"), plist).unwrap();
}

impl Sandbox {
    fn new(tag: &str) -> Sandbox {
        let root = temp_dir(tag);
        let src = root.join("src");
        std::fs::create_dir_all(src.join("scripts")).unwrap();
        std::fs::create_dir_all(src.join("src-tauri")).unwrap();
        std::fs::create_dir_all(src.join("node_modules")).unwrap();
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/install.sh");
        std::fs::copy(repo, src.join("scripts/install.sh")).unwrap();
        std::fs::write(src.join("src-tauri/tauri.conf.json"), r#"{ "version": "0.1.0" }"#).unwrap();
        std::fs::write(src.join("package-lock.json"), "{}").unwrap();
        std::fs::write(src.join("node_modules/.package-lock.json"), "{}").unwrap();
        // Older than node_modules', so no `npm ci` unless a test asks.
        touch_back(&src.join("package-lock.json"));
        let s = Sandbox {
            bin: root.join("bin"),
            apps: root.join("Applications"),
            target: root.join("target"),
            log: root.join("calls.log"),
            src,
            root,
        };
        std::fs::create_dir_all(s.root.join("home")).unwrap();
        std::fs::create_dir_all(s.root.join("tmp")).unwrap();
        std::fs::write(&s.log, "").unwrap();
        let record = r#"echo "$(basename "$0") $*" >> "$FAKE_LOG""#;
        for name in ["open", "npm", "npx", "launchctl", "osascript", "sfltool"] {
            script(&s.bin.join(name), &format!("{record}\n"));
        }
        s.security("   0 valid identities found");
        s.cargo("");
        s
    }

    /// The fake `security`: `find-identity` prints `listing`.
    fn security(&self, listing: &str) {
        let r = r#"echo "security $*" >> "$FAKE_LOG""#;
        script(&self.bin.join("security"), &format!("{r}\ncat <<'EOF'\n{listing}\nEOF\n"));
    }

    /// The fake `cargo`: `tauri build` writes the bundle where Tauri does,
    /// then runs `extra`.
    fn cargo(&self, extra: &str) {
        let bundle = r#"$CARGO_TARGET_DIR/release/bundle/macos/Quota Station.app"#;
        let plist = INFO_PLIST.replace("BUNDLE_ID", "com.shin315.quota-station");
        let plist = plist.replace("VERSION", "0.1.0");
        let body = format!(
            r#"echo "cargo $* QS_COMMIT=$QS_COMMIT" >> "$FAKE_LOG"
if [ "$1 $2" = "tauri --version" ]; then echo "tauri-cli 2.0.0"; exit 0; fi
if [ "$1 $2" = "tauri build" ]; then
  app="{bundle}"
  mkdir -p "$app/Contents/MacOS"
  cp /usr/bin/true "$app/Contents/MacOS/quota-station"
  cat > "$app/Contents/Info.plist" <<'EOF'
{plist}EOF
  {extra}
fi
"#
        );
        script(&self.bin.join("cargo"), &body);
    }

    fn run(&self, args: &[&str]) -> Output {
        let path = format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", self.bin.display());
        Command::new("/bin/bash")
            .arg(self.src.join("scripts/install.sh"))
            .args(args)
            .current_dir(&self.root)
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("TMPDIR", self.root.join("tmp"))
            .env("PATH", path)
            .env("FAKE_LOG", &self.log)
            .env("CARGO_TARGET_DIR", &self.target)
            .output()
            .unwrap()
    }

    fn dest(&self) -> String {
        self.apps.display().to_string()
    }

    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log).unwrap().lines().map(String::from).collect()
    }

    fn installed(&self) -> PathBuf {
        self.apps.join(APP)
    }
}

fn touch_back(path: &Path) {
    let status = Command::new("/usr/bin/touch").args(["-t", "202601010000"]).arg(path).status();
    assert!(status.unwrap().success());
}

fn ok(out: &Output) {
    assert!(
        out.status.success(),
        "install.sh failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn verifies(app: &Path) -> bool {
    Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(app)
        .status()
        .unwrap()
        .success()
}

/// `codesign -dv`'s description of the signature.
fn signature(app: &Path) -> String {
    let out = Command::new("/usr/bin/codesign").arg("-dv").arg(app).output().unwrap();
    String::from_utf8_lossy(&out.stderr).to_string()
}

fn quarantined(path: &Path) -> bool {
    let out = Command::new("/usr/bin/xattr").arg("-r").arg(path).output().unwrap();
    String::from_utf8_lossy(&out.stdout).contains("com.apple.quarantine")
}

fn quarantine(path: &Path) {
    let status = Command::new("/usr/bin/xattr")
        .args(["-w", "com.apple.quarantine", "0081;00000000;Safari;"])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(quarantined(path));
}

/// Nothing that registers a login item or a launchd job ran.
fn no_login_item(s: &Sandbox) {
    for call in s.calls() {
        for tool in ["launchctl", "osascript", "sfltool"] {
            assert!(!call.starts_with(tool), "install.sh ran {call}");
        }
    }
}

#[test]
fn builds_signs_ad_hoc_installs_and_opens() {
    let s = Sandbox::new("install-build");
    let out = s.run(&["--dest", &s.dest()]);
    ok(&out);

    let calls = s.calls();
    assert!(calls.iter().any(|c| c.starts_with("cargo tauri build --bundles app")), "{calls:?}");
    assert!(calls.iter().any(|c| c == "security find-identity -v -p codesigning"));
    assert!(!calls.iter().any(|c| c.starts_with("npm")), "node_modules is up to date: {calls:?}");

    let app = s.installed();
    assert!(app.join("Contents/MacOS/quota-station").is_file());
    assert!(verifies(&app));
    assert!(signature(&app).contains("Signature=adhoc"));
    assert!(!quarantined(&app));
    // Opened once, at the installed path, after it was in place.
    let opens: Vec<&String> = calls.iter().filter(|c| c.starts_with("open ")).collect();
    assert_eq!(opens.len(), 1, "{calls:?}");
    assert!(opens[0].ends_with(&format!("/Applications/{APP}")), "{opens:?}");
    // Only the app went into the Applications folder.
    let left: Vec<String> = std::fs::read_dir(&s.apps)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(left, vec![APP.to_string()]);
    no_login_item(&s);
}

#[test]
fn signs_with_the_apple_development_identity_when_there_is_one() {
    let s = Sandbox::new("install-identity");
    s.security(
        "  1) 0123456789ABCDEF0123456789ABCDEF01234567 \"Apple Development: Example Person (EXAMPLE123)\"\n     1 valid identities found",
    );
    // A codesign that records what it was asked and signs ad-hoc for real,
    // since the identity exists only in the fake listing.
    script(
        &s.bin.join("codesign"),
        r#"echo "codesign $*" >> "$FAKE_LOG"
args=""
next=0
for a in "$@"; do
  if [ "$next" = 1 ]; then a="-"; next=0; fi
  [ "$a" = "--sign" ] && next=1
  args="$args|$a"
done
IFS='|'
set -- ${args#|}
exec /usr/bin/codesign "$@"
"#,
    );
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    let calls = s.calls();
    let signs: Vec<&String> = calls.iter().filter(|c| c.contains("--sign")).collect();
    assert_eq!(signs.len(), 1, "{calls:?}");
    assert!(signs[0].contains("--sign 0123456789ABCDEF0123456789ABCDEF01234567"), "{signs:?}");
    assert!(verifies(&s.installed()));
}

#[test]
fn an_identity_given_wins_and_dash_is_ad_hoc() {
    let s = Sandbox::new("install-dash");
    s.security("  1) 0123456789ABCDEF0123456789ABCDEF01234567 \"Apple Development: Example Person (EXAMPLE123)\"");
    ok(&s.run(&["--dest", &s.dest(), "--no-open", "--identity", "-"]));
    assert!(!s.calls().iter().any(|c| c.starts_with("security")), "{:?}", s.calls());
    assert!(signature(&s.installed()).contains("Signature=adhoc"));
}

#[test]
fn no_open_installs_without_opening() {
    let s = Sandbox::new("install-noopen");
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    assert!(verifies(&s.installed()));
    assert!(!s.calls().iter().any(|c| c.starts_with("open")), "{:?}", s.calls());
}

#[test]
fn replaces_the_app_already_there() {
    let s = Sandbox::new("install-replace");
    let old = s.installed();
    fake_bundle(&old, "com.shin315.quota-station", "0.0.9");
    std::fs::write(old.join("Contents/old-marker"), "old").unwrap();
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    let app = s.installed();
    assert!(!app.join("Contents/old-marker").exists());
    assert!(verifies(&app));
    let names: Vec<String> = std::fs::read_dir(&s.apps)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec![APP.to_string()], "no staging copy is left behind");
}

#[test]
fn a_failed_build_leaves_the_installed_app_alone() {
    let s = Sandbox::new("install-fail");
    let old = s.installed();
    fake_bundle(&old, "com.shin315.quota-station", "0.0.9");
    std::fs::write(old.join("Contents/old-marker"), "old").unwrap();
    s.cargo("echo 'error[E0425]: cannot find value' >&2; exit 101");
    let out = s.run(&["--dest", &s.dest()]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("install.sh: error: cargo tauri build failed"),
        "{}",
        stderr(&out)
    );
    assert!(old.join("Contents/old-marker").is_file());
    assert!(!s.calls().iter().any(|c| c.starts_with("open")));
}

#[test]
fn runs_npm_ci_when_the_lock_file_changed() {
    let s = Sandbox::new("install-npm");
    touch_back(&s.src.join("node_modules/.package-lock.json"));
    std::fs::write(s.src.join("package-lock.json"), "{ }").unwrap();
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    let calls = s.calls();
    let npm = calls.iter().position(|c| c == "npm ci --no-audit --no-fund").expect("npm ci ran");
    let build = calls.iter().position(|c| c.starts_with("cargo tauri build")).unwrap();
    assert!(npm < build);
}

#[test]
fn without_cargo_it_says_how_to_install_a_copy_built_elsewhere() {
    let s = Sandbox::new("install-nocargo");
    std::fs::remove_file(s.bin.join("cargo")).unwrap();
    let out = s.run(&["--dest", &s.dest()]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("cargo is not installed"), "{err}");
    assert!(err.contains("--package") && err.contains("--app"), "{err}");
    assert!(!s.installed().exists());
}

#[test]
fn the_build_carries_the_commit_it_was_built_from() {
    let s = Sandbox::new("install-commit");
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.invalid"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(&s.src)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
    let head = git(&["rev-parse", "HEAD"]);
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    assert!(s.calls().iter().any(|c| c.ends_with(&format!("QS_COMMIT={head}"))), "{:?}", s.calls());

    std::fs::write(s.src.join("package-lock.json"), "{ \"changed\": true }").unwrap();
    std::fs::write(s.log.clone(), "").unwrap();
    ok(&s.run(&["--dest", &s.dest(), "--no-open"]));
    assert!(
        s.calls().iter().any(|c| c.ends_with(&format!("QS_COMMIT={head}-dirty"))),
        "{:?}",
        s.calls()
    );
}

#[test]
fn packages_a_zip_that_installs_on_a_mac_that_cannot_build() {
    let s = Sandbox::new("install-package");
    let out_dir = s.root.join("out");
    ok(&s.run(&["--package", out_dir.to_str().unwrap()]));
    let zip = out_dir.join("quota-station-0.1.0.zip");
    assert!(zip.is_file());
    assert!(!s.installed().exists(), "--package installs nothing");
    assert!(!s.calls().iter().any(|c| c.starts_with("open")));

    // The other Mac: no cargo, no npm, and the zip arrived with the mark
    // macOS puts on downloads.
    let other = Sandbox::new("install-other");
    std::fs::remove_file(other.bin.join("cargo")).unwrap();
    std::fs::remove_file(other.bin.join("npm")).unwrap();
    let copied = other.root.join("quota-station-0.1.0.zip");
    std::fs::copy(&zip, &copied).unwrap();
    quarantine(&copied);
    let unpacked = other.root.join("unpacked");
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&copied)
        .arg(&unpacked)
        .status()
        .unwrap();
    assert!(status.success());
    let folder = unpacked.join("quota-station-0.1.0");
    assert!(folder.join("install.sh").is_file());
    quarantine(&folder.join(APP));

    let path = format!("{}:/usr/bin:/bin:/usr/sbin:/sbin", other.bin.display());
    let out = Command::new("/bin/bash")
        .arg(folder.join("install.sh"))
        .args(["--app", folder.join(APP).to_str().unwrap(), "--dest", &other.dest()])
        .current_dir(&other.root)
        .env_clear()
        .env("HOME", other.root.join("home"))
        .env("TMPDIR", other.root.join("tmp"))
        .env("PATH", path)
        .env("FAKE_LOG", &other.log)
        .output()
        .unwrap();
    ok(&out);
    let app = other.installed();
    assert!(verifies(&app));
    assert!(!quarantined(&app));
    assert!(other.calls().iter().any(|c| c.starts_with("open ")));
    assert!(
        !other.calls().iter().any(|c| c.starts_with("security")),
        "a built copy is not signed again"
    );
    no_login_item(&other);

    // The zip itself works as --app too.
    let third = Sandbox::new("install-zip");
    ok(&third.run(&["--app", copied.to_str().unwrap(), "--dest", &third.dest(), "--no-open"]));
    assert!(verifies(&third.installed()));
    assert!(!quarantined(&third.installed()));
}

#[test]
fn refuses_an_app_that_is_not_quota_station() {
    let s = Sandbox::new("install-other-app");
    let other = s.root.join("Other.app");
    fake_bundle(&other, "com.example.other", "1.0");
    let out = s.run(&["--app", other.to_str().unwrap(), "--dest", &s.dest()]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("is not Quota Station"), "{}", stderr(&out));
    assert!(!s.installed().exists());
}

#[test]
fn refuses_a_copy_whose_signature_is_broken() {
    let s = Sandbox::new("install-broken");
    let app = s.root.join(APP);
    fake_bundle(&app, "com.shin315.quota-station", "0.1.0");
    let status = Command::new("/usr/bin/codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(&app)
        .output()
        .unwrap();
    assert!(status.status.success());
    std::fs::write(
        app.join("Contents/Info.plist"),
        INFO_PLIST.replace("BUNDLE_ID", "com.shin315.quota-station").replace("VERSION", "6.6.6"),
    )
    .unwrap();
    let out = s.run(&["--app", app.to_str().unwrap(), "--dest", &s.dest()]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("does not verify"), "{}", stderr(&out));
    assert!(!s.installed().exists());
}

#[test]
fn unknown_options_are_refused() {
    let s = Sandbox::new("install-usage");
    let out = s.run(&["--frobnicate"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unknown option --frobnicate"));
    let help = s.run(&["--help"]);
    ok(&help);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--package"));
    assert!(s.calls().is_empty());
}
