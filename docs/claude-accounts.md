# Design note: Quota Station keeps the Claude accounts itself

Until now the app leaned on another account switcher for keeping the logins
of the Claude accounts, switching the active one and reading every account's
usage. This note says how Quota Station does that itself. Adapted code is
credited in `NOTICE`.

Its advanced part (`Cài đặt › Nâng cao`) also runs the automatic rotation
that an earlier script ran every three minutes from its own LaunchAgent (see
Rotation host below).

## Store

- `~/Library/Application Support/com.shin315.quota-station/claude/`
  (mode 700) holds `accounts.json` (mode 600): each account's slot number,
  alias, its kind (a login or a setup token), its `accountUuid` and `organizationUuid`, and a SHA-256 of
  its email and organization, to tell which account Claude Code is on
  without keeping the email. No email, organization name or token is in a
  file.
- The secrets are in the login Keychain, under the app's own service
  `com.shin315.quota-station.claude`: account `slot-N` holds the credential
  exactly as Claude Code stores it, and `slot-N.account` holds the account's
  `oauthAccount` object (the email and organization live only there, and in
  memory while the app runs). They are written with `/usr/bin/security -i`,
  the value as hex on stdin, so no secret is on a command line (a value too
  long for one stdin line falls back to the command line).
- `Nhập từ công cụ chuyển tài khoản khác` reads the other switcher's backup
  folder in your home folder once (its `sequence.json`, each slot's config for `oauthAccount`, each credential
  from its `.enc` file or Keychain item, its usage cache) and carries every
  account over under the same slot number, without a new login. It never
  writes there. Running it again changes nothing, except that a credential
  the other switcher holds a newer copy of (a later `expiresAt`) replaces the app's.
- `accounts.json` also keeps whether each account is held out of the
  rotation. The import puts every account in the
  rotation (the other switcher's own `disabled` is not carried over: the old
  rotation script never read it), and copies the rotation file and the `prime`
  flag when the app has none.

## Switch

`switch N` (the app's sheet, `quota-station claude switch N` and every other
switch) runs under five locks, in this order: the app's own `flock` on
`claude/.lock`, then Claude Code's lock folders `~/.claude/.oauth_refresh.lock`,
`~/.claude.lock`, `~/.claude/.storage-write.lock` and `~/.claude.json.lock`
(the `proper-lockfile` protocol Claude Code uses: `mkdir`, stale after 60, 60,
15 and 10 seconds, touched every 3 seconds while held). Under them it:

1. reads the live login (the Keychain item `Claude Code-credentials`,
   account `$USER`) and `oauthAccount` in `~/.claude.json`; when that
   account is one of the app's, it keeps the live credential in that slot
   (never an empty or wiped one, nor one whose access token expires before
   the app's copy's: a new sign-in made while Claude Code's login had died
   stays, and a switch to the account in use then writes that copy); with no `oauthAccount` to tell whose it
   is, the live login is the app's account whose refresh token it holds; a
   login the app does not know stops the switch, so it is never lost;
2. writes the target's credential to Claude Code's item (`-U`), keeping the
   live credential's MCP and plugin secrets, and
   rewrites `~/.claude/.credentials.json` only when it already exists;
3. replaces only `oauthAccount` in `~/.claude.json` (temporary file, rename,
   mode 600, every other key kept);
4. reads both back; when a write fails or either is not what it wrote, it
   puts the login and the file back as they were and reports the failure.

A switch never refreshes a token and makes no network call.

## A new sign-in

`Sign in again…` and `Add account` run `claude auth login` in
`claude/adding/`; the login it makes goes to the account of its identity, a
new one or one the app has. When that is the account Claude Code is on, the
new login also goes to Claude Code's own (the account in use is read with it,
never with the app's copy): under Claude Code's login and config locks, as a
switch's step 2 writes it, read back, and put back when it does not read back
(`quota-station claude add` says so on stderr; the app says so in the line
after the sign-in). `oauthAccount` in `~/.claude.json` is not touched. A sign-in
again that ends signed in to another account than the one asked for renews or
adds that one, and the window says the account asked for is still as it was.

## Usage

