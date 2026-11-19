# Linux: the Claude and Antigravity accounts from the command line

The app is for macOS and Windows. On Linux, and on Linux in WSL on a
Windows PC, where Claude Code and agy often run without a desktop, Quota
Station is the command line alone: `quota-station-cli`, a binary of its own
with the app's `quota-station claude …` commands, the same files and the
same JSON, and the automatic rotation from a `systemd --user` timer; built
with the Antigravity accounts, also the app's `quota-station antigravity …`
commands for the agy that runs on that machine
([below](#the-antigravity-accounts)). No window, no tray icon.

## Build and install

```sh
cargo build --release -p quota-station-cli
install -D -m 755 target/release/quota-station-cli ~/.local/bin/quota-station
```

With the Antigravity accounts (their code is in part under CC BY-NC-SA 4.0,
see `NOTICE`):

```sh
cargo build --release -p quota-station-cli --features antigravity-accounts
install -D -m 755 target/release/quota-station-cli ~/.local/bin/quota-station
```

The Linux build carries its own OpenSSL (the `openssl` crate's `vendored`
feature, on Linux only), so it needs a C compiler, `make` and `perl`, but not
`libssl-dev` or `pkg-config`.

## Where things are

| What | Where |
|---|---|
| Claude Code's login | `~/.claude/.credentials.json` (mode 600), and `oauthAccount` in `~/.claude.json`. Claude Code for Linux keeps no item in a keyring. |
| The accounts, usage, state, rotation file, log | `~/.config/com.shin315.quota-station/claude/` (mode 700), as on a Mac under `~/Library/Application Support`. `XDG_CONFIG_HOME` is not followed. |
| Each account's login | `~/.local/share/com.shin315.quota-station/secrets/com.shin315.quota-station.claude/slot-N.bin` and `slot-N.account.bin`, mode 600 in folders of mode 700, written to a temporary file and renamed over the old one. They are not sealed: a headless machine, or WSL, has no keyring the app could count on, and Claude Code keeps its own login the same way. |
| The rotation's units | `~/.config/systemd/user/com.shin315.quota-station.rotation.service` and `.timer` |

No command prints a token; only `status --emails` prints emails.

## Adding the accounts

```sh
quota-station claude add --current   # the account Claude Code is on now
quota-station claude add             # another: `claude auth login` in a folder of its own
quota-station claude status
```

`add` logs in with `claude auth login` in a folder of its own
(`CLAUDE_CONFIG_DIR`), so Claude Code's own login, and every Claude Code that
runs on it, is left alone, unless the login is of the account Claude Code is
on: then Claude Code's login gets it too. Add the accounts in the order their slots have on
your Macs (1, 2, 3, …) to keep the same numbers there and here.

## Switching

```sh
quota-station claude status [--json] [--emails] [--refresh]
quota-station claude switch N
quota-station claude rotate --dry-run
```

`rotate [--dry-run]` is one run of the Mac's rotation: a rotation file
(`claude/rotation`, copy the one from a Mac to follow its order), else the
switch to the account with the most room once the fullest window reaches
the threshold: `autoSwitchAt` in `settings.json` beside the `claude`
folder, 90 % by default.

`switch N` writes account N's login to `~/.claude/.credentials.json` (made
when it is not there, the live login's MCP and plugin secrets kept) and its
`oauthAccount` to `~/.claude.json`, under Claude Code's locks, as on a Mac
([claude-accounts.md](claude-accounts.md#switch)).

A Claude Code that is running takes the new login from its next request:
checked with Claude Code 2.1.322 in a terminal session, against a local
stand-in for the API with made-up logins. Each request after a switch,
including one sent at once, carried the new account's token; quitting the
session afterwards wrote neither the old login nor the old `oauthAccount`
back. Nothing needs restarting.

## The rotation

```sh
quota-station claude rotation on    # or off
systemctl --user list-timers com.shin315.quota-station.rotation.timer
tail ~/.config/com.shin315.quota-station/claude/rotation.log
```

`rotation on` writes the two units and starts the timer: `quota-station
claude rotate` 5 seconds after it starts, then every 180 seconds, as the
LaunchAgent runs it on a Mac. The rules are `rotate`'s above, the Mac's:
the rotation file, else the switch at `autoSwitchAt` (90 % by default). The pause
flag, the accounts held out (`disable N`) and the log are the Mac's too.

The timer runs while the user manager runs. On WSL that needs
`systemd=true` under `[boot]` in `/etc/wsl.conf`, and the distribution
running: WSL stops it some time after its last terminal closes, unless
something keeps it up. `loginctl enable-linger` keeps the user manager
running without a login session.

Where no user manager answers, `rotation on` writes nothing and gives the
line to run the rotation from `cron` instead (`crontab -e`):

```sh
*/3 * * * * '/home/you/.local/bin/quota-station' claude rotate >/dev/null 2>&1
```

## The Antigravity accounts

`quota-station antigravity status | switch N | switch EMAIL | rotate` work
as on a Mac ([features.md](features.md)), for the agy that runs on this
Linux machine. The accounts are not logged in here: they are logged in once
in the app on a Mac and carried over in a bundle.

### Carrying the accounts over

On the Mac, with the app built with the Antigravity accounts:

```sh
'/Applications/Quota Station.app/Contents/MacOS/quota-station' antigravity export ~/agy-accounts
```

Copy the `agy-accounts` folder to the Linux machine (on WSL, for example,
through `/mnt/c/Users/<you>/Downloads`, or over SSH with `scp -r` or the
system `tar`), then:

```sh
quota-station antigravity import ~/agy-accounts
rm -rf ~/agy-accounts
quota-station antigravity status
```

`export DIR` writes a new folder of mode 700, its files mode 600. `import
DIR` reads that folder into the app's store here. Both print the accounts
by number and email, nothing else. The accounts keep their numbers: the
Mac's numbers are this machine's, unless accounts were imported here
before, which keep theirs (the new ones come after them). An account
imported again gets its new token. Accounts held out of the rotation on the
Mac are held out here too.

The bundle holds every account's refresh token: whoever has it can use the
accounts. Keep it like a password and remove it once imported.

### The bundle

A folder (mode 700, its files 600):

| File | What |
|---|---|
| `manifest.json` | `{"format": "quota-station-antigravity-accounts", "version": 1}` |
| `accounts.json` | the account index, as the app keeps it (`version` `2.0`, each account's `id`, `email`, `name`, `created_at`, `last_used`, `rotation_paused`); `current_account_id` is left out |
| `oauth-client.json` | the OAuth client the tokens were got with, `{"client_id": …, "client_secret": …}`, which refreshes them here |
| `tokens/<id>.json` | each account's token, as the app keeps it in its Keychain item (`access_token`, `refresh_token`, `expiry_timestamp`, `email`, `project_id`…) |

`import` checks it all before it writes anything: the manifest, an index of
version 2.0 whose ids are the MD5 of their emails (`antigravity_<md5>`), a
usable client, and for each account a token with a refresh token, of that
account's email. Any other file, or a link, is refused.

### Where things are

| What | Where |
|---|---|
| agy's login | `~/.gemini/antigravity-cli/antigravity-oauth-token` (mode 600): the JSON of the access token, the refresh token and its expiry. agy for Linux keeps its login in this file where it finds no keyring it can count on: under WSL, on a host with no D-Bus session bus, over SSH, in a container. On a Linux desktop with a Secret Service agy reads the keyring first, and a switch here does not reach it. |
| The accounts, readings, switch and rotation files | `~/.config/com.shin315.quota-station/antigravity/` (mode 700), as on a Mac under `~/Library/Application Support` |
| Each account's token | `~/.config/com.shin315.quota-station/antigravity/secrets/com.shin315.quota-station.antigravity/<id>.bin`, mode 600 in folders of mode 700, written whole and renamed over the old one |
| The OAuth client | `~/.config/com.shin315.quota-station/oauth-client.json` (mode 600) |

### Switching

```sh
quota-station antigravity status [--json] [--emails] [--refresh]
quota-station antigravity switch 3
quota-station antigravity switch someone@example.com
quota-station antigravity rotate --dry-run
```

`switch` writes the account's login to agy's file, replacing it whole.
There is no desktop app to quit or open again. agy is a program each
terminal starts on its own: an agy session started after the switch is on
the new account; one that was already running stays on the old one and
writes the old login back at its next hourly token refresh, so quit and
start those again. `status` names the account in use from the last switch
(or from agy's own reading, when it answers).

`rotate` is one run of the Mac's rotation. `rotation on` is macOS's
LaunchAgent only and is refused here; to run the rotation every three
minutes on Linux, add it to `cron` (`crontab -e`):

```sh
*/3 * * * * '/home/you/.local/bin/quota-station' antigravity rotate >/dev/null 2>&1
```
