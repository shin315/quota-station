# Quota Station

A macOS menu bar app that shows how much Claude Code and Antigravity quota
is left on this Mac, as reported by [quota-axi](https://github.com/kunchenguid/quota-axi).
The interface is in Vietnamese.

**Personal, non-commercial project (cá nhân, phi thương mại).** It is shared
under CC BY-NC-SA 4.0. See [License](#license).

<img src="docs/popover.png" width="380" alt="The popover on fixture data: two pool tiles, four Claude Code accounts and two Antigravity accounts with their allowances and reset times">

![The menu bar item for four fixture cases, in light and dark menu bars](docs/menubar.png)

Both images use the synthetic test fixtures, not real accounts. The menu bar
image is drawn from the app's own glyph code, not captured from the screen.

## What it does

This is milestone M1, "Khung và số". The app only reads.

- **Menu bar item.** A template glyph with one bar for Claude and one for
  Antigravity. Next to it is the whole-rotation number. When there is a
  rotation this is the room left, counted in accounts: `2,3` means about 2.3
  accounts' worth. With no rotation it is the active account's percent, for
  example `57%`. Before the first read it shows `–`. The tooltip and the
  VoiceOver label say the same in words.
- **Popover** (left click). The two pools, then every Claude Code and
  Antigravity account with its allowances (`5 giờ`, `Tuần`, `Fable tuần`,
  `Gemini`, `Claude/GPT`) and when each one resets. It also has banners for
  stale numbers, an unavailable source, numbers waiting after a reset, an
  exhausted rotation, missing Keychain permission and a failing quota-axi. It
  has no switch buttons. The only button is the gear, which opens settings.
- **Short menu** (right click). `Làm mới` (refresh), `Cài đặt…` (settings),
  `Thoát Quota Station` (quit).
- **Settings window.** The quota-axi path, either found automatically or set
  by hand, and a one-time button that lets quota-axi ask macOS for Keychain
  access.

Each Mac runs its own copy and reads only its own machine. Switching
accounts, the main window, notifications, history and installing as a login
item come in later milestones.

### What it reads and runs

- `quota-axi --provider claude --json --max-age 120s --no-credential-refresh`,
  and the same for `--provider agy`. The app reads every 60 seconds while the
  popover is open and every 5 minutes while it is hidden. It also reads when
  the popover opens, after the Mac wakes and on `Làm mới`.
- The Keychain button runs
  `quota-axi --provider claude --json --allow-keychain-prompt --no-credential-refresh`
  once. macOS shows its own prompt. The app itself never opens the Keychain
  or any credential file.
- It ignores the identity fields in quota-axi's output (emails, ids). Accounts
  are shown by role and slot number.
- It writes only its own settings file,
  `~/Library/Application Support/com.shin315.quota-station/settings.json`.

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

`QUOTA_STATION_LOG=1` prints one line per read to stderr: the title, the tone
and each slot's percentages. No names are printed.

### Browser preview

`npm run dev` serves the popover on fixture data at http://localhost:1420/.
These query parameters choose what it shows:

- `state`: one of `fresh` (the default), `no-cockpit`, `keychain`, `stale`,
  `reset-pending`, `exhausted`, `no-rotation`, `unavailable`,
  `no-antigravity` or `agy-pool`.
- `view=settings` shows the settings window.
- `error`: one of `notConfigured`, `notFound`, `spawn`, `timeout`, `exit` or
  `parse`.
- `theme`: `light` or `dark`.

## Tests

```sh
cargo test --workspace   # parsing, snapshot rules, icon, runner, schedule
npm test                 # Vitest: popover and settings views
```

The quota-axi fixtures in `crates/quota-station-core/tests/fixtures/` are
synthetic, with made-up ids and values. The popover tests use JSON snapshots
in `src/fixtures/snapshots/`, which the Rust side builds from those fixtures.
After changing the snapshot rules, regenerate them with this command:

```sh
UPDATE_UI_FIXTURES=1 cargo test -p quota-station-core --test ui_fixtures
```

## Layout

- `crates/quota-station-core/` is plain Rust. It runs and parses quota-axi,
  turns its output into a snapshot, holds the icon rules and glyph, and
  schedules reads.
- `src-tauri/` is the Tauri 2 shell: the tray item, the popover and settings
  windows, the read loop and the commands.
- `src/` is the React and TypeScript frontend. Its `styles/tokens.css` and
  `styles/components.css` come from the approved prototype.

## License

Copyright © 2026 Shin315.

This work is licensed under the
[Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-nc-sa/4.0/).
The full text is in [`LICENSE`](LICENSE). The tools it talks to are listed in
[`THIRD_PARTY.md`](THIRD_PARTY.md).
