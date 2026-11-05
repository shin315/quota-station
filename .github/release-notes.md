- Auto-switch when quota runs out, any Claude account (Pro, Max, Team); sessions keep running.
- It moves to the account with the most quota left once the one in use reaches 90%.
- One switch in the Claude tab or the popover, with Pause and a notification per switch.
- The threshold is in Settings › Services; a switch by hand can hold the new account.
- Homebrew: brew install --cask shin315/tap/quota-station.
- Team rotation rules stay under Settings › Advanced and decide as before.

Install and update steps are in the [README](https://github.com/shin315/quota-station#install) ([Vietnamese](https://github.com/shin315/quota-station/blob/main/README.vi.md)).

## Checksums

SHA-256 of the files above (`shasum -a 256 -c SHA256SUMS.txt` on macOS, `Get-FileHash` in PowerShell on Windows):
