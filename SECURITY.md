# Security

Quota Station is a personal app that runs on its owner's own Macs. This file
says which secrets it keeps, which it never touches, how the tests hold it
to that, and the terms-of-service risk of its Antigravity login. Anyone who
changes the app should keep these rules, or say plainly which one changes.

## What the app never reads, prints or copies

- claude-swap's logins: `~/.claude-swap-backup/credentials/`, `configs/` and
  `sessions/*/.credentials.json`, and any Keychain item of claude-swap or
  Claude Code.
- Claude Code's `~/.claude/.credentials.json`.
- Cockpit Tools' files: `~/.cockpit_tools/accounts/*.json` (its encrypted
  tokens), its `*.key` files, `server.json`, `config.json`, and the same
  files in the older `~/.antigravity_cockpit`.
- The `gemini`/`antigravity` Keychain item that Antigravity.app and the
  `agy` CLI share. The app never reads it. Logging in and out does not touch
  it either: logging out of the app leaves Antigravity.app and agy logged in
  as they are. Switching the Antigravity account from the app only writes
  it, when the owner confirms the switch, with Cockpit Tools' exact command
  `security add-generic-password -U -s gemini -a antigravity -w
  go-keyring-base64:<…> -A`. `-U` updates the item in place in one
  Keychain call; when `security` fails or is stopped after 15 seconds, the
  switch ends there and points nothing at the new account. `-A` lets any
  app read it, as Antigravity's own keyring does. The value carries only
  the access token, the refresh token, the expiry, `token_type` (`Bearer`)
  and `auth_method` (`consumer`), and is on `security`'s command line for
  the moment it runs, the same limit as below.

