- Claude accounts can now be managed from the command line on Linux (WSL).
- On Linux, the account rotation now runs every three minutes from a systemd user timer.
- Antigravity accounts can now be managed from the command line, and the rotation switches accounts when Gemini runs low.
- A new sign-in of the account in use now reaches Claude Code.
- Switching Antigravity accounts no longer asks for the login password.

Copies of 1.2.0 and later update in place.

Install and update steps are in the [README](https://github.com/shin315/quota-station#install) ([Vietnamese](https://github.com/shin315/quota-station/blob/main/README.vi.md)).

## Checksums

SHA-256 of the files above (`shasum -a 256 -c SHA256SUMS.txt` on macOS, `Get-FileHash` in PowerShell on Windows):
