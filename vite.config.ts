import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// Tauri runs the dev server on a fixed port and serves `dist` in the app build.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'safari16', outDir: 'dist' },
  test: {
    environment: 'jsdom',
    // Fixture times read like the prototype (14:20) at UTC+7.
    env: { TZ: 'Asia/Bangkok' },
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
