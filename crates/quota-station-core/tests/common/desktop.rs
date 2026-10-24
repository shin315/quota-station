// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Fakes for the Antigravity switch tests: an Antigravity.app that is a
//! shell process, the `plutil`, `pgrep`, `osascript` and `open` the switch
//! asks about it, and a quota-axi that prints a chosen Antigravity report.
//! Nothing here looks at, quits or opens the real Antigravity.app.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use quota_station_core::antigravity::desktop::Desktop;

use super::script;

/// A fake Antigravity.app under `dir`: a bundle whose executable is a shell
/// loop, started in the background (so launchd, not the test, is its
/// parent and reaps it), and the four fake programs in `dir/bin`. Each
/// program records its arguments in `dir/calls`, one run per line.
pub struct FakeApp {
    pub dir: PathBuf,
    pub app: PathBuf,
    pub bin: PathBuf,
}

impl FakeApp {
    /// An installed app of `version` (`None`: not installed).
    pub fn new(dir: &Path, version: Option<&str>) -> Self {
        let app = dir.join("Antigravity.app");
        let bin = dir.join("bin");
        let fake = FakeApp { dir: dir.to_path_buf(), app: app.clone(), bin: bin.clone() };
        if let Some(version) = version {
            std::fs::create_dir_all(app.join("Contents")).unwrap();
            std::fs::write(
                app.join("Contents/Info.plist"),
                format!("CFBundleIdentifier=com.google.antigravity\nCFBundleShortVersionString={version}\n"),
            )
            .unwrap();
            script(
                &app.join("Contents/MacOS/Antigravity"),
                "trap 'exit 0' TERM\nwhile :; do sleep 0.05; done\n",
            );
        }
        let d = dir.display();
        let record = format!(
            "n=\"${{0##*/}}\"; printf '%s' \"$n\" >> '{d}/calls'; for a in \"$@\"; do printf '\\037%s' \"$a\" >> '{d}/calls'; done; printf '\\n' >> '{d}/calls'\n"
        );
        // `plutil -extract KEY raw -o - PLIST`: the fake plist is KEY=VALUE lines.
        script(
            &bin.join("plutil"),
            &format!(
                "{record}[ \"$1\" = -extract ] || exit 2\n[ -f \"$6\" ] || {{ echo 'file does not exist' >&2; exit 1; }}\nv=$(grep \"^$2=\" \"$6\" | cut -d= -f2-)\n[ -n \"$v\" ] || exit 1\necho \"$v\"\n"
            ),
        );
        // `pgrep -f PATTERN`: 0 while the fake app's process lives, else 1.
        script(
            &bin.join("pgrep"),
            &format!(
                "{record}[ -e '{d}/pgrep-fails' ] && {{ echo 'pgrep: cannot list processes' >&2; exit 3; }}\npid=$(cat '{d}/pid' 2>/dev/null) || exit 1\nkill -0 \"$pid\" 2>/dev/null && exit 0\nexit 1\n"
            ),
        );
        // What the shared login held when the app was asked to quit and
        // when it was opened, to check the order of a switch.
        let keep = |when: &str| {
            format!("[ -f '{d}/item-path' ] && cat \"$(cat '{d}/item-path')\" > '{d}/item-at-{when}' 2>/dev/null\n")
        };
        // `osascript -e 'tell application id … to quit'`: a TERM the fake app
        // honours, unless it was told to stay. Its pid file goes with it, so
        // nothing signals that pid again.
        script(
            &bin.join("osascript"),
            &format!(
                "{record}{}[ -e '{d}/stays' ] && {{ echo 'execution error: User canceled. (-128)' >&2; exit 1; }}\npid=$(cat '{d}/pid' 2>/dev/null) || exit 0\nkill -TERM \"$pid\" 2>/dev/null\ni=0\nwhile kill -0 \"$pid\" 2>/dev/null && [ $i -lt 250 ]; do sleep 0.02; i=$((i+1)); done\nkill -0 \"$pid\" 2>/dev/null || rm -f '{d}/pid'\nexit 0\n",
                keep("quit")
            ),
        );
        // `open -b com.google.antigravity`: starts the fake app again.
        script(
            &bin.join("open"),
            &format!(
                "{record}{}[ -e '{d}/open-fails' ] && {{ echo 'LSOpenURLsWithRole() failed with error -10810' >&2; exit 1; }}\n'{exe}' >/dev/null 2>&1 &\necho $! > '{d}/pid'\n",
                keep("open"),
                exe = app.join("Contents/MacOS/Antigravity").display()
            ),
        );
        fake
    }

