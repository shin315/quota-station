# Security

Quota Station is a personal app that runs on its owner's own Macs. This file
says which secrets it keeps, which it never touches, how the tests hold it
to that, and the terms-of-service risk of its Antigravity login. Anyone who
changes the app should keep these rules, or say plainly which one changes.

## What the app never reads, prints or copies

- claude-swap's logins, except once, read-only, when the owner presses
  `Nhập từ claude-swap` (see [The Claude accounts](#the-claude-accounts)):
  the app never writes in `~/.claude-swap-backup`, never writes or deletes
  a Keychain item of claude-swap's, and never reads its
  `sessions/*/.credentials.json`.
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

The app keeps the Claude accounts itself and runs no claude-swap command;
what it reads and writes of Claude Code's login is in
[The Claude accounts](#the-claude-accounts).

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
  The Antigravity row lists the accounts from the index and the readings,
  and Firstmate's quota-axi reads them as it reads Cockpit's. After a
  switch also `antigravity_legacy_instances.json` (only the account id
  bound to Antigravity.app, which the row and Firstmate's quota-axi read),
  `switch-log.jsonl`
  (time, account ids, the button pressed and the result of the last 200
  switches) and `watch.json` (the six-hour watch of the last switch). The
  files are mode 600; logging out removes the account's reading.
- `quota/<provider>.json` (mode 600): each quota reader's last good
  reading (below): windows, resets, error words and the first 16 hex
  digits of a SHA-256 of the login it was read with; no token, header or
  email. `quota/claude-keychain-hold` marks that macOS held Claude Code's
  Keychain item back.
- `update/install.log` (M7, mode 600): the output of the last
  `Cập nhật từ mã nguồn` build, which names folders on this Mac and nothing
  secret; each update replaces it.
- `history/history.jsonl` and `alerts/alerts.json` (M6, folders mode 700,
  files mode 600): the switch history (time, Claude slot numbers or the
  app's Antigravity account ids, how a switch was made, at most 500 lines)
  and the alerts (each card's words and buttons, when it was sent, what was
  pressed and how it went, and the snoozes; the newest 30 cards). Neither
  holds an email or a token: cards name accounts as the windows do
  (`Team A`, `Ultra 2`), and the usage read's error text never goes on a card:
  a login error shows only as one of the known codes (`invalid_grant`,
  `no_refresh_token`, `no-access-token`, `relogin_required`, or `relogin`
  for any other that asks for a new login).

- `agents/restore-last.json` (M10, mode 600): the last restore from a
  backup pack (its time, the pack's name and Mac name, the changes that
  undo it, the servers it put in the app's store and the Keychain items it
  made, and from M12 the skills it installed with their folder hashes, the
  plugins and Claude Code marketplaces it added, and the instruction files
  it wrote with their SHA-256 and backup's name), so `Hoàn tác` can take the
  whole restore back; undoing it removes the file. It holds no secret.
- `backups/instructions/<path>.<time>` (M12, mode 600 in a folder of mode
  700): an instruction file as it was before a restore replaced it, the
  newest 10 of each file. A file with a line shaped like a key is never
  replaced, so none of these holds one.
- `agents/copy-job.json` (M13, mode 600): on the Mac a copy is written to,
  the copy or undo running there (when it started, its process id) and how
  it ended (the same result a restore shows). It holds no key. While a copy
  is written, the items sent are in `agents/incoming/` (mode 700), as a
  pack of only those items with no key; it is removed when the copy ends.
- `agents/sets.json` (M15, mode 600): the configuration sets (each MCP
  server's and skill's name with on or off, and a side of the Codex switch)
  and what puts the last apply back (the same names, on or off, and the
  files' hashes). It holds no definition and no key.
- `stats/index.json` (M16, mode 600): the token statistics' index. For each
  session log read, its path, device and inode, how many bytes were read,
  the SHA-256 of its first 256 bytes, how many lines were skipped, the
  agent's version and, for Codex, the folder, model and running token
  totals; the token counts per quarter hour, agent, model and project
  folder for the last 62 days; and an 8-byte hash of each Claude response's
  id, so a response is counted once. No line of a log, no prompt, reply,
  tool output or instruction, nothing that came from a conversation.

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
the Claude accounts' emails and organizations from the app's own Keychain
items (held in memory only), while `Hiện email` is on, which it is by default;
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
`Cài đặt`, Firstmate's `quota-pool.last` (only a build with the
`author-tools` feature reads it). No path to that file is built in.
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

## Reading the quota

The app reads the quota itself (`crates/quota-station-core/src/quota/`,
ported from quota-axi's MIT source, `THIRD_PARTY.md`); no other program
reads it for the app. A reader takes a login only to send its one usage
request and never refreshes, rotates or writes it: an expired login shows
as stale or as a sign-in to do, until the tool that owns it renews it. No
token, header or email from a read reaches a file, a log or a window.

- **Codex.** `auth.json` in the Codex folder (`~/.codex`, or the one set
  in `Cài đặt`), and the one in each ChatGPT account's own folder the app
  keeps (below), is read into memory for the access token and the account
  id, which go only to `https://chatgpt.com/backend-api/wham/usage` (then
  `…/codex/usage`), and are dropped. An account waiting in its own folder
  whose token's time has run out is not asked at all. No file's content is
  ever rewritten and Codex's `app-server` is never started.
- **Antigravity.** The app reads no Antigravity credential, and not the
  shared `gemini`/`antigravity` item. `agy -p /quota --output-format json`
  answers with agy's own login; stand-ins for `open` and `xdg-open`, which
  only fail, go first on its `PATH`, so it cannot open a browser. A running
  agy or Antigravity.app is asked on 127.0.0.1 only, with the CSRF token
  its language server has on its command line (read with `ps`); that token
  goes to that one request and is not kept. The account's email in an
  answer is compared by a hash and never kept.
- **Claude Code**, on a Mac where the app keeps no Claude account: the item
  `Claude Code-credentials` is read with `security find-generic-password`,
  waiting 5 seconds at most; when macOS holds it back (a prompt not
  answered, or refused) the app stops reading its value and only asks
  whether it is there, so no prompt comes back by itself, until the owner
  presses `Cấp quyền một lần` (then it waits 60 seconds for the answer).
  Without the item it reads `~/.claude/.credentials.json`. The access token
  goes only to `https://api.anthropic.com/api/oauth/usage`. A 401 is shown
  as a sign-in, or as an expired token Claude Code will renew, never
  refreshed by the app.

The tests (`tests/quota_readers.rs`) read made-up logins against local
fakes and check that the login file and the Keychain item are unchanged
after every read, that no token request is ever sent, and that `agy` never
reaches a browser.

## The Claude accounts

The app keeps the Claude accounts the way claude-swap did, and the logic
that touches Claude Code's login follows claude-swap's MIT source
(`THIRD_PARTY.md`). [`docs/claude-accounts.md`](docs/claude-accounts.md)
has the design.

- **Keychain.** Two items per account in the login Keychain, service
  `com.shin315.quota-station.claude`: `slot-N`, the credential exactly as
  Claude Code stores it (a login's access and refresh token, or a setup
  token), and `slot-N.account`, the account's `oauthAccount` object (its
  email and organization). They are written with `security -i`, the value
  as hex on stdin so it stays off the command line, as claude-swap writes
  (a value too long for one line of `security -i` goes on the command line,
  as claude-swap's does). Removing an account deletes both.
- **Claude Code's login.** The item `Claude Code-credentials` (account
  `$USER`) and `~/.claude.json`'s `oauthAccount` are read to tell which
  account is in use and, for that account only, to read its usage with
  Claude Code's own token; the app never refreshes that token. A switch,
  started by the owner or by the rotation, writes that item (keeping the
  live login's MCP and plugin secrets) and replaces only `oauthAccount` in
  `~/.claude.json`, under Claude Code's lock folders
  (`~/.claude/.oauth_refresh.lock`, `~/.claude.lock`,
  `~/.claude.json.lock`); `~/.claude/.credentials.json` is read and
  rewritten only when Claude Code already keeps one. It reads both back and
  puts them back as they were when they do not match. A login the app does
  not keep stops the switch, so no login is ever lost.
- **Session folders.** `quota-station claude run N` and the rotation's
  priming run `claude` with `CLAUDE_CONFIG_DIR` set to `claude/sessions/N/`;
  the login goes to the Keychain item Claude Code keys to that folder
  (`Claude Code-credentials-<8 hex>`), never to a file, with
  `ANTHROPIC_API_KEY` and the other login variables taken out of the
  environment. A login added with `Đăng nhập tài khoản mới` runs
  `claude auth login` in the folder `claude/adding/`; the app copies the
  login into its own items and deletes that folder and its item.
- **Network.** `GET https://api.anthropic.com/api/oauth/usage` with the
  account's token, at most every 180 seconds an account, and, for an
  account not in use whose token has under five minutes left,
  `POST https://platform.claude.com/v1/oauth/token` with Claude Code's
  client id; the new login goes to the app's item. Nothing else.
- **Files**, in `claude/` in the app's folder (mode 700, files mode 600):
  `accounts.json` (slot, alias, in the rotation or not, kind, the account
  and organization ids and a SHA-256 of the email and organization; no
  email, no token), `usage.json` and `state.json` (each account's windows,
  how old its reading is and its error; no email, no token), `swap/` (the
  same in claude-swap's layout for Firstmate's quota-axi, written only by a
  build with the `author-tools` feature while `Cài đặt › Nâng cao` shows
  Firstmate, with placeholder addresses
  `slot-N@quota-station.local`), `rotation`, `rotation.log`, `prime` and
  `prime-state/`, `sessions/` and `.lock`. Emails reach a window only while
  `Hiện email` is on, and `quota-station claude status --json` prints them
  only with `--emails`.
- **Import.** `Nhập từ claude-swap` reads `~/.claude-swap-backup` once:
  `sequence.json`, each account's config for `oauthAccount`, its login from
  its `credentials/*.enc` file or claude-swap's Keychain item (read with
  `find-generic-password`, nothing else), and its usage cache. It copies the
  rotation file and the priming flag from `~/.config/cswap-prefer-team` when
  the app has none. It writes nothing there.
- **LaunchAgent.** `Tự chuyển bằng Quota Station` writes
  `~/Library/LaunchAgents/com.shin315.quota-station.rotation.plist`
  (`<the app>/Contents/MacOS/quota-station claude rotate` every 180 seconds)
  and loads it with `launchctl bootstrap`; turning it off runs `bootout` and
  removes the plist. While `com.cswap.prefer-team` or `com.cswap.auto` is
  loaded, neither the rotation nor a switch from the app or the command line
  writes a login, so two switchers never rotate the same refresh token. The
  app never stops, loads or removes either of them.
- A token pasted in `Thêm setup token` goes to the app's item and is never
  shown again; the command line takes one only on stdin. Errors carry
  `security`'s exit code and first line, never a value.

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
- It never writes into `auth.json`. It reads the file into memory only to
  tell which kind of login it holds (the names of its fields, never their
  values) and, for a ChatGPT sign-in, who it is (the user id, workspace id,
  email and plan in the `id_token`'s claims); it zeroes the bytes and keeps
  nothing else. A login kept in the Keychain
  (`cli_auth_credentials_store = "keyring"` or `"auto"`) is not looked at,
  and then no account is switched.
- Several ChatGPT accounts: each one the app keeps has its own folder,
  `codex/accounts/<n>/` in the app's folder (mode 700, its `auth.json` mode
  600, the folder kept out of Time Machine; never in a backup pack or a
  copy to another Mac). `codex login` writes the sign-in there itself, run
  with that new, empty folder as `CODEX_HOME`, so the Codex in use is not
  signed out. A switch moves two whole files, the `auth.json` in the Codex
  folder to its account's folder and the chosen one in, never over another
  file and never as a copy: a sign-in lives in one place only, since two
  copies would log each other out at their next renewal. The move under
  way is noted first in `codex/accounts.json` (numbers, those ids,
  nicknames, no token), so a switch cut short is put back. Removing an
  account deletes its folder, after `codex logout` in it when the owner
  ticks it.
- A new custom provider is one table added at the end of `config.toml`
  (`name`, `base_url`, `wire_api`, `experimental_bearer_token`); removing
  one takes out that table only, never the one in use. "Kiểm tra" sends
  `GET <base_url>/models` without the key and then with it, to that address
  only, following no redirect. The key typed in the window goes one way to
  the core and comes back only as `sk-…abcd`.
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
  fake keys and fake logins (made-up `id_token` claims for the accounts), a
  fake `codex` that records its arguments and a local server for the
  reachability check, which asserts it got no `Authorization` header. No
  test reads, switches or signs out the real Codex folder.

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

## Skills, plugins and instruction files (M11)

The Skills tab turns each skill on or off per agent and installs, updates
and removes skills from GitHub through `npx skills`; the Plugin and File
hướng dẫn tabs only read.

- There is one store of skills, the shared folder `~/.agents/skills` that
  `npx skills` installs into; the app keeps no second one. It reads
  `~/.agents/.skill-lock.json` (where each skill came from) and never writes
  it: only `npx skills` does. Claude Code's `~/.claude/skills` and
  Antigravity's `~/.gemini/config/skills` are read for their entries;
  Codex's `skills` folder only for the skills it brings (`.system`), which
  are shown and never touched, as are the skills a plugin brings.
- Turning a skill on for Claude Code or Antigravity makes a relative
  symlink in that agent's folder to the skill in the shared folder; turning
  it off removes that symlink and nothing else. The app never removes a
  folder to turn a skill off, and never removes a symlink that points
  anywhere but the shared folder, except `Bỏ liên kết` for a link that
  points to nothing. Codex reads the shared folder itself, so turning a
  skill off there is a `[[skills.config]]` entry (`name`, `enabled = false`)
  in Codex's `config.toml`; the edit is checked to change nothing else of
  the file, and is the MCP tab's checked write with its backup (keys
  `<redacted>`).
- Every batch goes through a preview, and a folder or `config.toml` changed
  after it stops the batch before anything is written. Before a write the
  app saves the agent's list of links as `backups/skills/<agent>-links.json.<time>`;
  before `Cập nhật` (and `Cập nhật tất cả` or `Cập nhật cả bộ`, every
  folder before the first update), `Gỡ hẳn` and `Gom về` it copies the skill's folder to
  `backups/skills/<name>.folder.<time>` (symlinks copied as symlinks, at
  most 100 MB or 5,000 files). It keeps the newest 10 of each, mode 600 in
  folders of mode 700.
- `Gom về` moves a skill installed by hand in one agent's folder into the
  shared folder, after that copy, and puts a link where it was; a skill
  that was a link to a folder elsewhere is copied and that folder is left
  as it is. `Hoàn tác` puts it back.
- `npx skills` runs only when the owner presses `Tìm skill`, `Đọc SKILL.md`,
  `Cài`, `Cập nhật` (or, in their preview, `Cập nhật tất cả` and `Cập nhật
  cả bộ`, one skill after another) or `Gỡ hẳn`: `npx -y skills add <owner/repo> --list`,
  `use <owner/repo>@<skill>`, `add <owner/repo> --skill <name> -g -y -a
  claude-code`, `update <name> -g -y` and `remove <name> -g -y`. The
  arguments are a list, never a shell line; it runs in an empty folder of
  the app's (`agents/npx`), not in a project, with `DO_NOT_TRACK=1`,
  `DISABLE_TELEMETRY=1`, no colours, and a limit of 2 minutes (listing,
  reading) or 5 minutes (installing). The tool downloads the repository
  and copies files; nothing in a skill is run by the app or by the install.
  `Gỡ hẳn` is offered only for skills in the lock file; the app never
  removes a skill installed by hand.
- Newer versions: once a day after a reading (and on `Kiểm bản mới`), the
  app asks GitHub one question per repository in the lock file,
  `GET https://api.github.com/repos/<owner>/<repo>/git/trees/HEAD?recursive=1`,
  with no token, and compares the skill's folder hash with the one in the
  lock file. It is the app's only network call for skills. A failed check
  is tried again after an hour. What it found (names, hashes, times, the
  versions skipped with `Bỏ qua bản này`) is `agents/skill-updates.json`.
  The app never updates a skill on its own. It shows `skill có bản mới` in
  the Thông báo panel (the bell) once per set of versions, and as a macOS card only when
  `Báo bằng thông báo macOS` is on in Cài đặt (off by default);
  `Kiểm bản mới` there can be turned off.
- The Plugin tab runs `claude plugin list --json` and `claude plugin
  marketplace list --json` (with `DISABLE_AUTOUPDATER=1`, 30 seconds) and
  `codex plugin list --json`, and reads each installed plugin's own folder
  to count what it brings. It never installs, removes, turns on or off a
  plugin or a marketplace; only a restore does (M12, below). A marketplace
  URL with a user or password has them taken out before it is shown or
  packed.
- Instruction files: `~/.claude/CLAUDE.md`, `~/.claude/rules/**/*.md` (three
  levels), `AGENTS.md` in the Codex folder, `~/.gemini/GEMINI.md`, and any
  `.md` or `.txt` file in the home folder the owner adds (not in the app's
  folder, at most 1 MB). The tabs never write one; `Mở file` opens it in
  the default text editor. Only a restore writes one (M12, below).
- A line shaped like a key (a private key block, a long token-like value
  after a key-ish name, a known token prefix) or holding any secret value
  now in an agent's MCP config marks the file: it stays out of every pack,
  and the windows get the line with the value as `••••••`. `Không phải
  khoá` remembers that one line by a SHA-256 of its path and text, in
  `agents/instructions.json`, with the files added and left out; the line
  itself is not kept.
- The backup pack gains three files, listed in the manifest's `sections`:
  `skills.json` (each skill's name, repository and path for skills from
  GitHub, `hand` for the others, and the agents it is on in; no content),
  `plugins.json` (each agent's plugins by `name@marketplace`, on or off,
  and the marketplaces; an agent whose plugins cannot be read keeps the
  newest pack's list) and `instructions.json` (the text and SHA-256 of each
  instruction file the owner keeps in, with no line shaped like a key). The
  pack is refused, as before, if any file in it holds a secret value the
  app read from an agent's file. M12 adds the files of the skills installed
  by hand and restores all three (below).

## Restoring skills, plugins and instruction files (M12)

A restore now also puts back the pack's skills, plugins and instruction
files, in the same sheet, after the same preview, under the same undo.

- The pack copies each skill installed by hand into `skills/<name>/` (files
  mode 600, or 700 for those that run; folders 700), with its Git tree id
  and the list of files that run in `skills.json`. A skill with a source
  (from GitHub, in the lock file) is not copied: a restore installs it from
  there again. A skill is left out, and `skills.json` says why, when a line
  of any of its files is shaped like a key or holds a secret value read
  from an agent's file, when it holds a symlink, or when it is over 20 MB
  or 1,000 files. Each folder is checked again as the pack is written, and
  one that now holds a key stops the whole pack. `.git` and `.DS_Store` are
  not copied. Pruning an old pack removes its `skills/` folder too.
- Nothing is written before `Khôi phục`. The preview reads the shared
  folder, the agents' skill folders and the instruction files, and asks
  each agent's own command for its plugin list (`claude plugin list
  --json`, `claude plugin marketplace list --json`, `codex plugin list
  --json`); that is all it runs.
- Skills: a skill this Mac already has (in the shared folder, an agent's
  own, Codex's own) is left as it is. One from GitHub is installed with
  `npx -y skills add <owner/repo> --skill <name> -g -y -a claude-code` as
  the Skills tab does (the newest version), one per repository. One
  installed by hand is copied from the pack into the shared folder, to a
  temporary name first; its tree id must match the pack's or the copy is
  removed; then it is renamed into place. Then each agent's box is set as
  the pack had it through the Skills tab's checked batch (links, Codex's
  `[[skills.config]]` entries). Nothing in a skill is run; a skill whose
  files iCloud Drive has not downloaded is not copied.
- Plugins: a plugin this Mac already has is left as it is, and so are those
  of the agent's own marketplaces (Codex App's bundled ones). Each other is
  installed with the agent's own command, the arguments as a list, no shell,
  `DISABLE_AUTOUPDATER=1`, 3 minutes each: `claude plugin marketplace add
  <source>` when this Mac has not got the marketplace (from GitHub or a Git
  URL without credentials; one only on the other Mac's disk is not
  offered), `claude plugin install <name@marketplace>`, then `claude plugin
  disable <name@marketplace>` for one that was off; for Codex `codex plugin
  marketplace add <owner/repo>` and `codex plugin add <name@marketplace>`.
  Codex has no command to turn a plugin off, so one that was off is offered
  unticked. The app never passes `--yes` and never answers a prompt: a
  plugin whose marketplace asks to run a command of its own fails, as one
  that needs a login does. These are the same commands the Plugin tab shows.
- Instruction files: one this Mac has not got is written (offered ticked).
  One that differs is offered unticked with a line-by-line `So sánh` (key
  values masked): the Mac's own stays unless the owner ticks it. One whose
  copy here has a line shaped like a key is never replaced, and one whose
  agent is not on this Mac is left out. A pack's path must be `~/…` in the
  home folder, a `.md`, `.mdc` or `.txt` file, outside the app's folder;
  Codex's `AGENTS.md` goes to this Mac's Codex folder. Each write is checked
  against the SHA-256 the preview read (a file changed since is not
  written), keeps the old file in `backups/instructions/` and goes through
  a temporary file and a rename.
- One part failing leaves the others to run: each skill, plugin and file
  ends on its own, and the result offers `Thử lại` where trying again may
  help (a login, iCloud Drive downloading, a command that failed). `Thử
  lại` runs that one part again and adds it to the same restore record.
- `Hoàn tác` takes all of it back with the servers: each skill the restore
  installed (through `npx skills remove` for one from GitHub, removed from
  the shared folder after a copy into `backups/skills/` for one copied by
  hand, then the links to it and the app's Codex entries for it), each
  plugin it installed (`claude plugin uninstall`, `codex plugin remove`),
  each Claude Code marketplace it added once no plugin from it is left
  (`claude plugin marketplace remove`), and each file it wrote (the old one
  put back, or a new one removed). A skill or file changed since the
  restore is asked about first; with a yes it stays as it is.
- Codex's provider, logins and accounts are never touched; the only lines a
  restore may add to `config.toml` are the MCP servers and the
  `[[skills.config]]` entries, and anything a `codex plugin` command itself
  writes.

## Copying to another Mac (M13)

`Chép sang Mac khác` compares this Mac with one of `Cài đặt › Các Mac
khác` and copies the items the owner ticks, one way or the other. It talks
to the other Mac only through `ssh` and only to that Mac's own Quota
Station, which does every write there.

- One fixed script per call, built from constants only: `ssh -o
  BatchMode=yes -o ConnectTimeout=5 -T <alias> /bin/sh -c '<script>'`, the
  alias checked as for the rotation copy. The script looks for `Quota
  Station.app` in `/Applications` and then `~/Applications`, reads
  `QuotaStationAgents` from its `Info.plist` with `/usr/bin/plutil`, and
  runs `quota-station agents read|export|start|status`, nothing else.
  Nothing that is copied ever goes into the command: names, definitions,
  files and keys go on stdin. Each call stops after 20 seconds.
- A Mac whose app does not say `QuotaStationAgents` (version 1 of the app),
  or that has no app, is never run, so never read or written to: the tab
  says so and offers nothing to copy.
- Reading the other Mac (`read`) prints what a backup pack would hold:
  names, definitions with every secret replaced by `<redacted>`, skill
  sources and tree ids, plugin lists, instruction files (none with a line
  shaped like a key) and the SHA-256 of each agent file it would write.
  Never a key's value. A command at another path on the other Mac is not a
  difference.
- Nothing is written before `Chép`, and only the items ticked are sent:
  never a whole pack. When both Macs have an item but not alike, it starts
  unticked; neither side wins on its own. A newer skill is never brought
  down to an older one, and a file with a line shaped like a key on either
  Mac is never copied.
- The Mac that receives the items writes them as the restore of a pack of
  only those items (M10, M12): the same checked write against the hashes
  read at the comparison (a file changed since stops it, nothing written),
  the same backups first, the same `Hoàn tác`. A skill from GitHub that is
  newer on the sending Mac is updated with `npx -y skills update <name> -g
  -y` after its folder is kept in `backups/skills/`. The write runs on its own
  on that Mac (`agents start` starts `agents job` in its own process group
  and returns), and this Mac asks `agents status` every second, each call
  again limited to 20 seconds, for at most 15 minutes. One write at a time.
- Keys stay home unless the owner turns on `Gửi cả khoá qua SSH`, which is
  off for each new comparison. Then the values the app keeps for the
  ticked servers go inside the SSH connection on stdin, from `agents start`
  to `agents job` through a pipe, and into the receiving Mac's Keychain as
  a restore puts a pasted key; they are never on a command line or on disk
  on either Mac. With it off, a server that needs a key is added turned
  off, and the receiving Mac asks for the key.
- `Hoàn tác trên <Mac>` runs on the Mac that was written to, with the
  record Quota Station there kept (`agents/restore-last.json`), and only
  while that record is still this copy's.
- Codex's provider, logins and accounts are never part of it, and nothing
  else is ever run on the other Mac.

## Variables that override a login (M14)

`Cài đặt › Dịch vụ › Biến môi trường đè đăng nhập`, the Claude tab's red banner and
one alert per variable say when one of eleven environment variables
(`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`,
`CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CONFIG_DIR`, `OPENAI_API_KEY`,
`OPENAI_BASE_URL`, `CODEX_HOME`, `GEMINI_API_KEY`, `GOOGLE_API_KEY`,
`GOOGLE_CLOUD_PROJECT`) is set where it overrides an agent's login. The app
learns whether each is set and where, never its value.

- Five shell files in the home folder are read as bytes: `~/.zshenv`,
  `~/.zprofile`, `~/.zshrc`, `~/.bash_profile`, `~/.profile`. None is ever
  run or `source`d, and no shell is started. A name counts only at the
  start of a line (`export NAME=` or `NAME=`, after spaces); only the line
  number is kept, and the bytes are zeroed before they are let go. A file
  larger than 1 MB is not read.
- `/bin/launchctl getenv NAME` is run for each name, with fixed arguments.
  It prints the value, so its output is only asked whether it is empty and
  then zeroed at once: never kept, logged, stored or shown.
- What the windows get, what an alert says and what the app keeps hold the
  name, the file's `~/…` and the line, nothing else. The alerts show in the
  app only, never as a macOS card. `Bỏ qua` keeps the ignored names and
  places in `agents/env-ignored.json` in the app's folder.
- `Mở file` runs `/usr/bin/open` on that one shell file: at the line with
  the `vscode://`, `cursor://` or `zed://` URL when that editor is in
  `/Applications` or `~/Applications`, else `open -t`. The app never edits a
  shell file and never unsets a variable, in a file or in launchd.

## Configuration sets (M15)

`Bộ cấu hình` keeps named sets of MCP servers and skills to turn on or off
and a side of the Codex switch, and applies one to the whole Mac when the
owner presses `Áp bộ` after its preview. It adds no write path of its own.

- A set never writes into a project's folder; it changes the same user-level
  files the MCP and Skills tabs do, through their checked batches
  (`agents::batch::write`, `agents::skills::write`): each file read again,
  backed up first without its secrets, written once, only the entries the
  set names, and nothing at all when any file or skills folder changed
  since the preview. The write takes only the changes the set makes now;
  any other change sent with it is refused before anything is written.
- It turns entries on and off only: it never adds a server to an agent that
  has not got it, never copies a definition or a key between agents, and
  never installs, updates or removes a skill (it never runs `npx`). Turning
  Claude Code's server off and on again is the MCP tab's own `claude mcp`
  path, with its key from the app's Keychain item.
- Codex's provider is never changed by a set. When a set wants the other
  side, the windows open the Codex switch's own confirmation sheet (M8) as
  step 2, and nothing switches until the owner confirms it there. The undo
  does not switch Codex back either; it offers that same sheet.
- `Hoàn tác bộ` puts back, through the same batches, the entries the apply
  changed and that are still as it left them; one changed or removed since
  stays as it is.
- The sets and the last apply's undo are `agents/sets.json` in the app's
  folder, mode 600: names, on or off, and when; never a definition or a
  key. A set is never applied on its own, at login or when a project opens.

## Token statistics (M16)

`Thống kê` counts the tokens Claude Code and Codex used on this Mac from
their own session logs: `~/.claude/projects/**/*.jsonl`, and `sessions/` and
`archived_sessions/` in the Codex folder. It reads session logs but keeps
only counts, never text.

- Only four things are read from a line: the token counts, the model, the
  time and the project folder (and the agent's version, to name a format the
  app cannot read). Each line is deserialized into a struct that names those
  fields and nothing else, so serde skips the conversation, the prompts,
  the thinking, the tool calls and their output without copying them, and
  the line's bytes are zeroed once read. Nothing from a conversation is
  stored, logged, indexed, exported or shown.
- The logs are read when the tab opens and on `Đọc lại`, on a thread of its
  own, never on a timer. Later reads take only what was appended; a log is
  never written, moved or deleted. Symlinks are not followed.
- A line or a log in a format the app does not know is skipped and counted
  (`Bỏ qua N phiên`), never guessed.
- Nothing is sent anywhere, and no other Mac's logs are read: the copy to
  another Mac (M13) does not carry the index. Antigravity keeps no token log
  on the Mac the app can read, so it has no numbers.
- `Giá API tương đương` comes from one table in the code
  (`crates/quota-station-core/src/stats/prices.rs`, with its source and
  date); nothing fetches prices. The Claude account of a session is
  approximate, from the switch history the app already keeps.

## How the tests hold the app to this

- Every file the Antigravity code opens goes through one guard
  (`crates/quota-station-core/src/app_dir.rs`). It refuses any path
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
- `tests/agents_skills.rs` runs the Skills, Plugin and File hướng dẫn code
  on a temporary home: a shared folder with its lock file, Claude Code's
  and Antigravity's folders (links, a skill installed by hand, a link to
  nothing), Codex's `config.toml` with trap keys, a fake `npx` that records
  its arguments and does to the folders what `npx skills` does, fake
  `claude` and `codex` plugin listers, and a fake GitHub on a local port.
  Every skill holds a `run.sh` that leaves a mark if it runs; no test sees
  the mark. It checks that turning off never removes a folder, that only
  `[[skills.config]]` changes in `config.toml`, that a folder changed after
  the preview stops the batch, `Hoàn tác`, gathering and putting back, that
  installing and updating keep each agent's box as asked, that the check
  asks GitHub once per repository with no token and updates nothing, that
  the plugin listers are never asked to install, remove, turn on or off,
  and that an instruction file holding a key (a trap value, and an MCP
  secret) never reaches a pack, a backup or what the windows get.
  `GitHub::api()` refuses to run in a test build.
- `tests/agents_restore.rs` (M12) packs skills, plugins and instruction
  files from one temporary home and restores them on another, with a fake
  `npx`, fake `claude` and `codex` that record every argument and keep
  their plugins in files, and a fake iCloud Drive folder. It checks that a
  pack copies the skills installed by hand and not those from GitHub,
  leaves out one holding a key or a link and stops when one gains a key;
  the exact `npx` and plugin commands a restore and its undo run, and that
  they are the ones the Plugin tab shows; that a file that differs keeps
  the Mac's own unless ticked and one holding a key here is never written;
  that a failing plugin, `npx` or a skill iCloud has not downloaded leaves
  the other parts to run and `Thử lại` gets them done; and that `Hoàn tác`
  takes it all back, asking first about a skill or file changed since. Its
  traps (Codex's provider key, an MCP key, a key in a skill installed by
  hand, a key in an instruction file on the new Mac, a marketplace URL's
  password) must not be in a pack, the plan, the result, the restore
  record, the app's backups, the shared skills or the commands run, and no
  skill's `run.sh` may ever run.
- `tests/agents_copy.rs` (M13) copies between two temporary homes with a
  fake `ssh` that records its arguments and stdin and runs the script on
  the other home, a fake app bundle there (with and without
  `QuotaStationAgents` in its `Info.plist`) that replays canned replies,
  and fake `npx`, `claude`, `codex` and `security`. It checks that the
  script is the same fixed text for every call, with `BatchMode=yes` and
  the 20-second limit; that a Mac on version 1 or without the app is never
  run; that an inventory and a payload without `Gửi cả khoá` hold no trap
  key, and that with it the keys go on stdin only and land in the fake
  Keychain; that a copy is the restore's write with its undo, that a file
  changed since the comparison is not written, that invalid names and paths
  and another protocol are refused, and that a write running or lost is
  told apart. No test opens a real SSH connection.
- `tests/env_check.rs` (M14) checks fake home folders with fake shell files
  and a fake `launchctl` that records its words. It checks that a name is
  found only at the start of a line and its last line wins, that
  `launchctl` is asked `getenv NAME` for the eleven names and nothing else,
  that a file it cannot read is said and the rest still checked, that a
  shell file with commands in it (`touch`, `$(…)`, `source`) is never run,
  and the trap: a value planted in every file and in `launchctl`'s answer
  is not in the result, the alerts, any file in the app's folder or what
  the window is sent. The window's tests draw that very model
  (`src/fixtures/env-trap.json`) on Cài đặt, the Claude tab and the Thông báo panel
  and find no value there. No test reads the real home, its shell files or
  launchd.
- `tests/agents_sets.rs` (M15) applies sets on a temporary home with
  Codex's and Antigravity's files, a fake `claude` and a fake `npx` that
  record their arguments, and a fake `security`. It checks that only the
  set's entries change (the provider line and every other entry stay, a
  file with nothing to change stays byte for byte), that Codex's provider
  never changes, that a file or folder changed since the preview stops the
  whole set, that a change the set does not make is refused, that `npx` is
  never run, and the undo, also after the provider line changed. Trap keys
  in the agents' files are in no preview, result, drift, undo, sets file or
  other file of the app's.
- `tests/stats.rs` (M16) reads synthetic session logs of Claude Code 1.0
  and 2.0 and of Codex before and after its token events
  (`tests/fixtures/stats/`) in a temporary home: a response written over
  several lines or copied into a resumed session counts once, a count Codex
  repeats adds nothing, a format the app does not know is skipped and
  counted, a later read takes only what was appended (a line still being
  written waits), and a truncated, replaced, deleted or archived log is
  counted right. The trap: every prompt, reply, thinking block, tool call,
  tool output and instruction in the fixtures carries the same text, which
  is in no index, report or other file of the app's. The window's tests
  draw that very report (`src/fixtures/stats-trap.json`), its table and its
  hover cards, and find no such text. No test reads the real `~/.claude`
  or `~/.codex`.
- The Claude account tests (`tests/claude_*.rs`, `tests/switching.rs`) run
  in a temporary home with a fake `security` that keeps items in files and
  records every call, a fake `launchctl`, a fake `claude` and a local fake
  Anthropic; every login in them is made up. `Security::system()` and
  `Endpoints::anthropic()` refuse to run in a test build, so no test can
  reach the real Keychain, Claude Code's real login or Anthropic. The import
  tests check that claude-swap's folder and Keychain items are byte for
  byte as they were and that only `find-generic-password` touched
  claude-swap's service; the switch tests check that a login the app does
  not keep stops a switch, that a write that does not read back is put
  back, and that nothing is written while an older switcher is loaded.
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
