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

- `agents/restore-last.json` (M10, mode 600): the last restore from a
  backup pack (its time, the pack's name and Mac name, the changes that
  undo it, the servers it put in the app's store and the Keychain items it
  made), so `Hoàn tác` can take the whole restore back; undoing it removes
  the file. It holds no secret.

In the owner's login Keychain, not in that folder:

- For MCP secrets (M9), one item per secret: see the MCP tab below. A
  restore (M10) adds the keys the owner pasted the same way.
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

## Switching Codex (M8)

- The app writes one file of Codex's, `config.toml` in the Codex folder
  (`~/.codex` unless set otherwise), and only when the owner confirms a
  switch. In it only `model_provider` and the custom provider table's
  `requires_openai_auth` change; the edit is checked before it is written,
  and any other change refuses the write.
- It never writes `auth.json`. It reads the file into memory only to tell
  which kind of login it holds (the names of its fields, never their
  values), zeroes the bytes and keeps nothing. A login kept in the Keychain
  (`cli_auth_credentials_store = "keyring"` or `"auto"`) is not looked at.
- A switch never writes `[mcp_servers]`; only the MCP tab does (M9, below).
  The switch is still the one path that changes the Codex provider: nothing
  in the MCP tab writes `model_provider` or a provider table.
- The custom provider's key can sit in `config.toml`. The app reads it only
  to tell how it is given (an environment variable name, a bearer token, a
  header) and that one is there; it never shows, logs or sends the key. The
  backups in `backups/codex/` are copies of `config.toml` with every secret
  value replaced by `<redacted>` (keys, tokens, passwords, secret headers and
  environment variables, secrets in an address), so they hold no key. Since
  M9 the app also rewrites, when it starts, any older backup there
  that was a whole copy, keeping its time; one it cannot read as TOML is
  removed. Backups are mode 600 in a folder of mode 700, and only the newest
  10 stay. `codex/state.json` and `codex/switch-log.jsonl` hold hashes,
  provider ids and times, no key.
- The reachability check sends `GET <base_url>/models` with no key, no
  login and no cookie. The address is the one in the provider's table.
- It reads the process table with `ps` (command lines, which can name
  folders) and keeps only counts and yes/no answers. It runs
  `codex --version` and, only after the owner confirms,
  `codex app-server daemon restart`; it never kills a Codex process.
- It never runs or opens CC Switch or reads its data: it only checks whether
  `/Applications/CC Switch.app` and `~/.cc-switch` exist.
- The Codex tests use a temporary Codex folder filled from fixtures with
  fake keys and fake logins, a fake `codex` that records its arguments and
  a local server for the reachability check, which asserts it got no
  `Authorization` header. No test reads the real Codex folder.

## The MCP tab (M9)

The MCP tab shows every MCP server of Claude Code, Codex and Antigravity in
one table and writes the owner's ticks after one preview.

- Files it reads: `~/.claude.json` (only its top-level `mcpServers` is
  parsed; the rest of the file, which can hold an API key or the account,
  is dropped unread), `config.toml` in the Codex folder (only
  `[mcp_servers]` is read into the table), and
  `~/.gemini/config/mcp_config.json`. To tell whether a server's command is
  installed it only checks that the file exists, in the usual folders.
- What it writes, and only after the owner saw the preview of that very
  write and confirmed it: in `config.toml` the `[mcp_servers.<name>]` tables
  the batch names (`enabled = false` to turn one off), in `mcp_config.json`
  the entries under `mcpServers` the batch names (`"disabled": true` to turn
  one off), and for Claude Code only `claude mcp add-json <name> '<json>'
  --scope user` and `claude mcp remove <name> --scope user`, so Claude Code
  writes `~/.claude.json` itself; the app never writes that file. Every
  other byte of the two files it edits stays as it was, which the edit is
  checked against before it is written. A server whose command lives inside
  an app's bundle (Codex App's own) and a file that does not parse are shown
  and never written.
- Each write is checked as the Codex switch's is: the file is compared with
  the one the preview read, the new text goes to a temporary file (mode
  600), the old one is backed up, the file is compared again and only then
  replaced, and read back. One file changed by another program stops the
  whole batch before anything is written. Claude Code's list is compared
  the same way, and checked after each `claude` command.
- Backups, under the app's folder: `backups/codex/` (shared with the switch),
  `backups/claude/` (Claude Code's `mcpServers` only) and
  `backups/antigravity/`, mode 600 in folders of mode 700, the newest 10 of
  each. Every secret in them is `<redacted>`; `Hoàn tác` takes secrets from
  the agent files and the Keychain, not from a backup.
- The app's own files: `agents/mcp-store.json` holds the servers in no
  agent's file (Claude Code has no off, so turning one off there removes it
  and the app keeps it; and servers put away with `Xoá`) with every secret
  `<redacted>`, the slots the secrets go back to, and the Bitwarden item
  names the owner typed; the app refuses to save it if a secret would be in
  it. `agents/mcp-log.jsonl` holds the time, the agent, `name:action` and
  the backup's file name of the last 200 writes. Both are mode 600.
- Secrets the app must keep itself (a key typed or pasted in the add form or
  for a server short of one, a server turned off in Claude Code or put away)
  go to the owner's login Keychain: service
  `com.shin315.quota-station.mcp-secret`, account `<server>:<slot>` (for
  example `docs:header:Authorization`), added without `-A` with
  `/usr/bin/security`. The value is on `security`'s command line for the
  moment it runs, the same limit as the Antigravity items. A key copied from
  one agent to another is read from the first agent's file and is not kept.
