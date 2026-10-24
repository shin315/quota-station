// SPDX-FileCopyrightText: 2026 Shin315
// SPDX-License-Identifier: MIT

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import type { Plugin } from 'vite';
import { defineConfig } from 'vitest/config';

/** Node's, read only here; the app's own code never sees it. */
declare const process: { env: Record<string, string | undefined> };

const src = decodeURIComponent(new URL('./src', import.meta.url).pathname);
/** The Antigravity accounts' screens: CC BY-NC-SA 4.0, see `NOTICE`. */
const agyScreens = src + '/antigravity/';

// `@agy/`: the Antigravity accounts' screens go into the app only when it is
// built with them (`QS_ANTIGRAVITY_ACCOUNTS=1`, which `scripts/install.sh`
// sets along with the `antigravity-accounts` feature). Otherwise
// `src/antigravity-off` stands in and the bundle holds only MIT code. The
// tests and the browser preview always draw the screens.
export default defineConfig(({ command }) => {
  const withAgy = process.env.QS_ANTIGRAVITY_ACCOUNTS === '1' || !!process.env.VITEST || command === 'serve';
  return {
    plugins: [react(), tailwindcss(), ...(withAgy ? [] : [noAgyScreens()])],
    // `@/`: the source folder, as shadcn/ui's components import it.
    resolve: {
      alias: {
        '@agy': src + (withAgy ? '/antigravity' : '/antigravity-off'),
        '@': src,
      },
    },
    clearScreen: false,
    server: { port: 1420, strictPort: true },
    build: { target: 'safari16', outDir: 'dist' },
    test: {
      environment: 'jsdom',
      // Fixture times read like the prototype (14:20) at UTC+7.
      env: { TZ: 'Asia/Bangkok' },
      include: ['src/**/*.test.{ts,tsx}'],
      // A Mac set to Vietnamese, as the tests were written; a test that wants English says so.
      setupFiles: ['src/test-setup.ts'],
      // The logos' look rules are read by their test; the other styles stay out.
      css: { include: [/logos\.css/] },
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
