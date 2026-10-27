// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

//! The macOS side of [`quota_station_core::os`], against fake `security`,
//! `launchctl`, `ps` and `lsof` shell scripts: the commands the app runs and
//! how it reads their answers. The other tests use the fakes kept in memory
//! (`common::os`).
#![cfg(unix)]

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{script, temp_dir};
use quota_station_core::claude::agent::{self, AgentError, Launchd};
use quota_station_core::claude::keychain::{KeychainError, Security};
use quota_station_core::os::{Process, ProcessList, Ps, Scheduler, SecretStore};

/// A `security` that keeps items as files in `dir/items/<service>--<account>`,
/// exits 44 for a missing one as macOS's does, takes a write from a
/// `security -i` line on stdin (the value as hex after `-X`), and records
/// every run (never a value) in `dir/calls`.
fn fake_security(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("items")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/security"),
        &format!(
            r#"if [ "$1" = "-i" ]; then
  IFS= read -r line
  eval "set -- $line"
  printf 'stdin ' >> '{d}/calls'
fi
cmd="$1"; shift
printf '%s' "$cmd" >> '{d}/calls'
svc=""; acct=""; hex=""; value=""; show=""
while [ $# -gt 0 ]; do
  case "$1" in
    -s) svc="$2"; printf ' -s %s' "$2" >> '{d}/calls'; shift 2 ;;
    -a) acct="$2"; printf ' -a %s' "$2" >> '{d}/calls'; shift 2 ;;
    -X) hex="$2"; printf ' -X' >> '{d}/calls'; shift 2 ;;
    -w) if [ $# -ge 2 ] && [ "$cmd" = add-generic-password ]; then value="$2"; shift 2; else show=1; shift; fi ;;
    *) shift ;;
  esac
done
printf '\n' >> '{d}/calls'
item='{d}/items/'"$svc--$acct"
missing() {{ echo "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain." >&2; exit 44; }}
case "$cmd" in
  add-generic-password)
    [ -e '{d}/fail-add' ] && {{ echo "security: SecKeychainItemCreateFromContent: User interaction is not allowed." >&2; exit 36; }}
    if [ -n "$hex" ]; then printf '%s' "$hex" | /usr/bin/xxd -r -p > "$item"; else printf '%s' "$value" > "$item"; fi ;;
  find-generic-password)
    [ -f "$item" ] || missing
    if [ -n "$show" ]; then cat "$item"; echo; fi ;;
  delete-generic-password)
    [ -f "$item" ] || missing
    rm "$item" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}

/// A `launchctl` that keeps loaded labels as files in `dir/loaded/`:
/// `list LABEL` exits 0 or 113, `bootstrap DOMAIN PLIST` loads the plist's
/// label, `bootout DOMAIN/LABEL` unloads it. Every run is recorded.
fn fake_launchctl(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("loaded")).unwrap();
    let d = dir.display();
    script(
        &dir.join("bin/launchctl"),
        &format!(
            r#"printf '%s\n' "$*" >> '{d}/launchctl-calls'
case "$1" in
  list) [ -e '{d}/loaded/'"$2" ] && exit 0; exit 113 ;;
  bootstrap) label=$(basename "$3" .plist); : > '{d}/loaded/'"$label" ;;
  bootout) label=${{2##*/}}; [ -e '{d}/loaded/'"$label" ] || exit 3; rm '{d}/loaded/'"$label" ;;
  *) exit 2 ;;
esac
"#
        ),
    )
}

#[test]
fn security_keeps_a_value_off_its_command_line_and_reads_it_back() {
    let dir = temp_dir("os-security");
    let security = Security::fake(fake_security(&dir));
    let calls = || std::fs::read_to_string(dir.join("calls")).unwrap_or_default();

    assert_eq!(security.get("svc", "acct"), Ok(None));
    assert_eq!(security.exists("svc", "acct"), Ok(false));
    security.set("svc", "acct", "{\"secret\":\"Lâm\"}").unwrap();
    assert_eq!(security.get("svc", "acct").unwrap().as_deref(), Some("{\"secret\":\"Lâm\"}"));
    assert_eq!(security.exists("svc", "acct"), Ok(true));
    assert!(calls().contains("stdin add-generic-password -a acct -s svc -X\n"), "{}", calls());

    // Too long for one `security -i` line: the hex goes on the command line.
    let long = "x".repeat(3000);
    security.set("svc", "long", &long).unwrap();
    assert_eq!(security.get("svc", "long").unwrap(), Some(long));
    assert!(calls().lines().any(|l| l == "add-generic-password -a long -s svc -X"), "{}", calls());

    security.delete("svc", "acct").unwrap();
    security.delete("svc", "acct").unwrap();
    assert_eq!(security.get("svc", "acct"), Ok(None));

    std::fs::write(dir.join("fail-add"), "").unwrap();
    let refused = security.set("svc", "acct", "v").unwrap_err();
    assert!(
        matches!(&refused, KeychainError::Failed { code: Some(36), message } if message.contains("not allowed")),
        "{refused:?}"
    );
    let gone = Security::fake(dir.join("bin/missing"));
    assert!(matches!(gone.get("svc", "acct"), Err(KeychainError::Spawn(_))));
}