- The windows never get a secret. The table says only which secrets a server
  has, whether the app can find them, and, for two agents, whether they are
  the same. The preview, the result and the error lines mask every secret
  (`••••••`); text `claude` prints is shown with the secrets it was given
  taken out. A key typed in the form is a password field that goes to the
  Keychain before the preview, and the form forgets it; a pasted JSON or
  TOML is read with its secrets masked, and its secrets go to the Keychain
  only when the owner presses `Tiếp`. No secret is written to a log.
- `claude mcp add-json` gets the server's JSON, secrets included, on its
  command line for the moment it runs, which other processes of the same
  macOS user can see; it is the only way to give Claude Code a server
  without the app writing `~/.claude.json`. The app runs `claude` with
  `DISABLE_AUTOUPDATER=1` and a 30-second limit.
- `Hoàn tác` puts back the last batch with the secrets each server has now,
  so a secret changed by that batch is not restored; and a server Claude
  Code had turned off (kept by the app) that the batch dropped is not put
  back in the app's store.

## Backup packs and restore (M10)

The Sao lưu tab saves the MCP servers of the three agents to a pack in
iCloud Drive, and restores one on this Mac or another.

- Where: a folder per pack, `agents-<date>-<time>-<Mac>`, under
  `Quota Station Agents` at the top of the owner's iCloud Drive
  (`~/Library/Mobile Documents/com~apple~CloudDocs/`). Each holds three
  files of the app's: `manifest.json` (format, app version, Mac name, time,
  how many servers and keys), `mcp.json` (each server, each agent it is in
  and whether it is on there) and `secrets-needed.json` (which server needs
  which key, by slot name and the Bitwarden item the owner typed). The app
  keeps the newest 10 packs of this Mac and never deletes another Mac's;
  pruning removes only its three files and then the empty folder. iCloud
  Drive turned off means no pack: the tab says so and writes nothing.
- No key ever goes in a pack. Every secret is `<redacted>` before the pack
  is made, and the app refuses to write a pack if any secret value it read
  from an agent's file, or the redaction check, finds one in it. Paths under
  the home folder are written as `~/…`; servers whose command lives in an
  app's bundle, and anything the app does not own (logins, accounts, Codex's
  provider and `model_provider`), are left out. The pack is written to a
  temporary folder next to it and renamed into place.
- Packs made on their own: after each write the app makes through the MCP
  tab, and once a day after a reading, but only when the servers changed
  since this Mac's newest pack. Both can be turned off on the tab; the
  choice is `autoBackup` in `settings.json`.
- A pack iCloud Drive has not downloaded yet (the folder holds only
  `.manifest.json.icloud`, or the file is a placeholder) is listed as not
  downloaded and cannot be opened; the tab offers to open the folder in
  Finder so iCloud downloads it. A pack from a newer app, or one that does
  not parse, is listed and never opened.
- Restoring reads the pack, compares it with this Mac's agent files and
  shows one preview: servers to add (ticked), servers that differ from the
  pack (not ticked, with a side by side), and the ones already the same.
  Paths change to this Mac's home; a command not found where the pack had
  it is looked for in the usual folders. The keys the pack needs are asked
  for in password fields; a pasted key goes to the Keychain as an MCP
  secret (above) only when the owner presses `Khôi phục`, and the window
  forgets it. A server still short of a key, or whose command is not
  installed here, is added turned off: Claude Code has no off, so the app
  keeps it in its store as M9 does, and for Codex and Antigravity the store
  marks it as waiting until the owner turns it on or removes it from the
  MCP tab.
- The write is the MCP tab's checked write, with its backups: every file is
  compared with the one the preview read, and one changed by another
  program stops the restore before anything is written. A restore never
  changes a login, an account or Codex's provider.
- `Hoàn tác` takes the whole restore back from `agents/restore-last.json`:
  the servers it added or replaced, the store entries it made and the
  Keychain items it created. A file changed since the restore is not
  overwritten unless the owner confirms it.

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
- `tests/agents_mcp.rs` runs the MCP tab's reads, previews and writes
  (turning on and off in each agent, a batch over all three, `Hoàn tác`, a
  file changed after the preview, a file that does not parse, Claude Code's
  off, putting away and bringing back, editing, a server added with a typed
  or pasted key) on a temporary home, with a fake `claude` that records its
  arguments and a fake `security`. Its fixtures hold trap secrets, and a
  sentinel outside `mcpServers` in `~/.claude.json`: the traps may appear
  only in the agent files, the fake `claude`'s arguments and the fake
  Keychain, never in the app's folder (backups, store, log) or in what the
  windows get, and the sentinel nowhere. It checks that only the entries in
  the batch change, byte for byte, that every `security` run names the
  app's own service without `-A`, and that no MCP source names a login file
  or runs `security` outside `secrets.rs`. `tests/codex.rs` checks that a
  switch's backup holds no key and that an older whole backup loses its
  keys. `Secrets::system()` refuses to run in a test build.
- `tests/agents_backup.rs` makes packs and restores them (on another Mac's
  home, a server that differs, a file changed after the preview, `Hoàn tác`
  of the whole restore and after a later edit, a pack not downloaded yet,
  the 10 kept packs) in temporary homes, with a fake iCloud Drive folder, a
  fake `claude` and a fake `security`. Its fixtures hold trap secrets and a
  pasted one: no pack may hold any of them, a pack that would is refused,
  the pasted key may appear only in the fake Keychain and the agent files,
  and a restore must leave Codex's provider and the logins as they were.
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
