# How Quota Station works

This page is the technical side of the [README](../README.md): how the app
reads the quota, keeps and switches accounts, what it runs, and every file it
writes. [`SECURITY.md`](../SECURITY.md) lists what the app never reads, and how
keys and tokens are kept. UI labels are given in English, with the Vietnamese
label where it helps to find it.

- [The quota readers](#the-quota-readers)
- [The Claude accounts](#the-claude-accounts)
- [Antigravity logins, quota and switching](#antigravity-logins-quota-and-switching)
- [The OAuth client file](#the-oauth-client-file)
<!-- @private:begin -->
- [The pause flag and the switcher](#the-pause-flag-and-the-switcher)
<!-- @private:end -->
- [How the numbers relate to quota-axi](#how-the-numbers-relate-to-quota-axi)
- [What it reads and runs](#what-it-reads-and-runs)
- [The files it writes](#the-files-it-writes)

Each Mac runs its own copy and switches only its own machine; there is no
switch button for another Mac. Each Mac keeps its own Antigravity logins, and
nothing assumes another Mac has any. Each Mac installs and updates its own
copy.

<!-- @private:begin -->

With the `author-tools` feature, the only things sent to another Mac are the
rotation file and, when the owner asks, the items of `Copy to another Mac`
(`Chép sang Mac khác`).

<!-- @private:end -->

## The quota readers

The app reads the quota itself, with readers ported from
[quota-axi](https://github.com/kunchenguid/quota-axi) (MIT), in
`crates/quota-station-core/src/quota/`. None of them refreshes, rotates or
writes a login: an expired login shows as stale or as a sign-in to do, and the
vendor's own tool renews it when it next runs.

- **Codex**, on a Mac with a Codex folder: the access token and account id
  from `~/.codex/auth.json` (read only), then
  `GET https://chatgpt.com/backend-api/wham/usage`, or
  `https://chatgpt.com/backend-api/codex/usage` when that one does not answer,
  for the ChatGPT login's five-hour and weekly windows. Codex's `app-server` is
  never started for a read.
- **Antigravity**: `agy -p /quota --output-format json` (the first `agy` on the
  app's `PATH`, else in `/opt/homebrew/bin` or `/usr/local/bin`, as quota-axi
  looked for it), with stand-ins for `open` and `xdg-open` first on its `PATH`
  so it cannot open a browser; when that gives no numbers, the language server
  of a running agy or Antigravity.app, found with `ps` and `lsof`, asked on
  127.0.0.1 for its quota summary with the CSRF token from its command line.
  The account's email is compared by a hash and never kept. An app opened from
  Finder has launchd's short `PATH`; an `agy` elsewhere is then read through
  the running agy or Antigravity.app.
- **Claude Code**, on a Mac where the app keeps no Claude account: its login
  from the Keychain item `Claude Code-credentials` (read with
  `/usr/bin/security find-generic-password` once the owner allowed it,
  otherwise only checked for), then `~/.claude/.credentials.json`, and
  `GET https://api.anthropic.com/api/oauth/usage`. Where the app keeps the
  Claude accounts, it reads them itself ([below](#the-claude-accounts)).

The app reads every 60 seconds while the popover or the main window is open
and every 5 minutes while both are hidden. It also reads when either opens,
after the Mac wakes and on `Refresh` (`Làm mới`). A reading younger than two
minutes for the same login is used without asking again
(`quota/<provider>.json` in the app's folder: windows, times and a hash of the
login only), and a read that fails shows the last numbers as stale. The check
after a switch asks again at once: a cached Claude reading may still be the
account from before the switch.

## The Claude accounts

Quota Station keeps the Claude accounts itself, the way claude-swap kept them,
so no other app is needed to switch or read them
([`claude-accounts.md`](claude-accounts.md) has the design; the logic that
touches Claude Code's login is adapted from claude-swap, MIT, credited in
[`THIRD_PARTY.md`](../THIRD_PARTY.md)).

- **Store.** Each account's login and its `oauthAccount` are in the login
  Keychain under the app's own service, `com.shin315.quota-station.claude`
  (`slot-N` and `slot-N.account`). `claude/accounts.json` in the app's folder
  holds the slot numbers, nicknames and the account ids; no email and no token
  is in a file.
- **Adding.** `Add account` (`Thêm tài khoản`) in the `Claude` tab opens a sheet
  with `Sign in with the browser` (`claude auth login` in a folder of its own,
  `claude/adding/`, so Claude Code's login does not change), taking the account
  Claude Code is signed in to, `Paste a setup-token` (a `claude setup-token`
  token, kept as a login without refresh; the field is a password field and the
  token is never shown again) and `Import from claude-swap` (once per Mac;
  again later only takes a newer login). Each row's `⋯` menu has `Details and
  nickname…`, `Sign in again…` and `Remove from this Mac…` (not the one in
  use).
- **Switch.** Under Claude Code's own lock folders, the app keeps the live login
  of the account in use, writes the target's login to Claude Code's Keychain
  item and its `oauthAccount` to `~/.claude.json`, and reads both back, putting
  them back when they do not match. A login the app does not keep stops the
  switch, so it is never lost. A switch never refreshes a token. It drops
  nothing from MCP or plugin secrets in the live login.
- **Usage.** Each account's five-hour, weekly and Fable weekly windows come from
  Anthropic's usage endpoint, at most every 180 seconds an account, with a
  back-off after a failure or a 429. The account in use is read with Claude
  Code's own token and never refreshed by the app; another account whose token
  has under five minutes left is refreshed first
  (`POST https://platform.claude.com/v1/oauth/token`). The readings are
  `claude/usage.json`, and the Claude rows are built from them. Its email and
  organization come from the app's Keychain items, stay in memory, go to the
  windows only while `Show emails` is on, and are never written to a file or a
  log.
<!-- @private:begin -->
- **With the `author-tools` feature.** In `Details and nickname…`, a switch
  lets automatic switching use the account, and the `⋯` menu can pause
  automatic switching for it (it is never switched to, but kept by the usual
  rules while in use, and can still be switched to by hand). While
  `Settings › Advanced` shows Firstmate, the app also writes a copy in
  claude-swap's layout, `claude/swap/`, with placeholder addresses, for
  Firstmate's quota-axi (`QUOTA_AXI_CLAUDE_SWAP_DIR` and
  `QUOTA_AXI_CLAUDE_ROTATION`, see [`claude-accounts.md`](claude-accounts.md)).
- **Rotation.** The `cswap-prefer-team` rules, ported line for line, on the
  app's own copy of the rotation file (`claude/rotation`; the rules editor and
  the copy to the other Macs work on it), with one change: the early switch is
  on. While the Team account in use has room, the rotation moves to another
  Team account whose five-hour window has started and ends sooner, when that
  account has room, at least 20 minutes of that window left and at least 10
  points of it before the threshold (and Fable left if the account in use has
  Fable); of several, the window that ends first. So a window is used before it
  resets unused, for about one switch back (each switch makes every running
  session read its context again). With no such account the one in use is kept
  until a window reaches the threshold, as in the script. `Auto-switch with
  Quota Station` (`Tự chuyển bằng Quota Station`) loads the app's LaunchAgent,
  `com.shin315.quota-station.rotation`, which runs `quota-station claude rotate`
  every 180 seconds whether or not the app is open, and logs to
  `claude/rotation.log`; a failed step is logged with its cause on the same
  line (`switch to slot N failed: <kind>: <reason>`), never a token. While
  `com.cswap.prefer-team` or `com.cswap.auto` is loaded (or `launchctl` cannot
  answer), the rotation, the switch sheet and `quota-station claude switch`
  write nothing, so two switchers never rotate the same refresh token.
<!-- @private:end -->

### The command line

The binary inside the app answers without a window:

```sh
QS="/Applications/Quota Station.app/Contents/MacOS/quota-station"
"$QS" claude status --json          # every account's windows and the one in use
"$QS" claude run 3 -- -p "…"        # claude as account 3, without switching
"$QS" claude session 3              # account 3's folder, for CLAUDE_CONFIG_DIR
```

`status --json` prints `schemaVersion`, `generatedAt`, `activeSlot` and
`accounts[]` (`slot`, `alias`, `active`, `kind`, `status`
`fresh`/`stale`/`noData`/`reloginRequired`, `fetchedAt`, `ageSeconds`,
`windows.fiveHour|sevenDay|fable` with `usedPercent` and `resetsAt`, and
`error`); emails only with `--emails`, and `--refresh` reads the accounts that
are due first. The same object is written to `claude/state.json` after every
read. `run N` runs `claude` with `CLAUDE_CONFIG_DIR` set to
`claude/sessions/N/`, whose login is the Keychain item Claude Code keys to that
folder, with the variables that would override that login
(`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` and two
`*_FILE_DESCRIPTOR` ones) removed, and returns `claude`'s exit code. For the
account in use it runs `claude` on Claude Code's own login, as `cswap run` did;
`--require-session` refuses that instead. `quota-station claude help` lists the
rest (`switch`, `import`, `add [--current | --token [--email E]]`, `remove`,
`alias`).

<!-- @private:begin -->

With the `author-tools` feature each account also has `enabled` (in the
rotation), and the rest takes `rotate [--dry-run]`, `rotation on|off`,
`enable` and `disable` too. Firstmate reads these.

<!-- @private:end -->

## Antigravity logins, quota and switching

These parts are adapted from [Cockpit Tools](https://github.com/jlcodes99/cockpit-tools)
and are built only with the `antigravity-accounts` feature (see
[`THIRD_PARTY.md`](../THIRD_PARTY.md)). Without it the `Antigravity` tab is marked
`read-only` (`chỉ đọc`): it reads the account signed in, and logs nothing in.

- **Logins.** `Add account` opens Google's consent screen in the default
  browser, with Antigravity's own OAuth client, so the screen names Antigravity,
  not Quota Station. The flow is Cockpit's: a loopback callback on a random
  port, a fresh `state`, a 10-minute wait, the code exchange, then userinfo for
  the email. A browser that cannot reach the app can paste the callback URL
  instead. The card lists the accounts by number (the order the Antigravity row
  uses) with `signed in`, `sign in again` (Google refused the refresh token),
  `needs verification` or `blocked by Google`, `Sign in again`, and `Sign out`
  with a confirmation in the row. Signing out forgets the account in this app
  only: Antigravity.app and agy stay signed in. Without a usable
  [`oauth-client.json`](#the-oauth-client-file) the card says what is missing
  and cannot add.
- **Quota.** The app reads the quota of the accounts logged in here itself, as
  Cockpit Tools does: the token, refreshed only when it has under five minutes
  left, then Cloud Code's `fetchAvailableModels` and `retrieveUserQuotaSummary`
  with Antigravity's user agent and headers (`antigravity/<version>
  darwin/<arch>`; the version is that of `/Applications/Antigravity IDE.app`,
  where Cockpit looks, or Cockpit's `1.20.5` without it). `loadCodeAssist`,
  which gives the account's project and whether it is a GCP ToS account, is
  asked after a login and while the token lacks either (unlike Cockpit, a
  failed answer ends the read like a failed quota call: the last reading stays,
  and a 429 or 5xx backs the account off); a GCP ToS account with a project is
  read from `cloudcode-pa.googleapis.com`, every other from
  `daily-cloudcode-pa.googleapis.com`. Each reading goes, in Cockpit's cache
  layout, to the app's `antigravity/` folder; the Antigravity row lists every
  account from there, beside the live read of the account in use, and
  quota-axi (`+cswap.5`) reads the same folder before Cockpit's. The
  pace is never denser than Cockpit's: the account in use with each of the
  app's reads, the others every 10 minutes and when the `Antigravity` tab opens,
  one account at a time, and no account twice within a minute (a reading
  younger than that is used as it is). A network error on
  `fetchAvailableModels` is tried three times, a second apart, as Cockpit does;
  after that, or after any other failure, the last numbers stay with their age
  and the row says `read failed: …`. After a 429 or a 5xx the account waits at
  least 2 minutes, then 4, 8, 16 and at most 30. A 403 on
  `fetchAvailableModels` marks it `blocked by Google` and stops its reads until
  a new login; an account that needs a new login is not read either. A Mac with
  no account logged in reads nothing.
- **Switching.** The sheet names the account and its allowances, says whether
  Antigravity.app runs, and that a running `agy` session keeps the old account
  until it restarts. While Antigravity.app runs it offers `Quit the app, then
  switch` (quit the app politely, switch, open it again), `Switch, keep the app
  open` (the running app may write its old login back) and `Cancel`; when it
  does not run, only `Switch` and `Cancel`. The switch makes the token fresh
  and, while the token lacks a project or the GCP ToS mark, asks
  `loadCodeAssist` for them as the quota read does (60 seconds at most); it
  quits the app and waits up to 20 seconds when asked (an app that does not
  quit stops the switch before the shared login is touched; `Switch` stops if
  the app runs again by then), writes the login Antigravity.app and agy share
  with Cockpit's own `security` command, then points the app's account index
  and binding at the new account and opens the app again. Five seconds later it
  reads Antigravity again and says it switched, that agy still uses the old
  account or could not be read, or why it did not switch. Antigravity.app older
  than 2.0 is named and refused, and one whose version cannot be read stops the
  switch before anything is written. A switch is refused, with a message, while
  an Antigravity login, logout or another switch runs, and a login is refused
  while a switch runs. For six hours after a switch, a reading that shows the
  old account back in use is counted as an overwrite and brings a notification;
  the `Antigravity` tab also shows the banner `Antigravity switch overwritten`
  (`Chuyển Antigravity bị ghi đè`) with `Switch again` and `Cancel`.

### Terms of service

Logging in with Antigravity's client and reading quota as Antigravity does
carries the same terms-of-service risk as using Cockpit Tools, no more: the app
makes the same calls, with the same user agent and headers, no more often.
[`SECURITY.md`](../SECURITY.md) says what could happen and which changes would
make the risk larger.

## The OAuth client file

The Antigravity login uses Antigravity's own OAuth client, the one Cockpit
Tools uses (its `src-tauri/src/modules/oauth.rs`). Its id and secret are not in
this repository. Put them in a file on each Mac that logs in, readable only by
you. With the real values in place of both `<…>`:

```sh
mkdir -p ~/Library/Application\ Support/com.shin315.quota-station
( umask 077
  cat > ~/Library/Application\ Support/com.shin315.quota-station/oauth-client.json <<'JSON'
{ "client_id": "<client id>.apps.googleusercontent.com", "client_secret": "<client secret>" }
JSON
)
```

The app refuses the placeholders, a file others can read (it must be mode
600), and anything that is not this JSON. `Settings › Services` shows whether
the file is usable (`Antigravity OAuth client`), and the `Antigravity` tab says
what is missing.

<!-- @private:begin -->

## The pause flag and the switcher

The rotation decides on its own every three minutes; a pause only asks it to
wait. The pause flag is `~/.config/cswap-prefer-team/pause`
(`until = YYYY-MM-DDTHH:MM` in local time and `reason = …`; no `until` pauses
until resumed), written through a temporary file and a rename; resuming
deletes it. The app's own rotation has the guard built in: while the flag holds
it switches nothing; once `until` has passed, it deletes the flag and goes on,
and its dry run prints `paused until <until>: <reason>` (or
`paused indefinitely: <reason>`). The older `cswap-prefer-team` script, which is
not part of this repository, needs the same guard at its top while it still
runs. `until` is compared with the local time as text, the same rule in both,
so they agree on when a pause ends. When a dry run made while the flag holds
still prints a normal decision, the app reports that the switcher ignores the
flag and does not show Claude as paused.

<!-- @private:end -->

## How the numbers relate to quota-axi

The readers are ported from quota-axi, so on the same Mac the app and
`quota-axi --provider codex|agy --json` give the same percentages and resets.

- **5-hour / Week.** For the active account, this is the tighter of the live
  five-hour and seven-day windows. A tie shows the week. For the other accounts
  it is the effective percent of the app's own reading (claude-swap's before
  the app keeps the accounts), labelled by the window that limits it.
- **Fable week.** For the active account, this is the live Fable weekly window.
  For the other accounts, it is the Fable scope of the same reading. When
  another window limits that account first, the scope is only a bound, so the
  app shows it as `≤98%` ("used at most") with no reset time.

## What it reads and runs

- The quota, as [above](#the-quota-readers).
- The Keychain access button (`Grant once`, `Cấp quyền một lần`) reads Claude
  Code's Keychain item once with `/usr/bin/security`, so macOS shows its own
  prompt; after `Always Allow` the quiet reads go on by themselves. The app
  opens Claude Code's Keychain item only to read the quota, and to keep and
  switch the Claude accounts; the other Keychain items it opens are its own.
- No email or account id from a read is kept or shown. Before each read the app
  reads the Claude accounts that are due and rewrites `claude/state.json`.
  <!-- @private:begin -->
  With Firstmate shown it also rewrites `claude/swap/`.
  <!-- @private:end -->
  Before the app keeps an account, the row has the one login Claude Code is
  on.
- Switching Claude runs no other program: the app takes Claude Code's lock
  folders, then `/usr/bin/security` reads and writes Claude Code's item
  `Claude Code-credentials` and its own items, and the app rewrites
  `oauthAccount` in `~/.claude.json`. `Sign in with the browser` runs
  `claude auth login` (the first `claude` on `PATH`, else in `~/.local/bin`,
  `/opt/homebrew/bin` or `/usr/local/bin`) with `CLAUDE_CONFIG_DIR` set to
  `claude/adding/`; `Import from claude-swap` reads `~/.claude-swap-backup` and
  claude-swap's Keychain items once and writes nothing there.
- Live sessions are counted from `~/.claude/sessions/*.json`: only the `pid`
  and `kind` fields are read, and a session counts when its process is alive.
<!-- @private:begin -->
- The rotation: a dry run of its rules in the app (no program runs and nothing
  switches), the last line of `claude/rotation.log` (of
  `~/Library/Logs/cswap-prefer-team.log` before the app keeps an account), and
  `launchctl list` for `com.shin315.quota-station.rotation`,
  `com.cswap.prefer-team` and `com.cswap.auto`. These run with each read while
  the popover or the main window is open, and after a switch or a pause; while
  both are hidden only the pause flag is read. `Auto-switch with Quota Station`
  runs `launchctl bootstrap` or `bootout` for the app's own LaunchAgent only;
  the app never loads, stops or removes the older ones. The rules editor's
  preview runs the same dry run on the draft, in memory.
- Copying the rotation file runs `ssh -o BatchMode=yes -o ConnectTimeout=5 -T
  <alias>` with one of two fixed scripts: one prints the rotation file (the
  app's `claude/rotation` when that Mac's app keeps accounts or has its own
  rotation file, else `~/.config/cswap-prefer-team/rotation`), the other takes
  the new file on stdin, keeps a backup, renames it into place and prints it
  back. Batch mode means SSH never asks for a password or a host key. An alias
  must start with a letter or digit, hold only letters, digits and `. - _ @`,
  and be at most 64 characters. The `ssh` used is the first on `PATH`, else
  `/usr/bin/ssh`.
<!-- @private:end -->
- Antigravity logins talk to Google when the owner presses `Add account` or
  `Sign in again`: the consent page opens in the browser (`open <url>`), then
  `https://oauth2.googleapis.com/token` and
  `https://www.googleapis.com/oauth2/v2/userinfo`. Before a quota read, a token
  with under five minutes left is refreshed at the same token URL. The tokens go
  to the app's own Keychain items through `/usr/bin/security
  add-generic-password`, `find-generic-password` and `delete-generic-password`,
  service `com.shin315.quota-station.antigravity`.
- Antigravity quota reads, at the pace above:
  `POST https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels`
  and `…/v1internal:retrieveUserQuotaSummary` (on `cloudcode-pa` for a GCP ToS
  account with a project), and `…/v1internal:loadCodeAssist` when the token
  lacks a project or the GCP ToS mark. The Antigravity version comes from the
  bundle's `product.json` or, through `/usr/bin/plutil -p`, its `Info.plist`;
  the bundle is only read.
- Switching Antigravity runs, when the owner confirms:
  `plutil -extract CFBundleShortVersionString raw` on Antigravity.app's
  `Info.plist`, `pgrep -f /Applications/Antigravity.app/Contents/MacOS/Antigravity`,
  `osascript -e 'tell application id "com.google.antigravity" to quit'` and
  `open -b com.google.antigravity` (only with `Quit the app, then switch`),
  `…/v1internal:loadCodeAssist` while the token lacks a project or the GCP ToS
  mark, `security add-generic-password -U -s gemini -a antigravity -w
  go-keyring-base64:<…> -A`, and then the Antigravity quota read above, without
  the cache. Opening the sheet runs only the `plutil` and `pgrep` questions.
- History and notifications read, after each read: the app's own Antigravity
  and Codex switch logs and, while an overwrite card waits for an answer,
  `pgrep` for Antigravity.app.
  <!-- @private:begin -->
  With the `author-tools` feature also the last 64 KB of the switcher's log,
  the rotation file (where the app keeps no Claude account), and Firstmate's
  `quota-pool.last` at the path set in `Settings › Advanced` (at most 16 KB, a
  plain file of that name only).
  <!-- @private:end -->
  The cards go through macOS's own
  notification center (`UNUserNotificationCenter`); `Open System Settings` runs
  `open x-apple.systempreferences:com.apple.Notifications-Settings.extension`.
- `Open at login` (`Mở khi đăng nhập`) registers and unregisters the app with
  macOS's `SMAppService.mainApp`, only when the switch is pressed; Settings asks
  macOS for the switch's state when it opens.
- The app's update check asks GitHub's API for this repository's latest release
  once a day while `Tell me about new versions` / `Check for app updates` is on,
  and on `Check for new versions`, and keeps the answer in `release.json`.
  `Download and install` only opens the release's DMG (or its page) in the
  browser; nothing is downloaded or installed by the app.
<!-- @private:begin -->
- `Update from source` (`Cập nhật từ mã nguồn`, `author-tools` builds) runs,
  only when pressed, `git -C <clone> pull --ff-only` (with
  `GIT_TERMINAL_PROMPT=0` and `GIT_SSH_COMMAND="ssh -o BatchMode=yes"`, so git
  never waits for a password), `git -C <clone> rev-parse HEAD`, and then, unless
  this copy was built from that commit, `/bin/bash <clone>/scripts/install.sh
  --dest <the folder this app is in> --no-open`. `Restart` (`Mở lại`) quits the
  app and has `/bin/sh` wait for it to end before `open` starts the new copy.
<!-- @private:end -->
- Codex, while the popover or the main window is open every 60 seconds, every 5
  minutes while both are hidden, and when the `Codex` tab opens: `config.toml`
  in the Codex folder; the shape of `auth.json` (which kind of login it holds;
  no value is kept); `ps -ww -Ao tty=,command=` for terminal sessions, Codex App,
  the background process and CC Switch; `codex --version`; the first line of
  each `sessions/**/rollout-*.jsonl` for its provider; and whether
  `/Applications/CC Switch.app` and `~/.cc-switch` exist (nothing inside is
  read). It checks the custom provider answers with `GET <base_url>/models`
  with no key and no login, five seconds at most, three tries; any HTTP answer,
  a 401 included, counts. (`Check` (`Kiểm tra`) when adding a provider is the
  one call that sends the key typed there, as a bearer token, to the same
  address.) A switch writes `config.toml` as described in
  [features.md](features.md#codex); with the restart box ticked, or from the
  restart sheet, it runs `codex app-server daemon restart` with `CODEX_HOME`
  set. Signing a ChatGPT account in runs `codex login` (or `codex login
  --device-auth` for a device code) with `CODEX_HOME` set to that account's own
  folder, removing one can run `codex logout` there, the folders are left out
  of Time Machine with `/usr/bin/tmutil addexclusion`, and `Wake ChatGPT` runs
  `codex exec` in each chosen account's folder with `--ignore-user-config
  --ignore-rules --sandbox read-only --ephemeral`, 180 seconds at most, then
  deletes what that run left in the folder.
- MCP, when the `MCP` tab opens, on `Read again`, and after each write:
  `~/.claude.json` (only its `mcpServers`), the Codex folder's `config.toml`
  (only `[mcp_servers]`) and `~/.gemini/config/mcp_config.json`; whether
  `claude` and each server's command are in the usual folders; and, for a
  server whose key must come from the app, whether its Keychain item is there
  (never its value until a write needs it). After a preview the owner confirmed
  it writes those two files and runs `claude mcp add-json` and
  `claude mcp remove` with `--scope user`.
- Backups, when the `Backup` tab opens, after each MCP write and once a day: the
  backup folder (iCloud Drive's `Quota Station Agents` unless `Settings ›
  General › Backup folder` says otherwise), each pack's `manifest.json` (and
  whether iCloud has downloaded it). A pack holds `manifest.json`, `mcp.json`,
  `secrets-needed.json`, `skills.json`, `plugins.json`, `instructions.json` and
  a `skills/` folder with the skills installed by hand, with no key. A restore
  reads them and writes the agent files as the MCP tab does, installs skills
  with `npx skills`, copies the others into `~/.agents/skills`, runs
  `claude plugin …` and `codex plugin …` to install plugins, writes the
  instruction files the owner ticked (keeping the old ones in
  `backups/instructions/`), and keeps `agents/restore-last.json` (what the undo
  needs, no key).
- Skills, when the `Skills` tab opens and on `Read again`: the shared folder
  `~/.agents/skills` and its lock file `~/.agents/.skill-lock.json` (read only),
  `~/.claude/skills`, `~/.gemini/config/skills`, the Codex folder's `skills` and
  the `[[skills.config]]` entries of its `config.toml`. After a preview the
  owner confirmed it makes or removes links in the two folders and writes those
  entries. `npx -y skills …` runs only on `Find skills`, `Read SKILL.md`,
  `Install`, `Update` and `Remove for good`, in an empty folder of the app's.
  Once a day (unless `Check for skill updates` is `Off`) it asks
  `api.github.com/repos/<owner>/<repo>/git/trees/HEAD` once per repository in
  the lock file, with no token, and keeps what it found in
  `agents/skill-updates.json`.
- Plugins, when the `Plugins` tab opens: `claude plugin list --json`,
  `claude plugin marketplace list --json` and `codex plugin list --json`, and
  each installed plugin's own folder. Nothing else of the two commands.
- Instruction files, when the `Instruction files` tab opens:
  `~/.claude/CLAUDE.md`, `~/.claude/rules/**/*.md` (three folders deep, at most
  200 files), the Codex folder's `AGENTS.md`, `~/.gemini/GEMINI.md` and the
  files added by hand; the choices made there are `agents/instructions.json`.
  It never writes them, except when a restore puts back a file the owner
  ticked.
  <!-- @private:begin -->
  A copy from another Mac puts back the files the owner ticked too.
  <!-- @private:end -->
<!-- @private:begin -->
- Copying to another Mac, on `Compare` and the copy in the `Copy to another
  Mac` tab: `ssh -o BatchMode=yes -o ConnectTimeout=5 -T <alias>` with one fixed
  script that finds `Quota Station.app` in `/Applications` or
  `~/Applications`, checks its `Info.plist` with `/usr/bin/plutil` and runs
  `quota-station agents read|export|start|status`; whatever a call carries goes
  on stdin, and each call stops after 20 seconds. On the Mac written to, the app
  keeps `agents/copy-job.json` (the write running, no key) and, while it runs,
  the items in `agents/incoming/`.
<!-- @private:end -->
- Variables that override a login: the bytes of `~/.zshenv`, `~/.zprofile`,
  `~/.zshrc`, `~/.bash_profile` and `~/.profile`, searched for eleven names at
  the start of a line and zeroed after (never run or `source`d), and
  `/bin/launchctl getenv NAME` for each name, whose output is only asked whether
  it is empty, then zeroed. `Open file` runs `/usr/bin/open` on that file (a
  `vscode://`, `cursor://` or `zed://` URL at the line, else `-t`). `Ignore` is
  `agents/env-ignored.json`.
- Configuration sets: the MCP and Skills tables as their tabs read them, and
  `~/.claude/sessions` and the process list for the sessions running now. The
  sets and the last apply's undo (names and on or off, no key) are
  `agents/sets.json`. Every write is the MCP or Skills tab's.
- Agent updates: each tool's version and how it was installed, and the newest
  version from where it was installed; `Update` runs the tool's own update only
  when pressed, stops after 10 minutes, and keeps its log in `cli-updates/`. The
  last check is `cli-tools.json`.
- Token statistics: `~/.claude/projects/**/*.jsonl` and the Codex folder's
  `sessions/` and `archived_sessions/`, for each line's token counts, model,
  time and project folder only; the bytes are zeroed after. The counts are
  `stats/index.json` (no text). Nothing is sent anywhere.

## The files it writes

Everything is in `~/Library/Application Support/com.shin315.quota-station/`
unless named otherwise, and each file is written through a temporary file and
a rename.

- `settings.json` (its own settings), `release.json` (the last release check)
  and `cli-tools.json` (the last Agent updates check), `cli-updates/` (their
  logs).
  <!-- @private:begin -->
  With the `author-tools` feature, `rotation-sync.json` (where each other Mac
  stands) beside them.
  <!-- @private:end -->
- `quota/<provider>.json`: the last reading of each service (windows, times and
  a hash of the login only).
- `claude/`: the Claude accounts (`accounts.json`, `usage.json`, `state.json`,
  `sessions/`, `adding/` and the lock file `.lock`), no email and no token.
  <!-- @private:begin -->
  With the `author-tools` feature also `swap/`, `rotation`, `rotation.log`,
  `prime` and `prime-state/`.
  <!-- @private:end -->
<!-- @private:begin -->
- `update/install.log`: the last update from source's build output.
<!-- @private:end -->
- `antigravity/accounts.json` (Cockpit's account index layout, no token),
  `antigravity/status.json` (accounts that need a new login or a verification,
  or that Google refused),
  `antigravity/cache/quota_api_v1_desktop/authorized/<SHA-256 of the email>.json`
  (each account's last quota reading in Cockpit's cache layout, no token), and
  after an Antigravity switch `antigravity/antigravity_legacy_instances.json`
  (the account bound to Antigravity.app, read by the Antigravity row and by
  quota-axi), `antigravity/switch-log.jsonl` and
  `antigravity/watch.json`.
- `history/history.jsonl` (the app's own switches, at most 500 lines) and
  `alerts/alerts.json` (what was sent, answered and snoozed: the newest 30
  cards), no email or token.
- `codex/state.json` (the hash of the app's last write, to tell a change made
  outside the app), `codex/switch-log.jsonl` (the last 200 switches),
  `codex/accounts.json` and `codex/accounts/<n>/` (the ChatGPT accounts the app
  keeps, each folder owner only and out of Time Machine), `codex/wake.json`
  (`Wake ChatGPT`'s settings and runs) and `backups/codex/config.toml.<time>`
  (the newest 10 copies of Codex's `config.toml` before each write, mode 600 in
  a folder only you can open, every key replaced by `<redacted>`).
- `backups/claude/` and `backups/antigravity/` (the same for Claude Code's MCP
  list and Antigravity's `mcp_config.json`), `backups/instructions/`,
  `agents/mcp-store.json` (the servers the app keeps, no key),
  `agents/mcp-log.jsonl` (the last 200 MCP writes), `agents/skill-updates.json`,
  `agents/instructions.json`, `agents/restore-last.json`, `agents/sets.json`
  and `agents/env-ignored.json`.
  <!-- @private:begin -->
  On a Mac being copied to, `agents/copy-job.json` and `agents/incoming/`.
  <!-- @private:end -->
- `stats/index.json`: the token counts, no text.
<!-- @private:begin -->
- Outside that folder: the switcher's pause flag,
  `~/.config/cswap-prefer-team/pause`, and the rotation file with its backups
  (`rotation.bak-<time>`, the newest 10).
<!-- @private:end -->
- Outside that folder, the agents' own files (`config.toml`,
  `~/.claude.json`, `mcp_config.json`, the skill folders and links, the
  instruction files, Claude Code's Keychain item) are written only after a
  preview or a sheet the owner confirmed.
