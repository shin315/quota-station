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
  as they are. Switching the Antigravity account from the app, a later
  milestone, will only write it, with the same `security` command Cockpit
  Tools uses.

The app gets Claude numbers only from quota-axi's output, and switches
Claude only through `cswap switch N --json`.

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
  quota-axi reads the index and the readings, as it reads Cockpit's. The
  files are mode 600; logging out removes the account's reading.

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

Emails stay in the index and the Keychain. They do not show in the windows
unless the owner turns on `Hiện email`, and the app logs none. The windows
do get each account's id, which is Cockpit's: `antigravity_` and the MD5 of
the email. It is not the email, but anyone who guesses an address can check
the guess against it, so it is kept as private as the index.

Tokens are refreshed only right before a use (a quota read), when they have
under five minutes left. Nothing refreshes them on a timer. Logging out
deletes the app's item and the index line, and asks Google to revoke nothing.

The only place outside its folder the Antigravity code reads is the
Antigravity app's bundle, `/Applications/Antigravity IDE.app` (where
Cockpit Tools looks), for its version: `product.json`, google-auth-library's
`package.json`, and `Info.plist` through `/usr/bin/plutil -p`. It never
writes there.

## How the tests hold the app to this

- Every file the Antigravity code opens goes through one guard
  (`crates/quota-station-core/src/antigravity/fs.rs`). It refuses any path
  outside the app's folder, and any path through a folder in it that is a
  symlink, before opening it; its tests check both. The symlink check runs
  just before the file is opened, so it guards against mistakes in the
  app, not against another program of the same macOS user, which could
  read the folder anyway. The same file holds the read-only `Bundle` the
  version is read through; it names files only inside the bundle.
- `tests/antigravity_spy.rs` runs every login flow (two logins, a pasted
  callback, a new login of an account, a refresh, a refused refresh, a
  logout) and quota reads of both accounts (in turn, from the cache, and a
  refused one) in a temporary home that has a decoy, holding a sentinel
  secret, at every path listed above, and a decoy of the shared Keychain
  item. It checks that every file opened was in the app's folder or the
  fake Antigravity bundle, that every `security` run named the app's own
  service and no `-A`, that the decoys are unchanged, and that the sentinel
  appears nowhere the app wrote or sent anything. A second test checks that
  no other Antigravity source file, nor the app's quota thread, opens files
  itself or names those paths or the shared item.
- The tests talk to a local fake Google (OAuth and Cloud Code) and a fake
  `security` that keeps items in a temporary folder. The Google and Cloud
  Code endpoints and the Antigravity bundle can be changed only in a test
  build, and `Keychain::system()` refuses to run in one, so no test can
  reach Google or the real Keychain.
- CI has no Keychain access and no Google account.

## Terms of service

The Antigravity login borrows Antigravity's own OAuth client, as Cockpit
Tools does, so Google's consent screen names Antigravity, not Quota Station.
The app calls Google exactly as Cockpit does: the same client, flow, scopes,
parameters and refresh rule, and for quota the same three Cloud Code calls
(`loadCodeAssist`, `fetchAvailableModels`, `retrieveUserQuotaSummary`) with
Antigravity's user agent, headers and bodies, built by code ported from
Cockpit. It calls never more often: `loadCodeAssist` less often than
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
