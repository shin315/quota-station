// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: CC-BY-NC-SA-4.0

//! Fakes for the Antigravity switch tests: on Unix an Antigravity.app that
//! is a shell process, with the `plutil`, `pgrep`, `osascript` and `open`
//! the switch asks about it; on Windows an Antigravity kept in memory behind
//! the switch's seam (`desktop::Host`), with a fake `Antigravity.exe` in a
//! temporary folder; and an `agy` whose `/quota` prints a chosen reading (a
//! shell script, on Windows a batch file). Nothing here looks at, quits or
//! opens the real Antigravity.

use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use quota_station_core::antigravity::desktop::Desktop;
use quota_station_core::antigravity::export;
use quota_station_core::app_dir::AppDir;
use quota_station_core::quota::agy;
use serde_json::{json, Value};

#[cfg(unix)]
use super::script;

#[cfg(unix)]
/// A fake Antigravity.app under `dir`: a bundle whose executable is a shell
/// loop, started in the background (so launchd, not the test, is its
/// parent and reaps it), and the four fake programs in `dir/bin`. Each
/// program records its arguments in `dir/calls`, one run per line.
pub struct FakeApp {
    pub dir: PathBuf,
    pub app: PathBuf,
    pub bin: PathBuf,
}

#[cfg(unix)]
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

    /// Leaves the app there with no version to read: its Info.plist goes.
    pub fn forget_version(&self) {
        std::fs::remove_file(self.app.join("Contents/Info.plist")).unwrap();
    }
}

#[cfg(unix)]
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

/// An `agy` that records its arguments in `dir/agy-args` and prints
/// `dir/agy.json`, or fails when there is none, with no process running:
/// agy's own `/quota` is the only reading.
#[cfg(unix)]
pub fn fake_agy(dir: &Path) -> agy::Tools {
    let d = dir.display();
    agy::Tools {
        agy: Some(script(
            &dir.join("bin/agy"),
            &format!(
                "printf '%s\\n' \"$@\" > '{d}/agy-args'\n[ -f '{d}/agy.json' ] || {{ echo 'agy: not signed in' >&2; exit 2; }}\ncat '{d}/agy.json'\n"
            ),
        )),
        processes: std::sync::Arc::new(super::os::FakeProcesses::none()),
    }
}

/// The same `agy`, as a batch file: each argument on a line of its own in
/// `dir\agy-args`, then `dir\agy.json`.
#[cfg(windows)]
pub fn fake_agy(dir: &Path) -> agy::Tools {
    let d = dir.display();
    let args = format!("(for %%a in (%*) do @echo %%~a) > \"{d}\\agy-args\"");
    let missing =
        format!("if not exist \"{d}\\agy.json\" (echo agy: not signed in 1>&2& exit /b 2)");
    let print = format!("type \"{d}\\agy.json\"");
    agy::Tools {
        agy: Some(super::batch(&dir.join("bin").join("agy"), &[&args, &missing, &print])),
        processes: std::sync::Arc::new(super::os::FakeProcesses::none()),
    }
}

// Tests that share `common` without the fake app leave it unused.
#[cfg(windows)]
#[allow(unused_imports)]
pub use self::windows::FakeApp;

/// The pid the fake Antigravity runs as on Windows.
#[cfg(windows)]
pub fn windows_pid() -> u32 {
    windows::PID
}

/// An Antigravity kept in memory on Windows: `Antigravity.exe` (an empty
/// file) and its `resources\app\product.json` in `dir\Antigravity`, and a
/// [`Host`] that lists its process while it runs, closes it when asked
/// (unless told to stay) and starts it again. Every question is recorded
/// under the name of the program Windows answers it with: `tasks` (the
/// process table), `taskkill`, `start` and `powershell`.
#[cfg(windows)]
mod windows {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use quota_station_core::antigravity::desktop::{Desktop, Host, Task};

    /// The pid the fake app runs as, and one of its helpers.
    pub const PID: u32 = 4242;
    const HELPER: u32 = 4243;

    type Item = Arc<dyn Fn() -> Option<String> + Send + Sync>;

    #[derive(Default)]
    struct State {
        running: bool,
        stays: bool,
        fails: BTreeSet<String>,
        calls: Vec<Vec<String>>,
        item: Option<Item>,
        item_at: BTreeMap<String, String>,
    }

    pub struct FakeApp {
        pub dir: PathBuf,
        /// The install folder.
        pub app: PathBuf,
        state: Arc<Mutex<State>>,
    }

    struct FakeHost(Arc<Mutex<State>>, PathBuf);

    impl FakeHost {
        fn record(&self, program: &str, args: &[String]) -> std::sync::MutexGuard<'_, State> {
            let mut state = self.0.lock().unwrap();
            let mut call = vec![program.to_string()];
            call.extend(args.iter().cloned());
            state.calls.push(call);
            state
        }

