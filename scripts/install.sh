#!/bin/bash
# SPDX-FileCopyrightText: 2026 Shin315
# SPDX-License-Identifier: MIT

# Builds Quota Station from this clone and installs it in an Applications
# folder, or installs a copy built on another Mac (README, "Install").
#
#   scripts/install.sh                  build, sign, install in /Applications, open
#   scripts/install.sh --no-open        the same, without opening it
#   scripts/install.sh --dest DIR       install in DIR instead of /Applications
#   scripts/install.sh --app PATH       install a built "Quota Station.app", or
#                                       the .zip that --package writes, instead
#                                       of building
#   scripts/install.sh --package [DIR]  build and sign, then write
#                                       quota-station-<version>.zip (the app and
#                                       this script) to DIR, default
#                                       target/release/bundle; installs nothing
#   scripts/install.sh --identity ID    sign with ID ("-" is ad-hoc); without it
#                                       the first "Apple Development" identity,
#                                       else ad-hoc
#
# A build from source comes with the Antigravity accounts (the
# antigravity-accounts feature, CC BY-NC-SA 4.0: NOTICE);
# QS_ANTIGRAVITY_ACCOUNTS=0 builds the MIT app without them.
# @private:begin
# It also comes with the tools made for the author's own Macs (the
# author-tools feature: Cài đặt › Nâng cao and what it holds), so a Mac that
# installs from source keeps them; QS_AUTHOR_TOOLS=0 leaves them out, as a
# release build does.
# @private:end
#
# It never adds a login item: launching at login is a switch in the app's
# settings. It touches nothing outside the build folder and DIR/Quota Station.app.
# Runs with macOS's own bash 3.2.

set -euo pipefail

APP_NAME="Quota Station"
BUNDLE_ID="com.shin315.quota-station"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

dest="/Applications"
from=""
package=""
packaging=0
identity=""
identity_given=0
open_after=1

say() { printf '==> %s\n' "$*"; }
fail() {
  printf 'install.sh: error: %s\n' "$*" >&2
  exit 1
}
# The comment block under the licence lines, up to its first empty line.
usage() { awk 'NR > 4 && /^$/ { exit } NR > 4' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --dest)
      [ $# -ge 2 ] || fail "--dest needs a folder"
      dest="$2"
      shift 2
      ;;
    --app)
      [ $# -ge 2 ] || fail "--app needs a path"
      from="$2"
      shift 2
      ;;
    --package)
      packaging=1
      if [ $# -ge 2 ] && [ "${2#-}" = "$2" ]; then
        package="$2"
        shift
      fi
      shift
      ;;
    --identity)
      [ $# -ge 2 ] || fail "--identity needs a name, or - for ad-hoc"
      identity="$2"
      identity_given=1
      shift 2
      ;;
    --open)
      open_after=1
      shift
      ;;
    --no-open)
      open_after=0
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *) fail "unknown option $1 (see --help)" ;;
  esac
done

[ "$(uname -s)" = "Darwin" ] || fail "Quota Station runs on macOS only"

