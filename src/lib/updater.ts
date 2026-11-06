// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

// "Tải và cài" on a release build that updates itself (`release.way` is
// `install`): Tauri's updater plugin reads `latest.json` from the latest
// release, downloads this system's updater file, checks its signature
// against the public key built into the app, and installs it over the
// running copy. Only such a copy registers the plugin (src-tauri/src/
// updater.rs); in any other these calls are refused.

import { invoke } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';

/** How "Tải và cài" goes (`quota_station_core::self_update::Way`). */
export type UpdateWay = 'install' | 'brew' | 'download' | 'source';

declare module './model' {
  interface ReleaseView {
    /** How "Tải và cài" goes for this copy. */
    way: UpdateWay;
  }
}

/** The command a copy Homebrew installed is updated with. */
export const BREW_UPGRADE = 'brew upgrade --cask shin315/tap/quota-station';

/** The update downloaded and checked, waiting for `installUpdate`. */
let pending: Update | null = null;

/**
 * Finds the newer release in `latest.json`, downloads this system's file and
 * checks its signature. `onProgress` hears the bytes so far and the whole
 * size when the server gives it. Rejects with `noUpdate` when `latest.json`
 * offers nothing newer for this system.
 */
export async function downloadUpdate(onProgress: (got: number, total: number | null) => void): Promise<void> {
  const update = await check();
  if (!update) throw new Error('noUpdate');
  let got = 0;
  let total: number | null = null;
  await update.download((e) => {
    if (e.event === 'Started') total = e.data.contentLength ?? null;
    else if (e.event === 'Progress') got += e.data.chunkLength;
    onProgress(got, total);
  });
  const before = pending;
  pending = update;
  if (before) void before.close().catch(() => undefined);
}

/**
 * Installs the update `downloadUpdate` downloaded over this copy. On macOS
 * the new copy runs once the app opens again (`relaunch`); on Windows the
 * app closes and the installer opens the new copy itself.
 */
export async function installUpdate(): Promise<void> {
  if (!pending) throw new Error('notDownloaded');
  await pending.install();
}

/** Quits and opens the copy just installed (macOS; Windows' installer reopens the app itself). */
export function relaunch(): Promise<void> {
  return invoke<void>('release_relaunch');
}
