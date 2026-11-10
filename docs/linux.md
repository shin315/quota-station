# Linux: the Claude accounts from the command line

The app is for macOS and Windows. On Linux, and on Linux in WSL on a
Windows PC, where Claude Code often runs without a desktop, Quota Station is
the command line alone: `quota-station-cli`, a binary of its own with the
app's `quota-station claude …` commands, the same files and the same JSON,
and the automatic rotation from a `systemd --user` timer. No window, no tray
icon.

## Build and install

```sh
cargo build --release -p quota-station-cli
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
runs on it, is left alone. Add the accounts in the order their slots have on
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