#[test]
fn launchd_loads_the_rotation_agent_and_unloads_it() {
    let dir = temp_dir("os-launchd");
    let home = dir.join("home");
    let exe = dir.join("Quota Station.app/Contents/MacOS/quota-station");
    let launchd = Launchd { launchctl: fake_launchctl(&dir) };
    let plist = agent::plist_path(&home);

    assert_eq!(launchd.loaded(), Some(false));
    assert_eq!(launchd.legacy_loaded(), None);
    launchd.install(&home, &exe).unwrap();
    assert_eq!(launchd.loaded(), Some(true));
    let text = std::fs::read_to_string(&plist).unwrap();
    assert!(text.contains(&format!("<string>{}</string>", exe.display())), "{text}");
    let calls = std::fs::read_to_string(dir.join("launchctl-calls")).unwrap();
    assert!(calls.lines().any(|l| l.starts_with("bootstrap gui/")), "{calls}");

    launchd.remove(&home).unwrap();
    assert_eq!(launchd.loaded(), Some(false));
    assert!(!plist.exists());

    // An older switcher loaded: nothing is installed beside it.
    std::fs::write(dir.join("loaded/com.cswap.prefer-team"), "").unwrap();
    assert_eq!(launchd.legacy_loaded(), Some("com.cswap.prefer-team"));
    assert_eq!(
        launchd.install(&home, &exe),
        Err(AgentError::Legacy("com.cswap.prefer-team".into()))
    );

    // `launchctl` that cannot tell counts as an older switcher running.
    let unknown = Launchd { launchctl: script(&dir.join("bin/launchctl-unknown"), "exit 1\n") };
    assert_eq!(unknown.loaded(), None);
    assert_eq!(unknown.legacy_loaded(), Some("com.cswap.prefer-team"));
}

#[test]
fn ps_and_lsof_list_the_processes_and_their_ports() {
    let dir = temp_dir("os-ps");
    let ps = script(
        &dir.join("bin/ps"),
        "case \"$*\" in\n  '-ww -Ao tty=,command=') printf 'ttys001 codex --resume\\n??       /Applications/Codex.app/Contents/MacOS/Codex\\n' ;;\n  '-x -u '*' -o pid=,command=') printf '  PID COMMAND\\n 4242 /usr/local/bin/agy --resume\\n' ;;\n  *) exit 2 ;;\nesac\n",
    );
    let lsof = script(
        &dir.join("bin/lsof"),
        "[ \"$*\" = '-nP -a -p 4242 -iTCP -sTCP:LISTEN' ] || exit 1\necho 'COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME'\necho 'agy 4242 me 9u IPv4 0x1 0t0 TCP 127.0.0.1:53210 (LISTEN)'\n",
    );
    let list = Ps { ps, lsof };
    let wait = Duration::from_secs(5);

    let all = list.all().unwrap();
    assert_eq!(
        all[0],
        Process { pid: 0, tty: Some("ttys001".into()), command: "codex --resume".into() }
    );
    assert_eq!(all[1].tty, None);
    assert_eq!(
        list.own(wait).unwrap(),
        [Process { pid: 4242, tty: None, command: "/usr/local/bin/agy --resume".into() }]
    );
    assert_eq!(list.listening(4242, wait), Some(vec![53210]));
    assert_eq!(list.listening(7, wait), None);

    let none = Ps { ps: dir.join("bin/missing"), lsof: dir.join("bin/missing") };
    assert_eq!(none.all(), None);
    assert_eq!(none.own(wait), None);
    assert!(list.alive(std::process::id() as i32));
}
