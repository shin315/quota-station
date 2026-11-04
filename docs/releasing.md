# Releasing

Releases are built by the **Release** workflow
([`.github/workflows/release.yml`](../.github/workflows/release.yml)), started
by hand. It never publishes and never creates a tag: it leaves a draft, and the
tag `v<version>` appears when the owner presses **Publish** on it.

## What it builds

| Asset | Built on | Signed |
|---|---|---|
| `Quota.Station_<version>_universal.dmg` | `macos-latest`, `--target universal-apple-darwin` | Developer ID Application, notarized and stapled (app and disk image) |
| `Quota.Station_<version>_x64-setup.exe` | `windows-latest`, NSIS | No: SmartScreen asks once (README) |
| `SHA256SUMS.txt` | | |

These are the names the app's update check looks for
(`crates/quota-station-core/src/release.rs`, `installs`): the first `.dmg` on
macOS, the file ending in `_x64-setup.exe` on Windows x64. Keep one `.dmg` per
release.

Both builds have the Antigravity accounts (`--features antigravity-accounts`,
`QS_ANTIGRAVITY_ACCOUNTS=1`, and `src-tauri/tauri.antigravity.conf.json`, so the
bundle's licence reads `MIT AND CC-BY-NC-SA-4.0`), carry `LICENSE`, `NOTICE` and
`LICENSES/CC-BY-NC-SA-4.0.txt`, and set `QS_RELEASE=1`, `QS_COMMIT` (the commit
built) and `QS_BUILD` (`git rev-list --count HEAD` on a full checkout).

The notes come from [`.github/release-notes.md`](../.github/release-notes.md)
(`{{VERSION}}` is replaced), followed by the checksums. The app lists the first
six `- ` lines of the notes, so those stay short and English; the Vietnamese
part is a paragraph. Edit the file before a release.

## Running it

1. The version in `Cargo.toml` (`[workspace.package]`),
   `src-tauri/tauri.conf.json` and `package.json` is the one to release, on
   `main`, and no tag `v<version>` exists. The first job stops otherwise.
2. **Dry run** (the default): Actions › Release › Run workflow, or

   ```sh
   gh workflow run release.yml -R shin315/quota-station -f version=<version> -f dry_run=true
   ```

   It builds both installers (the app signed ad-hoc, as a build from source
   is) and keeps them, `SHA256SUMS.txt` and the notes as the run's artifacts
   for 14 days. It uses no environment and no secret and creates nothing.
3. **Real run**: the same with `dry_run=false`, from `main`. It needs the
   `release` environment with a required reviewer and the six secrets below;
   it stops before building when the environment is missing, unprotected (no
   required reviewer or no deployment branch policy) or its settings cannot be
   read, and the macOS job stops when a secret is missing. The macOS job
   waits for the reviewer's approval, then the last job creates the draft
   release `v<version>` on the built commit. A run that fails leaves no draft. A draft
   that is wrong can be deleted and the workflow run again. Once published, the
   release, its assets and its tag cannot be changed, and the name of a deleted
   published tag cannot be used again, so a bad published release is fixed
   with a new version.

## One-time repository setup

Before the first real run:

- A **Developer ID Application** certificate, exported with its key as a
  `.p12`, and an **App Store Connect API key** with Developer access for
  notarization, entered as the six secrets of the `release` environment below.
- The **`release` environment**, with the owner as required reviewer (below).
- A **tag ruleset** for `v*` that lets only admins create, update or delete
  those tags (Settings › Rules › Rulesets).
- **Immutable releases** turned on (Settings › General › Releases).

### The release environment

Settings › Environments › `release`: the owner as required reviewer,
deployment branches and tags limited to `main` and `v*`. Its secrets (Tauri
reads the first five; the Release workflow writes the sixth to a temporary
file and points `APPLE_API_KEY_PATH` at it; the last is the
[Homebrew](#homebrew) workflow's):

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | the Developer ID Application certificate and key, `.p12`, base64 |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12`'s password |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: <name> (<team id>)` |
| `APPLE_API_ISSUER` | App Store Connect API Issuer ID |
| `APPLE_API_KEY` | App Store Connect API Key ID |
| `APPLE_API_KEY_P8` | the key's `AuthKey_<id>.p8`, its whole text |
| `HOMEBREW_TAP_DEPLOY_KEY` | the private key of shin315/homebrew-tap's deploy key with write access |

The signed app runs with the hardened runtime and
`src-tauri/Entitlements.plist` (Apple Events, to quit Antigravity before an
account switch); `NSAppleEventsUsageDescription` in `src-tauri/Info.plist` is
what macOS shows when it asks.

## Before pressing Publish

On the draft's page (only people with write access see it):

- The three assets are there with the names above, and `shasum -a 256 -c
  SHA256SUMS.txt` (or `Get-FileHash`) matches the downloads.
- macOS: the DMG opens without a Gatekeeper warning beyond "downloaded from
  the Internet"; `spctl -a -vv "/Applications/Quota Station.app"` says
  `source=Notarized Developer ID`. Settings shows `<version> (build N ·
  <commit>)` and offers the download, not a rebuild from a clone. Switching
  Antigravity accounts quits and reopens Antigravity (macOS asks once to let
  Quota Station control it).
- Windows: SmartScreen's More info › Run anyway installs it; over an older
  version the installer offers the upgrade. The same checks in the app.
- The notes read right, and the release is not marked as a pre-release.

Publishing creates the tag on the draft's commit. From then on the app's
update check offers this version.

## Homebrew

Publishing also starts the **Homebrew** workflow
([`.github/workflows/homebrew.yml`](../.github/workflows/homebrew.yml)). Its
job is in the `release` environment, so it waits for the same approval. Then it
checks the disk image's sha256 in `SHA256SUMS.txt` against the digest GitHub
records for the asset, sets `version` and `sha256` in `Casks/quota-station.rb`
of [shin315/homebrew-tap](https://github.com/shin315/homebrew-tap) and pushes
the commit `quota-station <version>` to its `main` with the environment's
secret `HOMEBREW_TAP_DEPLOY_KEY`, the tap's deploy key with write access. Drafts
and pre-releases are skipped.

To run it again for a published release: Actions › Homebrew › Run workflow
with the tag, or

```sh
gh workflow run homebrew.yml -R shin315/quota-station -f tag=v<version>
```

A cask already at that version and sha256 is left as it is.
