- Add account in the Antigravity tab now signs in through the browser, with no setup file.
- Works on macOS and Windows, for the first account and every account after it.
- The app finds the sign-in client in the Antigravity app or agy CLI on the same computer.
- An oauth-client.json, if there, is still read first; the files searched are only read.
- The app and its docs no longer name other tools; licence credits stay in NOTICE.
- A shorter README; the details moved to docs/features.md and docs/how-it-works.md.

Install and update steps are in the [README](https://github.com/shin315/quota-station#install) ([Vietnamese](https://github.com/shin315/quota-station/blob/main/README.vi.md)).

## Checksums

SHA-256 of the files above (`shasum -a 256 -c SHA256SUMS.txt` on macOS, `Get-FileHash` in PowerShell on Windows):