# Paths given relative to where this was started, before the build moves.
abs() { case "$1" in /*) printf '%s\n' "$1" ;; *) printf '%s\n' "$PWD/$1" ;; esac; }
dest="$(abs "$dest")"
[ -z "$from" ] || from="$(abs "$from")"
[ -z "$package" ] || package="$(abs "$package")"
started_in="$PWD"

# An app started from Finder, which runs this to update itself,
# has launchd's short PATH: add where rustup, Homebrew and nvm put their
# tools, after what is there already.
extra="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin"
if [ -d "$HOME/.nvm/versions/node" ]; then
  node_dir="$(ls -1 "$HOME/.nvm/versions/node" | sort -t. -k1.2,1n -k2,2n -k3,3n | tail -n 1)"
  [ -z "$node_dir" ] || extra="$extra:$HOME/.nvm/versions/node/$node_dir/bin"
fi
export PATH="$PATH:$extra"

work="$(mktemp -d "${TMPDIR:-/tmp}/quota-station-install.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The identity to sign a fresh build with: the one asked for, else the first
# valid "Apple Development" one, else ad-hoc.
pick_identity() {
  if [ "$identity_given" = 1 ]; then
    printf '%s\n' "$identity"
    return
  fi
  local found
  found="$(security find-identity -v -p codesigning 2>/dev/null |
    awk '/"Apple Development: / { print $2; exit }')" || true
  if [ -n "$found" ]; then printf '%s\n' "$found"; else printf -- '-\n'; fi
}

sign() {
  local app="$1" id="$2"
  if [ "$id" = "-" ]; then
    say "Signing ad-hoc"
  else
    say "Signing with $id"
  fi
  codesign --force --deep --sign "$id" "$app" || fail "codesign could not sign $app"
}

verify() {
  codesign --verify --deep --strict "$1" || fail "the signature of $1 does not verify"
}

# Clears the mark macOS puts on files downloaded or received over AirDrop, so
# Gatekeeper does not stop an app signed only for this Mac.
clear_quarantine() {
  xattr -dr com.apple.quarantine "$1" 2>/dev/null || true
  if xattr -r "$1" 2>/dev/null | grep -q 'com.apple.quarantine'; then
    fail "could not clear the quarantine mark on $1"
  fi
}

version_of() {
  plutil -extract CFBundleShortVersionString raw "$1/Contents/Info.plist" 2>/dev/null || echo "?"
}

build() {
  [ -f "$ROOT/src-tauri/tauri.conf.json" ] ||
    fail "no Quota Station source next to this script: pass --app with a built copy"
  command -v cargo >/dev/null 2>&1 ||
    fail "cargo is not installed: install Rust with rustup, or build on another Mac and use --package there and --app here"
  command -v npm >/dev/null 2>&1 ||
    fail "npm is not installed: install Node.js 20 or later, or build on another Mac and use --package there and --app here"

  local target="${CARGO_TARGET_DIR:-$ROOT/target}"
  built="$target/release/bundle/macos/$APP_NAME.app"
  cd "$ROOT"
  # The commit the app shows next to its version, and that "Cập nhật từ mã
  # nguồn" compares with the clone; "-dirty" when tracked files changed.
  QS_COMMIT="$(git rev-parse HEAD 2>/dev/null || true)"
  if [ -n "$QS_COMMIT" ] && [ -n "$(git status --porcelain --untracked-files=no 2>/dev/null)" ]; then
    QS_COMMIT="$QS_COMMIT-dirty"
  fi
  export QS_COMMIT
  # `npm run build`, which Tauri runs first, reads the same variable.
  local features=""
  if [ "${QS_ANTIGRAVITY_ACCOUNTS:-1}" = 0 ]; then
    export QS_ANTIGRAVITY_ACCOUNTS=0
  else
    export QS_ANTIGRAVITY_ACCOUNTS=1
    features="antigravity-accounts"
  fi
  # @private:begin
  if [ "${QS_AUTHOR_TOOLS:-1}" != 0 ]; then
    features="${features:+$features,}author-tools"
  fi
  # @private:end
  if [ -n "$features" ]; then
    features="--features $features"
  fi

  if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
    say "Installing npm packages (npm ci)"
    npm ci --no-audit --no-fund || fail "npm ci failed"
  fi
  rm -rf "$built"
  say "Building the release app (a few minutes)"
  if cargo tauri --version >/dev/null 2>&1; then
    cargo tauri build --bundles app $features || fail "cargo tauri build failed"
  else
    npx --no-install tauri build --bundles app $features || fail "tauri build failed"
  fi
  [ -d "$built" ] || fail "the build did not write $built"
  sign "$built" "$(pick_identity)"
  verify "$built"
}

# Puts the app given with --app (a .app, or the .zip --package writes) in
# the work folder.
take() {
  local src="${from%/}"
  [ -e "$src" ] || fail "nothing at $src"
  case "$src" in
    *.zip)
      say "Unpacking $src"
      ditto -x -k "$src" "$work/unzipped" || fail "could not unpack $src"
      src="$(find "$work/unzipped" -maxdepth 2 -type d -name "$APP_NAME.app" | head -n 1)"
      [ -n "$src" ] || fail "no $APP_NAME.app in $from"
      ;;
    *.app) ;;
    *) fail "--app takes $APP_NAME.app or the .zip from --package" ;;
  esac
  [ -f "$src/Contents/Info.plist" ] || fail "$src is not an app"
  local id
  id="$(plutil -extract CFBundleIdentifier raw "$src/Contents/Info.plist" 2>/dev/null || true)"
  [ "$id" = "$BUNDLE_ID" ] || fail "$src is not Quota Station (bundle id ${id:-missing})"
  built="$work/$APP_NAME.app"
  ditto "$src" "$built" || fail "could not copy $src"
  clear_quarantine "$built"
  if [ "$identity_given" = 1 ]; then
    sign "$built" "$identity"
  fi
  verify "$built"
}

built=""
if [ -n "$from" ]; then
  take
else
  build
fi
version="$(version_of "$built")"

if [ "$packaging" = 1 ]; then
  if [ -n "$package" ]; then
    out="$package"
  elif [ -n "$from" ]; then
    out="$started_in"
  else
    out="$(dirname "$(dirname "$built")")"
  fi
  mkdir -p "$out"
  out="$(cd "$out" && pwd)"
  name="quota-station-$version"
  mkdir -p "$work/$name"
  ditto "$built" "$work/$name/$APP_NAME.app"
  cp "$HERE/install.sh" "$work/$name/install.sh"
  chmod 755 "$work/$name/install.sh"
  rm -f "$out/$name.zip"
  ditto -c -k --keepParent "$work/$name" "$out/$name.zip" || fail "could not write $out/$name.zip"
  say "Packaged $APP_NAME $version: $out/$name.zip"
  printf 'On the other Mac: ditto -x -k %s.zip . && ./%s/install.sh --app "./%s/%s.app"\n' \
    "$name" "$name" "$name" "$APP_NAME"
  exit 0
fi

# Install: copy next to the old app, then swap, so a failed copy leaves the
# old one in place.
mkdir -p "$dest" || fail "cannot make $dest"
dest="$(cd "$dest" && pwd)"
target_app="$dest/$APP_NAME.app"
staged="$dest/.$APP_NAME.app.installing"
previous="$dest/.$APP_NAME.app.previous"
rm -rf "$staged" "$previous"
say "Copying to $target_app"
ditto "$built" "$staged" || fail "could not copy into $dest"
clear_quarantine "$staged"
verify "$staged"
if [ -e "$target_app" ]; then
  mv "$target_app" "$previous" || fail "could not move the old $target_app aside"
fi
if ! mv "$staged" "$target_app"; then
  [ ! -e "$previous" ] || mv "$previous" "$target_app"
  fail "could not put the new app in place; the old one stays"
fi
rm -rf "$previous"
say "Installed $APP_NAME $version in $dest"

if [ "$open_after" = 1 ]; then
  if pgrep -f "$target_app/Contents/MacOS/" >/dev/null 2>&1; then
    say "$APP_NAME is running: quit it from its menu and open it again to use this version"
  else
    open "$target_app" || fail "could not open $target_app"
  fi
fi
