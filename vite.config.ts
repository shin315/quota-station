// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { normalizePath, type Plugin } from 'vite';
import { defineConfig } from 'vitest/config';

/** Node's, read only here; the app's own code never sees it. */
declare const process: { env: Record<string, string | undefined> };

// `/`-separated on Windows too (`C:/…`), as Vite writes the module ids `noAgyScreens` compares.
const src = normalizePath(fileURLToPath(new URL('./src', import.meta.url)));
const root = src.slice(0, src.lastIndexOf('/'));
/** The Antigravity accounts' screens: CC BY-NC-SA 4.0, see `NOTICE`. */
const agyScreens = src + '/antigravity/';
/** The registry of a build without an extension. */
const noExt = src + '/ext/none.ts';

// `@qs-ext`: the interface's extension registry (src/ext/types.ts). A build
// with an extension names its folder in `QS_EXT_DIR`, from the repository's
// root, and `@qs-ext` is that folder's `index.ts`; its tests run then too,
// in a Vitest project of their own (`ext`), while the shared tests keep the
// empty registry (`app`). Without it `src/ext/none.ts` stands in, and the
// build stops if it would bundle a file of `ext/`.
const extDir = process.env.QS_EXT_DIR ? root + '/' + normalizePath(process.env.QS_EXT_DIR).replace(/^\.\/|\/$/g, '') : null;

// `@agy/`: the Antigravity accounts' screens go into the app only when it is
// built with them (`QS_ANTIGRAVITY_ACCOUNTS=1`, which `scripts/install.sh`
// sets along with the `antigravity-accounts` feature). Otherwise
// `src/antigravity-off` stands in and the bundle holds only MIT code. The
// tests and the browser preview always draw the screens.
export default defineConfig(({ command }) => {
  const withAgy = process.env.QS_ANTIGRAVITY_ACCOUNTS === '1' || !!process.env.VITEST || command === 'serve';
  return {
    plugins: [react(), tailwindcss(), ...(withAgy ? [] : [noAgyScreens()]), ...(extDir ? [] : [noExtension()])],
    // `@/`: the source folder, as shadcn/ui's components import it.
    resolve: {
      alias: {
        '@agy': src + (withAgy ? '/antigravity' : '/antigravity-off'),
        '@qs-ext': extDir ? extDir + '/index.ts' : noExt,
        '@': src,
      },
    },
    clearScreen: false,
    server: { port: 1420, strictPort: true },
    // WebView2 (Chromium) on Windows, WebKit on macOS; the target is Tauri's Vite guide's for Windows.
    build: { target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari16', outDir: 'dist' },
    test: {
      environment: 'jsdom',
      // Fixture times read like the prototype (14:20) at UTC+7.
      env: { TZ: 'Asia/Bangkok' },
      // A Mac set to Vietnamese, as the tests were written; a test that wants English says so.
      setupFiles: ['src/test-setup.ts'],
      // The logos' look rules and the icons' tint tokens are read by their tests; the other styles stay out.
      css: { include: [/logos\.css/, /tokens\.css/] },
      // Each project's own files: a project's list would add to one here.
      ...(extDir
        ? {
            projects: [
              { extends: true, test: { name: 'app', include: ['src/**/*.test.{ts,tsx}'] }, resolve: { alias: { '@qs-ext': noExt } } },
              { extends: true, test: { name: 'ext', include: [extDir.slice(root.length + 1) + '/**/*.test.{ts,tsx}'] } },
            ],
          }
        : { include: ['src/**/*.test.{ts,tsx}'] }),
    },
  };
});

/** Stops a build without the Antigravity accounts that would still bundle one of their screens. */
function noAgyScreens(): Plugin {
  return {
    name: 'quota-station:no-agy-screens',
    apply: 'build',
    load(id) {
      if (id.startsWith(agyScreens)) this.error('a build without the Antigravity accounts bundles ' + id.slice(src.length + 1));
      return null;
    },
  };
}

/** Stops a build without an extension that would still bundle a file of `ext/`. */
function noExtension(): Plugin {
  return {
    name: 'quota-station:no-extension',
    apply: 'build',
    load(id) {
      if (id.startsWith(root + '/ext/')) this.error('a build without QS_EXT_DIR bundles ' + id.slice(root.length + 1));
      return null;
    },
  };
}
