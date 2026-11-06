- The app now downloads, checks and installs a new release by itself, then reopens.
- One press of "Download and install" does it all, on macOS and on Windows.
- The download is checked against the app's built-in key before anything is installed.
- Copies installed with Homebrew keep updating with `brew upgrade`.
- Copies built from source keep their own update path.
- The tab "Rotation rules" is now called "Auto-switch accounts".

Going from 1.1.1 to 1.2.0 is still a manual install: download and install it as before. From 1.2.0 on, updates install in place.

Install and update steps are in the [README](https://github.com/shin315/quota-station#install) ([Vietnamese](https://github.com/shin315/quota-station/blob/main/README.vi.md)).

## Checksums

SHA-256 of the files above (`shasum -a 256 -c SHA256SUMS.txt` on macOS, `Get-FileHash` in PowerShell on Windows):