`GET https://api.anthropic.com/api/oauth/usage` with the account's token and
`anthropic-beta: oauth-2025-04-20`: the five-hour and
weekly windows and the per-model weekly windows (`limits[]`, Fable among
them). The account in use is read with Claude Code's live token, read-only:
the app never refreshes it, Claude Code does. Another account whose token has
under five minutes left is refreshed first (`POST
https://platform.claude.com/v1/oauth/token`, Claude Code's client id), under
the app's lock, after reading the stored credential again, and the new one is
stored in the app's item. A login a running Claude Code holds, in the
account's session folder or as Claude Code's own login (the same refresh
token), is never refreshed by the app: it reads with that copy's token while
it lasts. A login that needs a new sign-in is not read again until another
copy of it is stored; the account in use is read all the same. When the
account in use is read with an expired Claude Code login, the reading keeps
when it expired; 15 minutes after that, still not refreshed, the account's
status is `liveExpired` (Claude Code refreshes its login when it next runs,
so a Mac left overnight wakes up with an expired one that still works). An account is read at most every 180 seconds, with a
back-off after a failure or a 429. The readings go to
`claude/usage.json` (no email, no token). Emails stay in memory and reach the
windows only while `Hiện email` is on, as before.

## Rotation host

The rotation is the earlier rotation script's rules, ported line for line
(`claude/engine.rs`, the same decision text) except for two changes. The Team
account whose weekly window resets soonest goes first, so a week is used before
it resets unused: while the Team account in use has room, a Team account with
room whose week resets sooner is taken (`switch 3 (week resets
2026-11-05T03:00 with 66% left, before slot 2)`), and when the account in use
is full, the Team account with room whose week resets soonest, then whose
five-hour window ends first. And the early switch is on: while the Team account
in use has room, the Team account whose five-hour window has started and ends
sooner is taken, with room, at least 20 minutes and 10 points before `full`
left of that window, and a week that resets no later than the one in use's
(the module doc has the whole rule). It reads the app's own copy of
the rotation file, `claude/rotation` (same format; the editor and the copy to
the other Macs work on it, and on a Mac that has not imported yet they still
work on the earlier script's rotation file under `~/.config`). The pause flag
is the app's own too, `claude/pause`, with the same guard; a flag left in the
earlier script's folder is carried over once, as it is.

It runs from the app's own LaunchAgent, `com.shin315.quota-station.rotation`,
every 180 seconds: `Quota Station.app/Contents/MacOS/quota-station claude
rotate`, so it keeps going when the windows are closed or the app is quit.
`Tự chuyển bằng Quota Station` in the Claude tab installs it (`launchctl
bootstrap`) or removes it (`bootout`). Each run, and the app before showing a
decision, checks the old auto-switcher's LaunchAgents: while either is
loaded, or `launchctl` cannot tell, the run switches nothing and the window says so, so two switchers never
run at once. The decisions and switches go to `claude/rotation.log`, in the
script's log format. A switch that fails is logged with its cause on the same
line, `switch to slot 3 failed: <kind>: <reason>` (`unknownSlot`, no account
in that slot; `busy`, a lock another process holds; `unconfirmed`, a login or account that did not read back as
written, with the old login put back; `keychain`; `noCredential`, the account
has no usable login; `unmanaged`, Claude Code is on a login the app does not
keep; `store`; `write`), and a prime that could not start as
`prime of slot 3 failed: <kind>: <reason>`. The reason is one line of at most
200 characters, with no token or Keychain value. An account held out of the rotation is never switched
to; the account in use is kept or left by the script's rules, held out or not.

On Windows, which has no launchd, the app runs the rotation itself every three
minutes while it runs and the rotation is on; the next start runs it once at
once.

On Linux, which has no app, `quota-station claude rotation on` (the command line,
`quota-station-cli` installed as `quota-station`) runs it
from a `systemd --user` timer every 180 seconds, and the accounts' logins are
files of mode 600 ([linux.md](linux.md)).

## Session folders

`quota-station claude run N [--require-session] -- <claude args>` runs
`claude` as account N without touching the active login: `CLAUDE_CONFIG_DIR` is `claude/sessions/N/` (its `.claude.json` holds
the account's `oauthAccount`; `settings.json`, `CLAUDE.md`, `skills`,
`commands`, `agents` and `keybindings.json` are links to `~/.claude`), and
its credential is the Keychain item Claude Code keys to that folder
(`Claude Code-credentials-<first 8 hex of SHA-256 of the path>`), written
from the app's copy when the app's is newer, and taken back into the app's
copy when Claude Code refreshed it there. No credential is written to a file.
`quota-station claude session N` prepares the folder and prints its path.

## Data file

`claude/state.json` is rewritten after every read: each account's slot,
alias, its three windows (share used and reset), how old its reading is and
its error, and the account in use; no email, no token. `quota-station
claude status --json` prints the same, and with `--emails` adds the emails.
Each account's `status` is `fresh`, `stale`, `noData`, `reloginRequired` (the
app's copy of its login was refused) or `liveExpired` (the account in use,
whose login in Claude Code itself stayed expired, above).
The app's Claude row is built from the same readings; no other tool is run
to read them. Each account also says whether it is in the rotation
(`enabled`).

## Cutover

On a Mac where the older switcher runs: stop its LaunchAgent (`launchctl
bootout`, plist kept as a backup; the Claude tab's `How to turn it off` gives
the exact commands), import, and turn on Quota Station's rotation.
Until the older switcher is stopped, the app switches nothing and refreshes
no token, so the two never rotate the same refresh token. Nothing of the
other switcher is removed; the way back is to turn the app's rotation off and
load the older switcher's plist again.
