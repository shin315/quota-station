# Linux: the Claude accounts from the command line

The app is for macOS and Windows. On Linux, and on Linux in WSL on a
Windows PC, where Claude Code often runs without a desktop, Quota Station is
the command line alone: `quota-station-cli`, a binary of its own with the
app's `quota-station claude …` commands, the same files and the same JSON.
No window, no tray icon.

## Build and install

```sh
cargo build --release -p quota-station-cli
install -m 755 target/release/quota-station-cli ~/.local/bin/quota-station
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
quota-station claude status [--json] [--refresh]
quota-station claude switch N
quota-station claude rotate --dry-run
```

`rotate [--dry-run]` is one run of the Mac's rotation: a rotation file
(`claude/rotation`, copy the one from a Mac to follow its order), else the
switch at 90 % of the fullest window to the account with the most room.

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
