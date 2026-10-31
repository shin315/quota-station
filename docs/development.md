# Developing Quota Station

How to build, run, preview and test the app, and how the code is laid out.
The user-facing side is in the [README](../README.md); what the app reads,
runs and writes is in [how-it-works.md](how-it-works.md).

- [Build and run](#build-and-run)
- [Build features](#build-features)
- [Browser preview](#browser-preview)
- [The README's screenshots](#the-readmes-screenshots)
- [Tests](#tests)
- [Layout](#layout)
- [Front-end stack](#front-end-stack)
- [Milestones](#milestones)

## Build and run

You need Rust stable (1.85 or later) and Node.js 20 or later (CI uses 24).

```sh
npm install
npm run app            # dev: Vite and the Tauri app, with reload
```

To make a release `.app` without installing it:

```sh
cargo install tauri-cli --version '^2' --locked   # once
cargo tauri build
# or, with the CLI from npm:
npm run app:build
```

The bundle is written to `target/release/bundle/macos/Quota Station.app`. It is
ad-hoc signed and is not installed anywhere. Open it from there to try it. A
copy run this way or with `npm run app` cannot launch at login or update
itself; `scripts/install.sh` installs it.

### What `scripts/install.sh` does

`scripts/install.sh` runs `npm ci` when `package-lock.json` changed, makes a
release build (`cargo tauri build --bundles app` when the Tauri CLI is installed
with cargo, else the npm one) with both [build features](#build-features) on,
signs it with the first `Apple Development` identity in the Keychain or ad-hoc
when there is none (`--identity <name>` picks one, `--identity -` forces
ad-hoc), checks the signature, copies it to `/Applications/Quota Station.app`
(next to the old copy first, then swapped, so a failed copy leaves the old one),
clears the quarantine mark and opens it. Options: `--dest <folder>` installs
elsewhere, `--no-open` does not open it, `--package [DIR]` builds and signs,
then writes `quota-station-<version>.zip` instead of installing, `--app <path>`
installs a built `Quota Station.app` (or that zip) instead of building, and
`--help` lists them. It never adds a login item and never asks for `sudo`.
While the old copy runs it is not opened again: quit it from its menu and open
the new one.

A copy given with `--app` must be Quota Station (`com.shin315.quota-station`)
with a valid signature; it is installed as it is, not signed again, with the
quarantine mark macOS puts on downloads and AirDrop cleared. The zip holds the
signed app and `install.sh`, so a Mac without Rust or Node.js can install it:

```sh
scripts/install.sh --package ~/Desktop          # writes quota-station-<version>.zip
# on the other Mac, where the zip arrived:
ditto -x -k quota-station-0.1.0.zip . && ./quota-station-0.1.0/install.sh --app "./quota-station-0.1.0/Quota Station.app"
```

## Build features

Two Cargo features decide what is compiled in. A plain `cargo tauri build`,
`npm run app` or a release build has neither; `scripts/install.sh` turns both
on (`QS_ANTIGRAVITY_ACCOUNTS=0` or `QS_AUTHOR_TOOLS=0` leaves one out).

- **`antigravity-accounts`**: logging in and switching several Antigravity
  accounts, adapted from Cockpit Tools (see [`THIRD_PARTY.md`](../THIRD_PARTY.md)).
  The Rust side is the feature; the interface's
  screens go into the bundle only with `QS_ANTIGRAVITY_ACCOUNTS=1` (Vite swaps
  in `src/antigravity-off/` otherwise, and a build without it stops if it would
  bundle a file from `src/antigravity/`). By hand:
  `cargo tauri build --features antigravity-accounts` with
  `QS_ANTIGRAVITY_ACCOUNTS=1`. Without it the app reads the
  Antigravity account signed in and does not log in or switch accounts.
- **`author-tools`**: the tools made for the author's own Macs: `Settings ›
  Advanced` and what it holds, the `Rotation rules` editor and the copy of the
  rotation file to the other Macs, `Copy to another Mac`, `Source code` and
  `Update from source`, the `Firstmate` tab with its file and its notification,
  the notification for an account leaving the rotation, and the copy of the
  Claude accounts in claude-swap's layout (`claude/swap/`). A build without it
  compiles none of that code and has none of its commands;
  `quota-station agents` refuses there as a copy of another version would.
  `Advanced` counts as off whatever the settings file says, so the windows show
  no `Advanced` tab and none of the tabs, cards, switches or names it holds
  (automatic switching with its pause and the background check card, the
  Personal/Team and Ultra names). The model says which build it is
  (`capabilities.authorTools`). By hand: `cargo tauri build --features
  author-tools`, or `npm run app -- --features author-tools`. The author's
  tools are Unix only.

The settings file is the same in every build: a build without `author-tools`
reads the values `Advanced` holds (its two switches, the other Macs, the clone,
Firstmate's file, the switches of those two notifications) and writes them back
as they were, so a settings file written by either build loads unchanged in the
other.

## Browser preview

`npm run dev` serves the popover at http://localhost:1420/ on the fixtures and
the fake Rust side the tests use, without Tauri. Nothing is switched, no
browser opens and no file is written: an action only waits (a Claude switch
1.5 s, a Claude account action or an Antigravity login 3 s, a rules save 1 s,
an update step 1.5 s, MCP 0.6 s, a restore 1.2 s, skills 0.8 s, a copy 0.9 s,
a set 0.5 s, an Agent updates check 1 s and an update 2 s) and updates the
fixture. The comment at the top of `src/preview.ts` explains each state; the
query parameters are:

- `state`: a snapshot in `src/fixtures/snapshots/`: `fresh` (the default),
  `exhausted`, `keychain`, `no-antigravity`, `reset-pending`, `single`,
  `stale`, `unavailable` or `unnamed`.
- `view=window` with `tab`: `overview`, `stats`, `claude`, `agy`, `codex`,
  `rules`, `history`, `alerts` (Overview with the bell's panel open),
  `firstmate`, `mcp`, `skills`, `plugins`, `instr`, `sets`, `cliTools`,
  `backup`, `copy` or `settings`.
- The look: `lang` (`vi` or `en`; without it the preview follows the browser's
  language, as the app follows macOS), `theme` (`light` or `dark`), `pack` (a
  theme pack: `default`, `kinhNhip`, `daiSangTinh`, `motNhipNhin`,
  `banDieuKhien` or `chieuLang`), `rt=1` (Reduce transparency), `os=win` (the
  app as Windows draws it) and `email=off` (names by place).
- The build: `build=release` (the downloaded app: no author's tools, no
  Antigravity accounts).
- The first run: `setup` (`on`, `signedout` for Claude Code signed in to none,
  `none` for no agent).
- Releases: `release` (`avail`, `later`, `skipped`, `offline`, `none` or `off`;
  up to date by default) and `releasefinds=offline`.
- Claude: `switcher` (`held`, `noguard`, `auto`, `stopped` or `missing`),
  `outcome` (`raced`, `failed`, `busy` for Claude Code's locks held, or
  `unconfirmed`; a switch lands by default), `accounts=none` (a Mac where Quota
  Station keeps no Claude account yet) and `claude` (`relogin`, `renew` or
  `fail`).
- Rotation rules, on `tab=rules`: `rotation` (`unreachable`, `newer`, `edited`,
  `unstamped`, `missing`, `invalid`, `unwritable` or `single`).
- Antigravity: `agy` (`two`, `relogin`, `verify`, `noclient`, `tooopen`,
  `invalid`, `broken`, `reading`, `failing` or `forbidden`), `login` (how a
  login ends: `timeout`, `cancelled`, `noCode`, `stateMismatch`,
  `noRefreshToken`, `invalidGrant`, `network`, `keychain`, `index` or
  `browser`), `agyswitch` (`on`, or `overwritten` for the banner), `agyapp`
  (`closed`, `missing`, `old`, `unreadable` or `unknown`; Antigravity.app runs
  by default), `agyoutcome` (`unconfirmed`, `notquit` or `write`) and
  `agyhot=on`.
- The menu bar settings: `menubar` (`claude`, `gemini` or `codex`) and
  `mbwindow` (`fiveHours`, `week`, `fableWeek` or `tightest`).
- History, notifications and Firstmate: `history=empty`, `alerts` (`on`, the
  default on `tab=alerts`, `off`, `unbundled`, `empty`, `many` or `env`) and
  `firstmate` (`ok`, `holding`, `stale`, `missing` or `unset`, the default).
- The app, on `tab=settings`: `app` (`on`, `approval`, `unavailable`,
  `nosource` or `unbundled`) and `update` (`uptodate`, `pull`, `install` or
  `timeout`; by default an update installs 0.1.1).
- Codex: `codex` (`chatgpt`, `ccswitch`, `down`, `external`, `nologin`,
  `nokey`, `placeholder`, `header`, `envkey`, `noplugins`, `broken` or `none`;
  the custom provider is in use by default), `codexoutcome` (`changed` or
  `refused`), `codexaccounts` (`one` or `empty`; several by default),
  `chatgpts=N` (with `one`, at most 5), `codexlogin` (`dup`, `timeout`, `port`,
  `noCodex`, `denied` or `other`), `codexprovider` (`down` or `rejected`),
  `codeximport=none` and `codextrigger` (`on` or `off`).
- MCP, on `tab=mcp`: `mcp` (`empty`, `parse`, `noagy`, `nocli`, `unread` or
  `brands`) and `mcpoutcome=changed`.
- Backup, on `tab=backup`: `backup` (`empty`, `icloud`, `noicloud`, `restored`
  or `unread`) and `restoreoutcome` (`changed`, `undochanged` or `partial`).
- Skills, Plugins and Instruction files: `skills` (`empty`, `nonpx`,
  `offline`, `unread` or `sets`), `skillsoutcome` (`changed` or `updatefail`),
  `plugins` (`noclaude` or `brands`) and `instr=clean`.
- Copy to another Mac, on `tab=copy` with two made-up Macs: `copy` (`unreach`,
  `old`, `noapp`, `none` or `changed`).
- Variables that override a login: `env` (`found`, `claude2`, `launchd`,
  `unread` or `unchecked`), on `tab=settings` and `tab=claude`.
- Config sets, on `tab=sets`: `sets` (`empty`, `fresh` or `even`) and
  `setsoutcome=changed`.
- Stats, on `tab=stats`: `stats` (`first`, `new`, `skip` or `fail`).
- Agent updates, on `tab=cliTools`: `cli` (`running`, `missing`, `npm`,
  `bundled`, `unwritable`, `unknown`, `offline`, `never`, `updated`,
  `updating`, `latest`, `failed` or `checking`, several joined with commas),
  `clifail` (`offline`, `permission`, `busy`, `exit`, `timeout`, `unchanged`,
  `started` or `sessions`) and `cliclosed=1`.

## The README's screenshots

The screenshots in [`screenshots/`](screenshots/) come from this preview, never
from the real app (which shows real accounts and emails): `screenshots/en/`
with `lang=en` and `screenshots/vi/` with `lang=vi`. The popover is `theme=dark`
at 396 px wide, cut to its card; the main window is the light look at 1100×760
(the first run's second step at 1100×380), and Settings shows its `Display` tab;
every capture is at device scale 2. `screenshots/menubar.png` is drawn, not
captured: the C04 glyph from `render_glyph` in
`crates/quota-station-core/src/icon.rs`, with the titles the snapshot fixtures
give, in a light and a dark menu bar.

## Tests

```sh
cargo test --workspace   # the core and the app crate, the MIT build
npm test                 # Vitest: every window, sheet and tab, in Vietnamese and English
```

CI (`.github/workflows/ci.yml`) has three jobs:

- **Licenses**: `reuse lint`.
- **Build and test (macOS)**, the required check: Vitest, the frontend's type
  check and build with and without `QS_ANTIGRAVITY_ACCOUNTS=1`, rustfmt, and
  clippy and `cargo test --workspace` four times: with no feature, with
  `antigravity-accounts`, with `author-tools` and with both, as
  `scripts/install.sh` builds; then the MIT app (`tauri build --bundles app`).
  The tests of the other Macs, the copy to another Mac and the update from
  source build only with `author-tools`, and the tests of the settings file,
  the alerts, the Claude accounts' copy, the model and the list of commands
  check each build's side (`npm test` checks the windows of both).
- **Build and test (Windows x64)**: Vitest, the core's `cargo check` and tests,
  clippy, and the app crate's tests, each with and without
  `antigravity-accounts` (the author's tools are Unix only), then an unsigned
  NSIS test installer built with `QS_TEST_SESSION=1` for the guided sessions on
  a Windows PC, uploaded as the workflow artifact
  `quota-station-windows-x64-test-setup` (kept 14 days). No release carries it.

The core's integration tests are `crates/quota-station-core/tests/*.rs`:
`agents_backup`, `agents_copy`, `agents_mcp`, `agents_restore`, `agents_sets`,
`agents_skills`, `alerts`, `antigravity_credential`, `antigravity_login`,
`antigravity_quota`, `antigravity_spy`, `antigravity_switch`,
`claude_accounts`, `claude_import`, `claude_manage`, `claude_rotation`,
`claude_switch`, `claude_windows`, `cli_tools`, `codex`, `codex_accounts`,
`codex_wake`, `detail`, `env_check`, `history`, `icon`, `install_script`,
`login_item`, `os_macos`, `pause`, `peers`, `quota_readers`, `rotation`,
`runner`, `sessions`, `settings`, `snapshot`, `stats`, `switcher`,
`switching`, `ui_fixtures` and `update`, with shared helpers in `common/` and
synthetic data in `fixtures/`. The app crate (`src-tauri/src`) has its tests
inline. The Vitest tests are the `*.test.ts(x)` files under `src/`; the
`*.en.test.*` files draw the windows in English and the `*.win.test.*` files as
Windows draws them.

What the main suites check:

- **Install** (`install_script.rs`) runs `scripts/install.sh` with macOS's
  `/bin/bash` on a clean temporary folder: a source tree holding only the
  script and the files it checks, a fake `cargo` whose `tauri build` writes a
  small bundle, and an Applications folder in the same folder. The real
  `codesign`, `ditto`, `xattr` and `plutil` work on those files; `open`,
  `security`, `npm` and `npx` are fakes that record their arguments, and so are
  `launchctl`, `osascript` and `sfltool`, which must never run. They check the
  build output, the ad-hoc and the Apple Development signature, the copy and the
  swap with an old copy, the quarantine mark cleared, `open` called once (or not
  with `--no-open`), a failed build that leaves the old copy, `npm ci` after a
  lock file change, the commit passed to the build, and a package that installs
  with `--app` on a Mac without `cargo` or `npm`, from the folder or the zip.
- **Update and login item** (`update.rs`, `login_item.rs`): real git on
  temporary repositories (a bare origin, the clone, and a second clone that
  pushes) with a temporary HOME and no git config, and a fake `install.sh` that
  records its arguments: a new commit pulled and installed, nothing new, a clone
  that cannot fast-forward (left as it is), an origin that is gone, a failed
  install, and what counts as a clone. The login item tests use a fake backend
  that keeps the state macOS would: on, off, waiting for approval, not a
  packaged app, and refused. Reading the switch never registers anything. No
  test opens an app, registers a login item or writes to `/Applications`.
- **Antigravity logins** (`antigravity_login.rs`, `antigravity_credential.rs`)
  never reach Google or the Keychain. They run against a local fake Google (a
  consent page that redirects to the app's callback with a code and a state,
  `/token` and `/userinfo`), a fake browser that records the URL it is asked to
  open and follows the redirect, and a fake `security` that records its
  arguments and keeps items in a temporary folder. They cover a wrong `state`, a
  consent without a code, no refresh token, `invalid_grant` on the exchange and
  on a refresh, the 10-minute timeout, a pasted callback URL, a cancel, a
  Keychain that refuses (the index stays unwritten), logging out, and the
  `oauth-client.json` checks. The spy test (`antigravity_spy.rs`) is described
  in [`SECURITY.md`](../SECURITY.md).
- **Antigravity quota** (`antigravity_quota.rs`) runs against the same fake
  Google, which also answers `loadCodeAssist`, `fetchAvailableModels` and
  `retrieveUserQuotaSummary` with fixtures in Cockpit's cache format
  (`tests/fixtures/cloud-code/`), under a `daily` and a `prod` path; the hosts
  can be changed only in a test build. They compare the headers, bodies and user
  agents with Cockpit's, with and without a fake Antigravity bundle on disk;
  check that the cache file is the record Cockpit writes, byte for byte; and
  cover the 60-second reuse, one account at a time with the one in use first, a
  refresh that is due, `invalid_grant`, 401, 403, 429 and 5xx, network
  failures, and a verification request. One more test reads the Antigravity row
  from the files they write, with a fake `agy`, and expects both accounts listed
  with the one in use told by its reading.
- **Antigravity switch** (`antigravity_switch.rs`) never touches the Keychain,
  Antigravity.app or the real `agy`. A fake `security` checks that the shared
  login is written with Cockpit's exact arguments, keeps the old item when the
  write fails and is stopped when it hangs (the two tests ported from Cockpit).
  Antigravity.app is a shell process behind fake `plutil`, `pgrep`,
  `osascript` and `open`: one that quits, one that stays (the shared login is
  not touched and it keeps running), one opened again before a plain switch,
  one whose plist cannot be read, one older than 2.0 and none at all; the fake
  `open` also records that the new login was in place before the app came back.
  A fake `agy` answers the check with the new account, the old one, or no
  answer. The project lookup before a switch is the quota read's
  `loadCodeAssist`, against the same fake Google. They also cover a token
  refreshed before the write, an account that needs a new login or is logged
  out while it is prepared, the 60-second preparation and one that breaks, one
  switch at a time, a reopen that fails, and the six-hour watch on synthetic
  readings.
- **Claude accounts and switching** (the `claude_*.rs` files, `switching.rs`,
  `switcher.rs`, `sessions.rs`, `pause.rs`) never touch a real account, the real
  Keychain or Anthropic. They run in a temporary HOME with a fake `security`
  that keeps items in files (and can refuse a write or drop one, to check that a
  switch puts everything back), a fake `launchctl`, a fake `claude` and a local
  fake Anthropic for the usage and token calls; every login is made up, and with
  the `test-support` feature `Security::system()` and `Endpoints::anthropic()`
  refuse to run. They cover the switch and its locks, a login the app does not
  keep, the usage reads and their back-off, a refresh of an account not in use
  and never of the one in use, the import (claude-swap's folder and items
  unchanged, running it again changes nothing), adding and removing accounts,
  the session folders, the command line, and the rotation's decisions on the
  same cases as the old script; one ignored test runs the real
  `cswap-prefer-team` with `--dry-run` beside the port when
  `QUOTA_STATION_PREFER_TEAM` names it. The read after a switch is the app's own
  reading of the accounts. Session counting runs on session files in a
  temporary folder, and the pause flag tests write it there with an `until`
  ahead, passed, malformed or missing.
- **Rotation and the other Macs** (`rotation.rs`, `peers.rs`) write rotation
  files in temporary folders: one the rules accept, and ones they refuse, which
  leave the file as it was. The other-Mac tests put a fake `ssh` first on
  `PATH`; it runs the fixed scripts in a temporary home per alias, and has an
  alias that cannot be reached and one that reads back something else. They
  cover a Mac with no file, an older copy (replaced, backup kept, ten newest
  kept), the copy already there, the 5-minute retry, a newer copy that wins, and
  a copy edited by hand.
- **History and notifications** (`history.rs`, `alerts.rs`) run on synthetic
  reports, switcher logs and app folders in temporary directories, with a fake
  notification center that records each card, its buttons and what was
  withdrawn, and presses a button. They step through readings: the pool going
  dry and coming back, an account leaving the rotation in 30 minutes (and not
  for an account that left already), a refused Claude login, Antigravity
  accounts that need a new login or a verification, an overwrite within 6 hours
  (offering to switch back twice, then only saying so), and Firstmate's
  `pool=dry`, fresh and stale. They check that each event alerts once, that a
  switched-off kind sends nothing, that the reminder for tomorrow comes back at
  09:00, that a test card changes nothing, and that a press counts once on
  either card. The history tests merge the app's journal, its switch logs and
  the switcher's log, and keep a change nothing explains as observed only once.
  Nothing posts a real notification.
- **Quota readers** (`quota_readers.rs`) never use a real login: Codex,
  Antigravity and Claude Code each run against a local fake server, a fake
  `agy`, `ps` and `lsof`, and a fake `security`, with made-up tokens. For each
  reader they cover a fresh reading, the last one shown as stale when the next
  read fails, an expired or refused login (never refreshed: the login file and
  the Keychain item stay as they were) and a Mac without the tool; for Claude
  also a Keychain that holds back until the owner presses the button, and for
  Antigravity that `agy` never opens a browser. One test runs with only
  `/usr/bin:/bin:/usr/sbin:/sbin` on `PATH`, so no quota-axi can be found, and
  still reads every service.
- **Codex** (`codex.rs`, `codex_accounts.rs`, `codex_wake.rs`) works on a
  temporary Codex folder filled from `tests/fixtures/codex/` (fake providers,
  fake keys and fake logins) and a temporary app folder. They check that a
  provider switch either way changes only the owned lines and leaves every other
  byte, `auth.json` (bytes and modification time) and the MCP list as they were;
  that switching there and back gives the original file; that
  `requires_openai_auth` follows the rule for each kind of key, with and without
  a login; that each refusal writes nothing; that a file changed after the sheet
  opened, or between the check and the rename, is not replaced; that a
  symlinked file stays a symlink; the backups and their mode; the record of the
  app's own write; the login shapes; the process table; the restart command and
  its `CODEX_HOME` (a fake `codex`); and the reachability check against a local
  server, which sees no key. No test reads the real `~/.codex`, runs the real
  Codex or touches a login.
- **MCP** (`agents_mcp.rs`) works on a temporary home holding a Codex
  `config.toml` and an Antigravity `mcp_config.json` from fixtures, a fake
  `claude` that keeps `~/.claude.json` and records its arguments, and a fake
  `security`. They check that one preview leads to one write per file that
  changes only the entries in the batch, byte for byte, and that the undo gives
  back the files as they were; that a file changed after the preview stops the
  whole batch; that a file that does not parse is never written; Claude Code's
  off, putting a server away and bringing it back, editing, and adding with a
  typed or pasted key; and that the trap keys in the fixtures show up nowhere
  the app keeps or sends anything. The Vitest tests check the checkboxes' roles
  and keys, the column menus (roles, arrows, Esc, focus back on the icon), the
  pending bar and the preview, write and undo on a fake Rust side.
- **Backup** (`agents_backup.rs`) makes packs in a temporary iCloud Drive folder
  from the same kind of temporary home, and restores them on another one. They
  check that no pack holds a trap key and that a pack that would is refused;
  that a restore asks for the keys, adds a server short of one turned off and
  changes paths to the new home; that servers already the same are left alone
  and one that differs is not ticked; that a file changed after the preview
  stops the restore; that the undo takes the whole restore back, asking when a
  file changed since; a pack not downloaded yet; the 10 packs kept; and that a
  restore leaves Codex's provider and the logins as they were. The Vitest tests
  walk the tab and the sheet on a fake Rust side and check that a pasted key
  never stays in the page.
- **Restore of skills, plugins and instruction files** (`agents_restore.rs`)
  packs them in one temporary home and restores them on another, with a fake
  `npx` and fake `claude` and `codex` that record their arguments. They check
  that the pack copies skills installed by hand and not those with a source,
  the exact commands a restore and its undo run, that a file that differs keeps
  the Mac's own unless ticked, that one part failing leaves the others to run
  and `Try again` gets it done, that the undo takes it all back, and that no
  trap key reaches the pack, the plan, the result, the record or the app's
  backups.
- **Copy to another Mac** (`agents_copy.rs`) copies between two temporary homes
  through a fake `ssh` that records its arguments and stdin and runs the script
  on the other home, where a fake app bundle answers with canned replies, with
  or without `QuotaStationAgents` in its `Info.plist`. They check that the script
  is the same fixed text for every call, with `BatchMode=yes` and the 20-second
  limit; that a Mac on version 1 or without the app is never run; that the
  comparison and a copy without keys hold no trap key, and that with them keys
  go on stdin only and land in the fake Keychain; that a copy is the restore's
  checked write with its undo; that a file changed since the comparison is not
  written; and that bad names, paths and protocols are refused. No test opens a
  real SSH connection. The Vitest tests walk the tab and its dialogs on a fake
  Rust side.
- **Variables that override a login** (`env_check.rs`) check fake home folders
  with fake shell files and a fake `launchctl` that records its words: a name
  counts only at the start of a line, the last line wins, `launchctl` is asked
  `getenv NAME` for the eleven names and nothing else, a file it cannot read is
  said, and a shell file with commands in it is never run. Their trap plants a
  value in every file and in `launchctl`'s answer and finds it nowhere: not in
  the result, the alerts, the app's folder or what the window is sent. That
  model is `src/fixtures/env-trap.json`, which the Vitest tests draw on Settings,
  the Claude tab and the notifications panel, again finding no value.
- **Config sets** (`agents_sets.rs`) apply sets on a fake home: Codex's
  `config.toml` with a custom provider, Antigravity's `mcp_config.json`, Claude
  Code's list kept by a fake `claude` that records its arguments, a shared
  folder of skills with the agents' links and a fake `npx` that records its
  arguments (a set never runs it). They check that the preview writes nothing;
  that applying changes only the set's entries; that Codex's provider never
  changes and the side it was on is kept for the undo; that a file or skills
  folder changed since the preview stops the whole set; that a change the set
  does not make is refused; the drift after a change by hand; and the undo of a
  whole set, also after the Codex switch changed the provider line and with
  entries changed or removed since. Their trap keys are in no preview, result,
  drift, undo, sets file or other file of the app's.
- **Stats** (`stats.rs`) reads synthetic session logs (`tests/fixtures/stats/`)
  of Claude Code 1.0 and 2.0, of Codex before and after its token events, and of
  versions whose format the app does not know, in a fake home. They check each
  count; that a response written over several lines, or copied into a resumed
  session, counts once; that a count Codex repeats adds nothing; that unknown
  lines are skipped and counted per session, with the version that wrote them;
  that a later read takes only what was appended and leaves a line still being
  written for the next; that a truncated, replaced, deleted or archived log is
  counted right; the index across a restart and its 62 days; the days and hours
  on the Mac's clock, the Claude accounts from the switch history, and the
  prices. The trap: the conversation in every fixture carries one text, found in
  no index, report or file of the app's; the report is
  `src/fixtures/stats-trap.json`, which the Vitest tests draw with its table and
  hover cards, finding no such text.

The report fixtures in `crates/quota-station-core/tests/fixtures/` are
synthetic, with made-up ids and values. The popover and window tests use JSON
snapshots in `src/fixtures/snapshots/`, which the Rust side builds from those
fixtures. After changing the snapshot rules, the variable check or the stats
report, regenerate them:

```sh
UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test ui_fixtures
UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test env_check
UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test stats
```

## Layout

- `crates/quota-station-core/` is plain Rust, with no window. It reads the
  quota (`quota/`, ported from quota-axi), turns the readings into a snapshot,
  holds the icon rules and glyph (`icon.rs`), and schedules reads. Its main
  modules:
  - `claude/`: the Claude accounts: the Keychain store (and `credman.rs`, the
    Windows Credential Manager), Claude Code's locks and login, the switch, the
    usage reads, the rotation and its LaunchAgent (`timer.rs` on Windows), the
    session folders, the import from claude-swap and the `quota-station claude`
    command line.
  - Pausing, sessions and the switcher: it counts live sessions, reads and
    writes the pause flag, reads the rotation's dry run and log, reads, checks
    and writes the rotation file, and copies it over SSH (`peers.rs`).
  - `antigravity/`: logs Antigravity accounts in and out, reads their quota,
    paces those reads, and switches the account Antigravity.app and agy use.
  - `history`, `alerts` and `firstmate`: the switch history, the notification
    rules and Firstmate's `quota-pool.last`.
  - `update` pulls the clone and runs its install script; `login_item` holds
    the rule of the `Open at login` switch; `release.rs` checks GitHub
    Releases; `cli_tools.rs` checks and updates the agents' command-line tools;
    `settings.rs` is the settings file.
  - `codex/`: Codex's provider and accounts: the file and its edit, the login's
    shape, the safe write and backups, the process table and the restart, the
    reachability check, the resumable sessions, the switch log, the accounts,
    the sign-in, the import and the wake runs.
  - `agents/`: the MCP tab (each agent's list, the edits of one entry, the
    batch, the safe write it shares with `codex`, the backups without keys, the
    app's store and its Keychain items); `pack.rs` makes, lists and reads the
    backup packs and `restore.rs` plans, runs and undoes a restore; `copy.rs`
    compares two Macs and copies between them over SSH; `skills.rs` reads and
    writes the Skills tab's links and `[[skills.config]]` entries, `skill_ops.rs`
    and `npx.rs` install, update and remove through `npx skills`,
    `skill_updates.rs` asks GitHub for newer versions, `plugins.rs` reads the
    agents' plugin lists and `instructions.rs` the instruction files;
    `env_check.rs` finds the variables that override a login; `sets.rs` turns a
    configuration set into the MCP and Skills tabs' batches, applies and undoes
    it, and says how far the Mac drifted from it.
  - `stats/`: reads the session logs for their token counts (`claude_log.rs`,
    `codex_log.rs`), names their projects (`project.rs`), keeps the index of how
    far each was read and the counts (`index.rs`), adds them up for a period
    (`report.rs`) and holds the one price table (`prices.rs`).
  - The system behind small seams: `os.rs` and `win.rs`, `sealed.rs` (the app's
    secrets on Windows, sealed by DPAPI), `app_dir.rs`, `local_time.rs` and
    `detail.rs`.
- `src-tauri/` is the Tauri 2 shell: the tray item and its menu (`tray.rs`),
  the popover and the main window, the read loop (`reader.rs`), the Antigravity
  quota and switch (`agy_quota.rs`, `agy_switch.rs`, `antigravity.rs`), the
  macOS notification center (`notify.rs`, `alerting.rs`), the login item
  (`login_item.rs`), the update from source (`install.rs`), the release check
  (`release.rs`), the Codex thread, accounts, import and wake (`codex.rs`,
  `codex_accounts.rs`, `codex_import.rs`, `codex_wake.rs`), the MCP tab's
  commands (`agents.rs`), the backup tab's (`backup.rs`), the Skills, Plugins
  and Instruction files tabs' (`skills.rs`), the copy tab's and the
  `quota-station agents …` commands another Mac runs (`copy.rs`), the Claude
  accounts' commands and the `quota-station claude …` dispatch (`claude.rs`,
  `main.rs`), the variables check (`env.rs`), Config sets (`sets.rs`), Stats
  (`stats.rs`), Agent updates (`cli_tools.rs`), the rotation rules
  (`rules.rs`), the state and commands (`state.rs`, `commands.rs`, `open.rs`),
  and on Windows the tray, windows and the rotation timer (`windows.rs`,
  `winsys.rs`, `timer.rs`) and the test build's guided session and diagnostics
  zip (`session.rs`, `diag.rs`, `shot.rs`).
- `scripts/install.sh` builds, signs, installs and packages the app.
- `src/` is the React and TypeScript frontend: `popover/`, `window/`,
  `claude/`, `antigravity/` (and `antigravity-off/`, which stands in when the
  Antigravity accounts are left out), `codex/`, `agents/` (MCP and its
  `CheckMatrix`, which the Skills tab reuses, Plugins, Instruction files,
  Config sets and Agent updates), `backup/` (the backup tab, the restore sheet
  and the copy tab), `settings/`, `stats/` (the Stats tab and its SVG chart),
  `rules/`, `history/`, `alerts/`, `firstmate/`, `release/`, `setup/` (the first
  run), `session/` (the Windows test build's guide), `lib/` (the model, the
  view functions and `i18n.ts`), `components/` and `styles/`. `preview.ts` is
  the browser preview.

## Front-end stack

The frontend is React 19 and TypeScript, built by Vite. It is styled with
Tailwind CSS v4 (through `@tailwindcss/vite`, with no config file) and its
interactive parts are shadcn/ui components on Radix primitives. Every string
the windows show is `t('tiếng Việt', 'English')` (`src/lib/i18n.ts`), the two
side by side; `docs/ui-guidelines.md` has the approved wording.

- `src/styles/tokens.css` holds every colour, size, radius, shadow and blur of
  the approved prototype, for light and dark and for Reduce transparency, then
  one block per theme pack (`data-pack`) and the stronger edges of Increase
  Contrast (`data-contrast`). It is the one place to change a colour: nothing
  else names one. `src/styles/tokens.test.ts` checks every pack's text against
  WCAG AA and its quota bars apart, in light and dark. The packs are described
  in [themes.md](themes.md).
- `src/styles/theme.css` maps those tokens to Tailwind's theme (`@theme
  inline`: `text-ink2`, `bg-fill`, `rounded-tile`, `shadow-sheet` and the rest)
  and to shadcn/ui's variables. Tailwind's own colours, text sizes, weights,
  shadows and blurs are cleared, so only the prototype's exist. `dark:` follows
  the window's `data-theme`, not the system setting alone.
- `src/styles/glass.css` draws the materials: the popover's and the main
  window's glass (`material-pop`, `material-window`), the frosted panels and the
  solid fills Reduce transparency asks for.
- `src/styles/logos.css` styles the services' logos.
- `src/styles/globals.css` imports Tailwind (without its reset, which the
  prototype never had) and the four files above; `src/main.tsx` imports only
  it.
- `src/components/ui/` holds the shadcn/ui components, customised to the macOS
  controls: `button`, `badge`, `card`, `checkbox` (the macOS box), `dialog` (the
  sheets and dialogs), `dropdown-menu` (a column's menu), `input`, `popover`,
  `radio-group` and `switch`, with `segmented` (the app's own, on Radix Toggle),
  and `svgs/` (service logos from svgl). They keep the ARIA roles, keyboard
  paths and focus returns the tests check. `components.json` records their
  setup for the shadcn CLI.
- `src/components/parts.tsx`, `page.tsx` and `blocks.tsx` build the app's own
  pieces from them: a tab's head, its panels and rows, a diff, the progress of a
  write. `src/lib/utils.ts` has `cn`, which merges class names with
  tailwind-merge set to the theme's names.

A few class names stay on elements that tests or other selectors look for
(`wt-r`, `hrow`, `pendbar`, `toast` and the like); they carry no style. A test
reads a control's state from its ARIA attribute (`aria-checked`,
`aria-pressed`) or its `data-tone`, `data-variant` or `data-op`, not from its
classes.

## Milestones

The app was built in milestones, which the code's comments and the git history
still name: M1 `Khung và số` (the frame and the numbers), M2 `Cửa sổ chính chỉ
đọc` (a read-only main window), M3 `Chuyển Claude và tạm dừng` (switching Claude
and pausing), M4a `Đăng nhập Antigravity trong app`, M4b `App tự đọc hạn mức
Antigravity` and M4c `Chuyển Antigravity không cần Cockpit` (taking over what
Cockpit Tools did), M5 `Luật xoay vòng` (the rotation rules), M6 `Lịch sử, cảnh
báo và Firstmate`, M7 `Chuyển đổi` (installing, updating and launching at
login), M8 `Chuyển Codex`, M9 `MCP`, M10 `Sao lưu và khôi phục` (the MCP
servers), M11 `Skills, Plugin, File hướng dẫn`, M12 the rest of backup and
restore (skills, plugins and instruction files), M13 `Chép sang Mac khác`, M14
`Biến môi trường đè đăng nhập`, M15 `Bộ cấu hình` and M16 `Thống kê`. Later work
(the bilingual community edition, the first run, the Codex accounts, Agent
updates, the theme packs and the Windows plans W1–W5) is named by its pull
request.