The app gets Claude numbers only from quota-axi's output and from
`cswap list --json` (claude-swap's cached usage, read-only, no credential),
and switches Claude only through `cswap switch N --json`.

## What the app keeps

Files, under `~/Library/Application Support/com.shin315.quota-station/`:

- `oauth-client.json`: the Antigravity OAuth client, which the owner puts
  there (see the README). The app only reads it, and refuses it unless only
  the owner can read it (mode 600). The id and the secret never go into this
  repository, the logs or the windows; the windows only learn whether the
  file is usable.
- `antigravity/` (mode 700): `accounts.json`, the account index in Cockpit's
  layout (ids, emails, names, times; no token), `status.json`, the accounts
  that need a new login or a verification or that Google refused, and
  `cache/quota_api_v1_desktop/authorized/<SHA-256 of the email>.json`, each
  account's last quota reading in Cockpit's cache layout (the email, the
  Cloud Code project, the time, and Google's quota answers; no token).
  quota-axi reads the index and the readings, as it reads Cockpit's. After
  a switch also `antigravity_legacy_instances.json` (only the account id
  bound to Antigravity.app, which quota-axi reads), `switch-log.jsonl`
  (time, account ids, the button pressed and the result of the last 200
  switches) and `watch.json` (the six-hour watch of the last switch). The
  files are mode 600; logging out removes the account's reading.
- `update/install.log` (M7, mode 600): the output of the last
  `Cập nhật từ mã nguồn` build, which names folders on this Mac and nothing
  secret; each update replaces it.
- `history/history.jsonl` and `alerts/alerts.json` (M6, folders mode 700,
  files mode 600): the switch history (time, Claude slot numbers or the
  app's Antigravity account ids, how a switch was made, at most 500 lines)
  and the alerts (each card's words and buttons, when it was sent, what was
  pressed and how it went, and the snoozes; the newest 30 cards). Neither
  holds an email or a token: cards name accounts as the windows do
  (`Team A`, `Ultra 2`), and claude-swap's error text never goes on a card:
  a login error shows only as one of the known codes (`invalid_grant`,
  `no_refresh_token`, `no-access-token`, `relogin_required`, or `relogin`
  for any other that asks for a new login).

In the owner's login Keychain, not in that folder:

- One Keychain item per Antigravity account logged in through the app:
  service `com.shin315.quota-station.antigravity`, account the app's account
  id, value the account's tokens as JSON. Items are added without `-A`, so
  another program needs the owner's consent to read one. The app adds,
  reads and deletes them with `/usr/bin/security`, as claude-swap and
  Cockpit Tools do with theirs. The value is passed to `security` on its
  command line for the moment the command runs, which other processes of
  the same macOS user can see; claude-swap and Cockpit Tools have the same
  limit.

Emails stay in the index and the Keychain. They go to the windows, with
the Claude accounts' emails and organizations from `cswap list --json`
(held in memory only), while `Hiện email` is on, which it is by default;
turned off, the model sent to the windows carries none. The app writes no
email or organization to a file of its own or a log. The windows
do get each account's id, which is Cockpit's: `antigravity_` and the MD5 of
the email. It is not the email, but anyone who guesses an address can check
the guess against it, so it is kept as private as the index.

Tokens are refreshed only right before a use (a quota read), when they have
under five minutes left. Nothing refreshes them on a timer. Logging out
deletes the app's item and the index line, and asks Google to revoke nothing.

The history and the alerts read, besides the app's own folder, only the
last 64 KB of the switcher's log and, at the path the owner sets in
`Cài đặt`, Firstmate's `quota-pool.last`. No path to that file is built in.
The app takes it only when the file is called `quota-pool.last`, opens it
without following a symlink, reads it only when it is a plain file of at
most 16 KB, and keeps only the keys
Firstmate writes there (`fork`, `auto`, `cs`, `active`, `pool` and the
per-account percents and next reset), each checked, so nothing else in a
file the path points at can show. The macOS cards go through the system's
notification center with the app's own bundle id; a card is withdrawn once
it is answered in the app.

Besides its own Keychain items, the only place outside its folder the
Antigravity code reads is the Antigravity app's bundle,
`/Applications/Antigravity IDE.app` (where Cockpit Tools looks), for its
version: `product.json`, google-auth-library's `package.json`, and
`Info.plist` through `/usr/bin/plutil -p`. It never writes there.

A switch with `Thoát app rồi chuyển` asks Antigravity.app to quit
(`osascript`, the same as its Quit menu) and opens it again with
`open -b com.google.antigravity`; the app never kills it. If it does not
quit within 20 seconds, the shared login and the account pointers are not
touched (a token refreshed for the switch stays in the app's own item, and
the attempt is logged). `Chuyển`, offered while the app was not running,
stops the same way if it runs again by then.

## Installing, updating and launching at login (M7)

- `scripts/install.sh` signs with an `Apple Development` identity it finds
  with `security find-identity -v -p codesigning` (a list of names, no
  secret), or ad-hoc. It writes only the build folder and
  `<Applications folder>/Quota Station.app`, removes only the quarantine
  mark on that copy, never adds a login item or a launchd job, and never
  uses `sudo`. A copy given with `--app` must carry the app's bundle id and
  a valid signature.
- `Cập nhật từ mã nguồn` runs the clone the owner sets in `Cài đặt`: the
  code it builds is whatever that clone's upstream holds, as when the owner
  builds by hand. git runs with prompts off, so it never waits for a
  password, and with `--ff-only`, so it never merges or resets a clone with
  local changes. It runs only when the owner presses the button.
- `Mở khi đăng nhập` uses macOS's `SMAppService.mainApp`, only when the
  owner turns the switch on or off; macOS shows the item in System Settings
  › General › Login Items, where the owner can turn it off as well. Nothing
  registers it on install or at start.

## How the tests hold the app to this

- Every file the Antigravity code opens goes through one guard
  (`crates/quota-station-core/src/antigravity/fs.rs`). It refuses any path
  outside the app's folder, and any path through a folder in it that is a
  symlink, before opening it; its tests check both. The symlink check runs
  just before the file is opened, so it guards against mistakes in the
  app, not against another program of the same macOS user, which could
  read the folder anyway. The same file holds the read-only `Bundle` the
  version is read through, and that a switch asks whether Antigravity.app
  is there at all; it names files only inside the bundle.
- `tests/antigravity_spy.rs` runs every Antigravity flow (two logins, a
  pasted callback, a new login of an account, a refresh, quota reads of
  both accounts in turn, from the cache and a refused one, a switch that
  quits and reopens a fake Antigravity.app, the overwrite watch, a refused
  refresh, a logout) in a temporary home that has a decoy, holding a
  sentinel secret, at every path listed above, and a decoy of the shared
  Keychain item. It checks that every file opened was in the app's folder
  or a fake Antigravity bundle, that every `security` run named the app's
  own service and no `-A` except the switch's one write, which must be
  Cockpit's exact command, that the decoys are unchanged (the shared item
  replaced, never read), and that the sentinel appears nowhere the app
  wrote or sent anything. A second test checks that no other Antigravity
  source file, nor the app's quota or switch code, opens files itself or
  names those paths, and that only `credential.rs` names the shared item.
- The tests talk to a local fake Google (OAuth and Cloud Code) and a fake
  `security` that keeps items in a temporary folder, and switch a fake
  Antigravity.app (a shell process) through fake `plutil`, `pgrep`,
  `osascript` and `open`. The Google and Cloud Code endpoints and the
  Antigravity bundle can be changed only in a test build, and
  `Keychain::system()`, `SharedLogin::system()` and `Desktop::system()`
  refuse to run in one, so no test can reach Google, the real Keychain or
  the real Antigravity.app.
- CI has no Keychain access and no Google account.
- The install tests run `scripts/install.sh` in a temporary folder with fake
  `open` and `security`, and fakes of `launchctl`, `osascript` and `sfltool`
  that fail the test if they run; the update tests use temporary git
  repositories and a fake install script; the login item tests use a fake
  backend. No test opens an app, writes to `/Applications` or registers a
  login item.

## Terms of service

The Antigravity login borrows Antigravity's own OAuth client, as Cockpit
Tools does, so Google's consent screen names Antigravity, not Quota Station.
The app calls Google exactly as Cockpit does: the same client, flow, scopes,
parameters and refresh rule, and for quota the same three Cloud Code calls
(`loadCodeAssist`, `fetchAvailableModels`, `retrieveUserQuotaSummary`) with
Antigravity's user agent, headers and bodies, built by code ported from
Cockpit. A switch calls Google only to refresh a token about to expire
and, while the token lacks them, to ask `loadCodeAssist` for its project
and GCP ToS mark, as Cockpit's switch does. It calls never more often: `loadCodeAssist` less often than
Cockpit, the account in use at most once a minute, the others every 10
minutes, one account at a time, and longer waits after a 429 or a 5xx. Its
tokens are used only for what Cockpit uses them for; it never calls a
model. The risk is therefore the one of using Cockpit Tools: Google may ask
an account to verify itself, revoke its refresh token or lock it. The app
then marks the account `cần đăng nhập lại`, `cần xác minh` or `Google chặn`
and does not retry a refused account on its own.

Changes that would make that risk larger, and so need the owner's say
first: another user agent or other request headers, calling more often or
for several accounts at once, a different OAuth flow, using the tokens to
call a model, and putting the OAuth client into the repository.

## Reporting a problem

Please report a security problem privately, through GitHub's "Report a
vulnerability" on this repository, and never paste a token, a Keychain value
or the OAuth client into an issue.
