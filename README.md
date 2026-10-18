# Quota Station

A macOS menu bar app that shows how much Claude Code and Antigravity quota
is left on this Mac, as reported by [quota-axi](https://github.com/kunchenguid/quota-axi).
The interface is in Vietnamese.

**Personal, non-commercial project (cá nhân, phi thương mại).** It is shared
under CC BY-NC-SA 4.0. See [License](#license).

<img src="docs/popover.png" width="380" alt="The popover on fixture data: two pool tiles, four Claude Code accounts and two Antigravity accounts with their allowances and reset times">

![The menu bar item for four fixture cases, in light and dark menu bars](docs/menubar.png)

<img src="docs/window.png" width="760" alt="The main window's Tổng quan tab on fixture data: the two pools, the Claude Code and Antigravity accounts in tables, and the next resets">

All three images use the synthetic test fixtures, not real accounts. The menu
bar image is drawn from the app's own glyph code, not captured from the screen.
The popover and window images date from M2, before the switch buttons.

## What it does

This is milestone M5, "Luật xoay vòng", on top of M3 "Chuyển Claude và tạm
dừng", M2 "Cửa sổ chính chỉ đọc" and M1 "Khung và số". The app reads, and
acts in three ways, each only when the owner asks: it switches the active
Claude account through claude-swap, it pauses the automatic switcher, and it
edits the switcher's rotation rules and copies them to the owner's other Macs.

- **Menu bar item.** A template glyph with one bar for Claude and one for
  Antigravity. Next to it is the whole-rotation number. When there is a
  rotation this is the room left, counted in accounts: `2,3` means about 2.3
  accounts' worth. With no rotation it is the active account's percent, for
  example `57%`. Before the first read it shows `–`. The tooltip and the
  VoiceOver label say the same in words. While automatic switching is paused,
  two small pause bars follow the number.
- **Popover** (left click). The two pools, then every Claude Code and
  Antigravity account with its allowances (`5 giờ`, `Tuần`, `Fable tuần`,
  `Gemini`, `Claude/GPT`) and when each one resets. It also has banners for
  stale numbers, an unavailable source, numbers waiting after a reset, an
  exhausted rotation, missing Keychain permission and a failing quota-axi.
  Every Claude account other than the one in use is a button that opens the
  switch sheet (below). Antigravity rows do not switch yet. The footer has the
  `Tự chuyển Claude` switch and the line that says why Claude is on its
  account, and, when other Macs are set up, which of them have the rotation
  rules (`Mac B: 14:30 · Mac C: chưa tới được, sẽ thử lại`). Its header has
  two buttons: the gear opens the settings tab and the window button opens
  the main window.
- **Switching Claude** (F04). The confirmation sheet names the account, its
  allowances, and how many Claude Code sessions running on this Mac will
  rebuild their prompt cache on their next message, which costs quota. It
  offers a hold so the switcher does not switch straight back: 30 phút,
  1 giờ (the default), until the account's next five-hour reset, or none.
  Confirming runs `cswap switch N --json`, writes the pause flag when a hold
  was chosen (a hold never shortens a pause that is already on for longer),
  waits 5 seconds, reads the active account again without quota-axi's cache
  and reports what it finds: switched, switched back by the switcher (or by
  another switch), not yet confirmed, or claude-swap's error. When `cswap`
  times out or stops without an answer, the app reads the account in use
  before saying whether the switch landed. Outside the personal account's
  hours the sheet warns that the switcher will move away from it.
- **Pausing automatic switching** (F07). The switch opens a sheet to pause
  for 1 giờ, until 18:00 or until turned back on; while paused it resumes at
  once, and the banner offers `Thêm 1 giờ`. The why line shows the
  switcher's own decision from `--dry-run` and its last log line, or
  `Đang tạm dừng tới HH:MM: <lý do>` while the pause holds. Banners warn
  when claude-swap's own automatic switcher (`com.cswap.auto`) is loaded
  again, and when the switcher ignores the pause flag.
- **Short menu** (right click). `Mở cửa sổ chính` (main window), `Làm mới`
  (refresh), `Cài đặt…` (settings), `Thoát Quota Station` (quit).
- **Main window.** The same numbers as the popover, from the same read, in
  four tabs:
  - `Tổng quan`: both pools with how the rotation number adds up, every
    Claude Code and Antigravity account in a table with each allowance and
    its reset in words, `Vì sao đang ở …` with the `Tự chuyển Claude`
    switch, and `Sắp tới`, the resets and rotation ends of the next 24 hours
    as the readings give them.
  - `Claude`: the accounts with a `Cập nhật` column, the age of each
    account's own reading, and a `Chuyển` button on each account not in
    use. Below it, `Tự chuyển` (the switch, quick pause choices, the
    switcher's last decision and log line, each with its raw line) and
    `Switcher` (whether launchd runs it, its last log time, whether
    `com.cswap.auto` is loaded, the pause, and whether the switcher reads
    the pause flag). A switcher that launchd does not run gets a banner.
  - `Antigravity`: the accounts, how old the numbers of the accounts not in
    use are, and whether Cockpit answers. A Mac without Cockpit or without
    Antigravity says so.
  - `Luật xoay vòng`: the rotation editor (below).
  - `Cài đặt`: the quota-axi path, either found automatically or set by hand,
    a one-time button that lets quota-axi ask macOS for Keychain access, the
    name shown for this Mac (empty uses the name macOS gives it), and
    `Các Mac khác`: a name and an SSH alias for each other Mac, with
    `Thêm Mac`, `Bỏ` and `Thử SSH` (reads that Mac's rotation file and
    changes nothing). A settings file from before M5 with a single
    `sshAlias` moves into this list.

  The sidebar also lists `Codex`, `Lịch sử`, `Cảnh báo` and `Firstmate`,
  greyed out: they come in later milestones. Under the machine name it says
  whether the other Macs have the rules.
- **Rotation rules** (F06). The editor shows the rotation file's six keys:
  the Team accounts in the rotation (chips; the number is the order), the
  personal account and its hours, the two thresholds (sliders), and when an
  account leaves the rotation. It marks what changed, previews what the
  switcher would decide with the edited rules (its `--dry-run` pointed at a
  draft copy), and saves with `Lưu cho cả ba Mac`. A save rewrites the six
  keys in place and keeps every other line, puts a
  `# updated = <time> by <machine>` comment at the top, writes through a
  temporary file and a rename, and keeps the file before as
  `rotation.bak-<time>` (the newest 10 stay). quota-axi then reads the file;
  when it refuses it (a `rotation_…` reason) or cannot run, the file before
  goes back and nothing is sent. Then the file goes to each other Mac over
  SSH, the same way, and is read back to compare. The save bar has one line
  per Mac: `đã nhận 14:30`, `chưa tới được · thử lại 14:35`, `có bản mới hơn`.
  A Mac that could not take it is tried again every 5 minutes, also after a
  restart. When another Mac holds a newer copy (a later `updated`), or one
  edited by hand there, nothing is overwritten: the tab offers that copy,
  side by side with this Mac's, or keeping this Mac's (stamped again, so it
  is the newest). When a newer copy lands on this Mac during an edit, the
  editor says `Bản trên <máy> mới hơn, tải lại?` and saves over it only when
  told to. Leaving the tab or closing the window with unsaved changes asks
  first.

Each Mac runs its own copy and switches only its own machine; there is no
switch button for another Mac. The only thing sent to another Mac is the
rotation file. Switching Antigravity, notifications, history and installing
as a login item come in later milestones.

### What it reads and runs

- `quota-axi --provider claude --json --max-age 120s --no-credential-refresh`,
  and the same for `--provider agy`. The app reads every 60 seconds while the
  popover or the main window is open and every 5 minutes while both are
  hidden. It also reads when either opens, after the Mac wakes and on
  `Làm mới`. The check after a switch passes `--max-age 0` instead: a cached
  Claude reading may still be the account from before the switch.
- The Keychain button runs
  `quota-axi --provider claude --json --allow-keychain-prompt --no-credential-refresh`
  once. macOS shows its own prompt. The app itself never opens the Keychain
  or any credential file.
- It ignores the identity fields in quota-axi's output (emails, ids). Accounts
  are shown by role and slot number.
- Switching runs `cswap switch N --json` (claude-swap holds Claude Code's
  credential locks while it writes) and nothing else of claude-swap's. The
  app reads only the account numbers from its answer, takes email addresses
  out of its error messages, and looks for `cswap` in `~/.local/bin`,
  `/opt/homebrew/bin` and `/usr/local/bin`.
- Live sessions are counted from `~/.claude/sessions/*.json`: only the `pid`
  and `kind` fields are read, and a session counts when its process is alive.
- The switcher: `~/.local/bin/cswap-prefer-team --dry-run`, which switches
  nothing, the last line of `~/Library/Logs/cswap-prefer-team.log`, and
  `launchctl list` for `com.cswap.prefer-team` and `com.cswap.auto`. These
  run with each read while the popover or the main window is open, and after
  a switch or a pause; while both are hidden only the pause flag is read. The
  app never runs the switcher for real.
- The rotation editor's preview runs the same `--dry-run` with
  `CSWAP_ROTATION_FILE` pointing at a draft copy in the app's folder, which
  it deletes afterwards.
- Copying the rotation file runs `ssh -o BatchMode=yes -o ConnectTimeout=5
  -T <alias>` with one of two fixed scripts: one prints
  `~/.config/cswap-prefer-team/rotation`, the other takes the new file on
  stdin, keeps a backup, renames it into place and prints it back. Batch
  mode means SSH never asks for a password or a host key. An alias must be
  letters, digits and `. - _ @`, and cannot start with `-`. The `ssh` used
  is the first on `PATH`, else `/usr/bin/ssh`.
- It writes these files: its own settings file,
  `~/Library/Application Support/com.shin315.quota-station/settings.json`,
  with `rotation-sync.json` (where each other Mac stands) beside it; the
  switcher's pause flag, `~/.config/cswap-prefer-team/pause`
  (`until = YYYY-MM-DDTHH:MM` in local time and `reason = …`; no `until`
  pauses until resumed); and the rotation file with its backups. Each is
  written through a temporary file and a rename. Resuming deletes the flag.

### The pause flag and the switcher

The switcher decides on its own every three minutes; the app only asks it to
wait. That needs a guard at the top of the switcher: while the flag holds,
exit; once `until` has passed, delete the flag and go on, and print
`paused until <until>: <reason>` (or `paused indefinitely: <reason>`) on
`--dry-run`. The switcher script is not part of this repository. The app
compares `until` with the local time as text, the same rule as the guard, so
both agree on when a pause ends. When a dry run made while the flag holds
still prints a normal decision, the app reports that the switcher ignores the
flag and does not show Claude as paused.

### How the numbers relate to `quota-axi --tui`

- **5 giờ / Tuần.** For the active account, this is the tighter of the live
  five-hour and seven-day windows. A tie shows the week. For the other
  accounts it is claude-swap's effective percent, labelled by the window that
  limits it.
- **Fable tuần.** For the active account, this is the live Fable weekly
  window. For the other accounts, quota-axi's JSON gives claude-swap's Fable
  scope. When another window limits that account first, the scope is only a
  lower bound, so the app shows it as `≥98%` with no reset time.

## Requirements

- macOS 13 or later
- Rust stable (1.85 or later) and Node.js 20 or later (CI uses 24)
- quota-axi on the Mac, with JSON schema 5 or 6. The claude-swap, rotation and
  Cockpit sidecars come from the `+cswap` fork. Without them the app shows
  only the account in use.
- For switching, claude-swap (`cswap`) with `switch --json`. For pausing, the
  `cswap-prefer-team` switcher with the pause guard described above.
- For the other Macs, an SSH alias to each that logs in without a prompt
  (a key, `BatchMode=yes`).

The app looks for quota-axi in these places: `~/.local/bin`, nvm's Node
versions (newest first), `/opt/homebrew/bin` and `/usr/local/bin`. You can
also set the path in settings.

## Build and run

```sh
npm install
npm run app            # dev: Vite and the Tauri app, with reload
```

To make a release `.app`, run these commands:

```sh
cargo install tauri-cli --version '^2' --locked   # once
cargo tauri build
# or, with the CLI from npm:
npm run app:build
```

The bundle is written to `target/release/bundle/macos/Quota Station.app`. It
is ad-hoc signed and is not installed anywhere. Open it from there to try it.

### Browser preview

`npm run dev` serves the popover at http://localhost:1420/ on the fixtures and
the fake Rust side the tests use. Query parameters: `state` (a file name in
`src/fixtures/snapshots/`), `view=window` with `tab` (`overview`, `claude`,
`agy`, `rules` or `settings`), `error` (an error kind) and `theme` (`light` or `dark`).
For the rotation rules: `rotation` (`unreachable`, `newer`, `edited`,
`missing`, `invalid`, `refused` or `single`).
For switching: `switcher` (`held`, `noguard`, `auto`, `stopped` or
`missing`), `outcome` (`raced`, `failed`, `stopped` for a cswap timeout, or
`unconfirmed`; a switch lands by default) and `cswap=none`. Nothing is switched and no file is written: a
switch only waits 1.5 seconds and a save 1 second, and both update the fixture.

## Tests

```sh
cargo test --workspace   # parsing, snapshot rules, icon, runner, schedule, switching, rotation, other Macs
npm test                 # Vitest: popover and main window views, switch and pause sheets, rules editor
```

The switching tests never touch a real account. They run a fake `cswap` (a
shell script in a temporary HOME) that switches, fails with an error, or is
switched back by a fake switcher writing its log; a fake quota-axi answers
the read after the switch. Session counting runs on session files in a
temporary folder, and the pause flag tests write it there with an `until`
ahead, passed, malformed or missing.

The rotation tests write rotation files in temporary folders and check them
with a fake quota-axi that prints a fixture: one that accepts, one that
refuses a key (the backup comes back), and one without the rotation block.
The other-Mac tests put a fake `ssh` first on `PATH`; it runs the fixed
scripts in a temporary home per alias, and has an alias that cannot be
reached and one that reads back something else. They cover a Mac with no
file, an older copy (replaced, backup kept, ten newest kept), the copy
already there, the 5-minute retry, a newer copy that wins, and a copy edited
by hand.

The quota-axi fixtures in `crates/quota-station-core/tests/fixtures/` are
synthetic, with made-up ids and values. The popover and window tests use JSON snapshots
in `src/fixtures/snapshots/`, which the Rust side builds from those fixtures.
After changing the snapshot rules, regenerate them with this command:

```sh
UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test ui_fixtures
```

## Layout

- `crates/quota-station-core/` is plain Rust. It runs and parses quota-axi,
  turns its output into a snapshot, holds the icon rules and glyph, and
  schedules reads. It also runs `cswap switch`, counts live sessions, reads
  and writes the pause flag, and reads the switcher's dry run and log. It
  reads, checks and writes the rotation file, and copies it over SSH.
- `src-tauri/` is the Tauri 2 shell: the tray item, the popover and the main
  window, the read loop and the commands.
- `src/` is the React and TypeScript frontend. Its `styles/tokens.css` and
  `styles/components.css` come from the approved prototype.

## License

Copyright © 2026 Shin315.

This work is licensed under the
[Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-nc-sa/4.0/).
The full text is in [`LICENSE`](LICENSE). The tools it talks to are listed in
[`THIRD_PARTY.md`](THIRD_PARTY.md).