        fn keep(state: &mut State, when: &str) {
            if let Some(value) = state.item.as_ref().and_then(|item| item()) {
                state.item_at.insert(when.to_string(), value);
            }
        }
    }

    impl Host for FakeHost {
        fn processes(&self) -> Option<Vec<Task>> {
            let state = self.record("tasks", &[]);
            if state.fails.contains("pgrep") {
                return None;
            }
            let exe = self.1.join("Antigravity.exe");
            let mut tasks = vec![Task {
                pid: 7,
                exe: Some(PathBuf::from(r"C:\Windows\explorer.exe")),
                command: r"C:\Windows\explorer.exe".into(),
            }];
            if state.running {
                let command = format!("\"{}\" --reuse-window", exe.display());
                tasks.push(Task { pid: PID, exe: Some(exe.clone()), command });
                let helper = format!("\"{}\" --type=renderer", exe.display());
                tasks.push(Task { pid: HELPER, exe: Some(exe), command: helper });
            }
            Some(tasks)
        }

        fn close(&self, pid: u32) {
            let args = ["/PID".to_string(), pid.to_string(), "/T".to_string()];
            let mut state = self.record("taskkill", &args);
            Self::keep(&mut state, "quit");
            if pid == PID && !state.stays {
                state.running = false;
            }
        }

        fn start(&self, exe: &Path) -> Result<(), String> {
            let mut state = self.record("start", &[exe.display().to_string()]);
            Self::keep(&mut state, "open");
            if state.fails.contains("open") {
                return Err("The system cannot find the file specified. (os error 2)".into());
            }
            state.running = true;
            Ok(())
        }

        fn version_info(&self, exe: &Path) -> Result<String, String> {
            let _state = self.record("powershell", &[exe.display().to_string()]);
            Err("file does not exist".into())
        }
    }

    impl FakeApp {
        /// An installed app of `version` (`None`: not installed).
        pub fn new(dir: &Path, version: Option<&str>) -> Self {
            let app = dir.join("Antigravity");
            if let Some(version) = version {
                std::fs::create_dir_all(app.join("resources").join("app")).unwrap();
                std::fs::write(app.join("Antigravity.exe"), "").unwrap();
                std::fs::write(
                    app.join("resources").join("app").join("product.json"),
                    serde_json::json!({"nameShort": "Antigravity", "version": version}).to_string(),
                )
                .unwrap();
            } else {
                std::fs::create_dir_all(dir).unwrap();
            }
            FakeApp { dir: dir.to_path_buf(), app, state: Arc::default() }
        }

        fn state(&self) -> std::sync::MutexGuard<'_, State> {
            self.state.lock().unwrap()
        }

        pub fn desktop(&self) -> Desktop {
            Desktop::fake(vec![self.app.clone()], FakeHost(self.state.clone(), self.app.clone()))
        }

        /// Starts the fake app, as if the owner had it open.
        pub fn start(&self) {
            let mut state = self.state();
            state.running = true;
            state.calls.clear();
        }

        pub fn running(&self) -> bool {
            self.state().running
        }

        /// Waits up to five seconds for the fake app to be `running`.
        pub fn wait_running(&self, running: bool) -> bool {
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.running() != running && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            self.running() == running
        }

        /// Keeps what `item` gives at each quit and open, for [`FakeApp::item_at`].
        pub fn watch_item(&self, item: impl Fn() -> Option<String> + Send + Sync + 'static) {
            self.state().item = Some(Arc::new(item));
        }

        /// What the watched item held at the last `quit` or `open`.
        pub fn item_at(&self, when: &str) -> Option<String> {
            self.state().item_at.get(when).cloned()
        }

        /// Makes it ignore a close.
        pub fn stay(&self) {
            self.state().stays = true;
        }

        /// Makes one question fail, by the macOS fake's name: `pgrep` (the
        /// process table) or `open` (the start).
        pub fn fail(&self, what: &str) {
            self.state().fails.insert(what.to_string());
        }

        /// Every question: the program's name, then its arguments.
        pub fn calls(&self) -> Vec<Vec<String>> {
            self.state().calls.clone()
        }

        /// The runs of a program, named as on macOS (`pgrep`, `osascript`,
        /// `open`, `plutil`) or as on Windows (`tasks`, `taskkill`,
        /// `start`, `powershell`).
        pub fn runs_of(&self, program: &str) -> Vec<Vec<String>> {
            let program = match program {
                "pgrep" => "tasks",
                "osascript" => "taskkill",
                "open" => "start",
                "plutil" => "powershell",
                other => other,
            };
            self.calls().into_iter().filter(|c| c[0] == program).map(|c| c[1..].to_vec()).collect()
        }

        /// Leaves the app there with no version to read: its product.json
        /// goes, and its VersionInfo tells none.
        pub fn forget_version(&self) {
            std::fs::remove_file(self.app.join("resources").join("app").join("product.json"))
                .unwrap();
        }
    }
}

/// A quota summary with one Gemini five-hour bucket, `used` per cent used
/// and resetting at `reset`.
pub fn quota_summary(used: f64, reset: &str) -> Value {
    json!({"groups": [{"displayName": "Gemini", "buckets": [{
        "bucketId": "gemini-5h",
        "remainingFraction": 1.0 - used / 100.0,
        "resetTime": reset,
    }]}]})
}

/// What `agy -p /quota --output-format json` prints for that summary.
pub fn agy_quota(used: f64, reset: &str) -> String {
    json!({"command": {"name": "/quota", "data": quota_summary(used, reset)}}).to_string()
}

/// Keeps `email`'s reading of that summary in the app's own cache, as a
/// read of the account does.
pub fn remember(app: &AppDir, email: &str, used: f64, reset: &str, now_ms: i64) {
    export::write(app, email, None, json!({"quota_summary": quota_summary(used, reset)}), now_ms)
        .unwrap();
}
