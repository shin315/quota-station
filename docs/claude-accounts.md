# Design note: Quota Station keeps the Claude accounts itself

Until now the app leaned on claude-swap (`cswap`) for four jobs: keeping the
logins of the Claude accounts, switching the active one (`cswap switch N
--json`), reading every account's usage (`cswap list --json`), and the
automatic rotation (`~/.local/bin/cswap-prefer-team`, run every three minutes
by the LaunchAgent `com.cswap.prefer-team`, which calls `cswap`). This note
says how Quota Station does all four itself. The logic that touches Claude
Code's login is learned from claude-swap's MIT source and credited in
`THIRD_PARTY.md`.

## Store

- `~/Library/Application Support/com.shin315.quota-station/claude/`
  (mode 700) holds `accounts.json` (mode 600): each account's slot number,
  alias, whether it is held out of the rotation, its kind (a login or a
  setup token), its `accountUuid` and `organizationUuid`, and a SHA-256 of
  its email and organization, to tell which account Claude Code is on
  without keeping the email. No email, organization name or token is in a
  file.
- The secrets are in the login Keychain, under the app's own service
  `com.shin315.quota-station.claude`: account `slot-N` holds the credential
  exactly as Claude Code stores it, and `slot-N.account` holds the account's
  `oauthAccount` object (the email and organization live only there, and in
  memory while the app runs). They are written with `/usr/bin/security -i`,
  the value as hex on stdin, as claude-swap writes, so no secret is on a
  command line (a value too long for one stdin line falls back to the
  command line, as claude-swap's does).
- `Nhập từ claude-swap` reads `~/.claude-swap-backup` once (its
  `sequence.json`, each slot's config for `oauthAccount`, each credential
  from its `.enc` file or Keychain item, its usage cache) and carries every
  account over under the same slot number, without a new login. It never
  writes there. Running it again changes nothing, except that a credential
  claude-swap holds a newer copy of (a later `expiresAt`) replaces the app's.
  It also copies the rotation file and the `prime` flag when the app has none.

## Switch

`switch N` (the app's sheet, the rotation, and `quota-station claude switch
N`) runs under four locks, in claude-swap's order: the app's own `flock` on
`claude/.lock`, then Claude Code's lock folders `~/.claude/.oauth_refresh.lock`,
`~/.claude.lock` and `~/.claude.json.lock` (the `proper-lockfile` protocol
Claude Code uses: `mkdir`, stale after 60, 60 and 10 seconds, touched every
3 seconds while held). Under them it:

1. reads the live login (the Keychain item `Claude Code-credentials`,
   account `$USER`) and `oauthAccount` in `~/.claude.json`; when that
   account is one of the app's, it keeps the live credential in that slot
   (never an empty or wiped one); a login the app does not know stops the
   switch, so it is never lost;
2. writes the target's credential to Claude Code's item (`-U`), keeping the
   live credential's MCP and plugin secrets, as claude-swap does, and
   rewrites `~/.claude/.credentials.json` only when it already exists;
3. replaces only `oauthAccount` in `~/.claude.json` (temporary file, rename,
   mode 600, every other key kept);
4. reads both back; when either is not what it wrote, it puts the login and
   the file back as they were and reports the failure.

A switch never refreshes a token and makes no network call.

## Usage

`GET https://api.anthropic.com/api/oauth/usage` with the account's token and
`anthropic-beta: oauth-2025-04-20`, as claude-swap asks it: the five-hour and
weekly windows and the per-model weekly windows (`limits[]`, Fable among
them). The account in use is read with Claude Code's live token, read-only:
the app never refreshes it, Claude Code does. Another account whose token has
under five minutes left is refreshed first (`POST
https://platform.claude.com/v1/oauth/token`, Claude Code's client id), under
the app's lock, after reading the stored credential again, and the new one is
stored in the app's item. An account is read at most every 180 seconds, with
claude-swap's back-off after a failure or a 429. The readings go to
`claude/usage.json` (no email, no token). Emails stay in memory and reach the
windows only while `Hiện email` is on, as before.

## Rotation host

The rotation is the `cswap-prefer-team` script's rules, ported line for line
(`claude/engine.rs`, the same decision text), reading the app's own copy of
the rotation file, `claude/rotation` (same format; the editor and the copy to
the other Macs work on it, and on a Mac that has not imported yet they still
work on `~/.config/cswap-prefer-team/rotation`). The pause flag stays where it
was, `~/.config/cswap-prefer-team/pause`, with the same guard.

It runs from the app's own LaunchAgent, `com.shin315.quota-station.rotation`,
every 180 seconds: `Quota Station.app/Contents/MacOS/quota-station claude
rotate`, so it keeps going when the windows are closed or the app is quit.
`Tự chuyển bằng Quota Station` in the Claude tab installs it (`launchctl
bootstrap`) or removes it (`bootout`). Each run, and the app before showing a
decision, checks `com.cswap.prefer-team` and `com.cswap.auto`: while either is
loaded the run switches nothing and the window says so, so two switchers never
run at once. The decisions and switches go to `claude/rotation.log`, in the
script's log format.

## Session folders

`quota-station claude run N [--require-session] -- <claude args>` runs
`claude` as account N without touching the active login, as `cswap run N`
does: `CLAUDE_CONFIG_DIR` is `claude/sessions/N/` (its `.claude.json` holds
the account's `oauthAccount`; `settings.json`, `CLAUDE.md`, `skills`,
`commands`, `agents` and `keybindings.json` are links to `~/.claude`), and
its credential is the Keychain item Claude Code keys to that folder
(`Claude Code-credentials-<first 8 hex of SHA-256 of the path>`), written
from the app's copy when the app's is newer, and taken back into the app's
copy when Claude Code refreshed it there. No credential is written to a file.
`quota-station claude session N` prepares the folder and prints its path.

## Data file

`claude/state.json` is rewritten after every read: each account's slot,
alias, whether it is in the rotation, its three windows (share used and
reset), how old its reading is and its error, and the account in use; no
email, no token. `quota-station claude status --json` prints the same, and
with `--emails` adds the emails. For quota-axi, which still reads claude-swap's
layout, the app also writes `claude/swap/sequence.json` and
`claude/swap/cache/usage.json` in that layout (slot numbers, the account and
organization ids, placeholder addresses `slot-N@quota-station.local`, no
token), so `QUOTA_AXI_CLAUDE_SWAP_DIR` and `QUOTA_AXI_CLAUDE_ROTATION` point
quota-axi at the app without a change to quota-axi. The app sets both for the
quota-axi it runs.

## Cutover

An explicit checklist in the README, run by the owner Mac by Mac: import,
stop `com.cswap.prefer-team` (`launchctl bootout`, plist kept as a backup),
turn on Quota Station's rotation. Nothing of claude-swap is removed.
