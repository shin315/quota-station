// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

/** Node's, for `vite.config.ts` only; the app's own code never sees it. */
declare module 'node:url' {
  export function fileURLToPath(url: URL | string): string;
}
