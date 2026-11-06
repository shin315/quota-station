#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Shin315
# SPDX-License-Identifier: MIT
#
# A throwaway update key for a build that is never released: the Release
# workflow's dry run and CI. It makes a key pair without a password in
# $RUNNER_TEMP, writes a Tauri config that points the updater's public key
# at it, and sets, for the job's later steps (GitHub Actions' GITHUB_ENV):
#
#   TAURI_SIGNING_PRIVATE_KEY           the private key's file
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  empty
#   UPDATER_DRY_CONFIG                  the config, for `tauri build --config`
#   UPDATER_DRY_PUBKEY                  the public key's file
#
# Files signed with it show the updater files' shape, but no copy of the app
# accepts them: the app checks against src-tauri/updater.pub.

set -euo pipefail

: "${RUNNER_TEMP:?run inside GitHub Actions}"
: "${GITHUB_ENV:?run inside GitHub Actions}"

dir="$RUNNER_TEMP/updater-dry"
rm -rf "$dir"
mkdir -p "$dir"
npx --no-install tauri signer generate --ci --password "" --write-keys "$dir/throwaway.key" > /dev/null
[ -f "$dir/throwaway.key" ] && [ -f "$dir/throwaway.key.pub" ] || {
  echo "::error::tauri signer generate wrote no key"
  exit 1
}
# Node writes the JSON, so a Windows path keeps its backslashes escaped.
node -e 'require("fs").writeFileSync(process.argv[1], JSON.stringify({ plugins: { updater: { pubkey: process.argv[2] } } }))' \
  "$dir/updater.conf.json" "$dir/throwaway.key.pub"
{
  echo "TAURI_SIGNING_PRIVATE_KEY=$dir/throwaway.key"
  echo "TAURI_SIGNING_PRIVATE_KEY_PASSWORD="
  echo "UPDATER_DRY_CONFIG=$dir/updater.conf.json"
  echo "UPDATER_DRY_PUBKEY=$dir/throwaway.key.pub"
} >> "$GITHUB_ENV"
echo "A throwaway update key in $dir; nothing signed with it can install."