    pub fn desktop(&self) -> Desktop {
        Desktop::fake(&self.bin, &self.app)
    }

    /// Starts the fake app, as if the owner had it open.
    pub fn start(&self) {
        let out = std::process::Command::new(self.bin.join("open"))
            .args(["-b", "com.google.antigravity"])
            .output()
            .unwrap();
        assert!(out.status.success());
        std::fs::write(self.dir.join("calls"), "").unwrap();
        assert!(self.running(), "the fake app started");
    }

    /// Whether the fake app's process lives, asked without the fake `pgrep`
    /// so it records nothing.
    pub fn running(&self) -> bool {
        let Ok(pid) = std::fs::read_to_string(self.dir.join("pid")) else { return false };
        std::process::Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Waits up to five seconds for the fake app to be `running`.
    pub fn wait_running(&self, running: bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.running() != running && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        self.running() == running
    }

    /// Keeps what `item` holds at each quit and open, for [`FakeApp::item_at`].
    pub fn watch_item(&self, item: &Path) {
        std::fs::write(self.dir.join("item-path"), item.display().to_string()).unwrap();
    }

    /// What the watched item held at the last `quit` or `open`.
    pub fn item_at(&self, when: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(format!("item-at-{when}"))).ok()
    }

    /// Makes it ignore a quit.
    pub fn stay(&self) {
        std::fs::write(self.dir.join("stays"), "").unwrap();
    }

    pub fn fail(&self, what: &str) {
        std::fs::write(self.dir.join(format!("{what}-fails")), "").unwrap();
    }

    /// Every run of the four programs: the program's name, then its arguments.
    pub fn calls(&self) -> Vec<Vec<String>> {
        let text = std::fs::read_to_string(self.dir.join("calls")).unwrap_or_default();
        text.lines().map(|l| l.split('\u{1f}').map(String::from).collect()).collect()
    }

    pub fn runs_of(&self, program: &str) -> Vec<Vec<String>> {
        self.calls().into_iter().filter(|c| c[0] == program).map(|c| c[1..].to_vec()).collect()
    }
}

impl Drop for FakeApp {
    fn drop(&mut self) {
        // Only the fake's own shell loop, by the pid it wrote; a quit fake
        // has no pid file left.
        if let Ok(pid) = std::fs::read_to_string(self.dir.join("pid")) {
            let _ = std::process::Command::new("/bin/kill")
                .args(["-9", pid.trim()])
                .stderr(std::process::Stdio::null())
                .status();
        }
    }
}

/// A quota-axi that records its arguments in `dir/axi-args` and prints
/// `dir/agy.json`, or fails when there is none.
pub fn fake_quota_axi(dir: &Path) -> PathBuf {
    let d = dir.display();
    script(
        &dir.join("bin/quota-axi"),
        &format!(
            "printf '%s\\n' \"$@\" > '{d}/axi-args'\n[ -f '{d}/agy.json' ] || {{ echo 'agy: not signed in' >&2; exit 2; }}\ncat '{d}/agy.json'\n"
        ),
    )
}

/// An Antigravity report whose `cockpit` block lists `accounts` accounts
/// from `source`, with `active` in use as told by `basis`.
pub fn agy_report(accounts: u32, active: Option<u32>, basis: &str, source: &str) -> String {
    let rows: Vec<String> = (1..=accounts)
        .map(|n| {
            format!(
                r#"{{"account":{n},"active":{},"status":"noData","scopes":[]}}"#,
                Some(n) == active
            )
        })
        .collect();
    let head = match active {
        Some(a) => format!(r#""activeAccount":{a},"activeBasis":"{basis}","#),
        None => r#""activeUnresolved":"no_evidence","#.to_string(),
    };
    format!(
        r#"{{"generatedAt":"2026-01-16T07:21:00.000Z","schemaVersion":5,"providers":[{{"provider":"agy","windows":[],"state":{{"status":"fresh","stale":false}},"cockpit":{{"status":"ok","source":"{source}",{head}"accounts":[{}]}}}}]}}"#,
        rows.join(",")
    )
}
