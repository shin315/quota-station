import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// Tauri runs the dev server on a fixed port and serves `dist` in the app build.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // `@/`: the source folder, as shadcn/ui's components import it.
  resolve: { alias: { '@': decodeURIComponent(new URL('./src', import.meta.url).pathname) } },
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'safari16', outDir: 'dist' },
  test: {
    environment: 'jsdom',
    // Fixture times read like the prototype (14:20) at UTC+7.
    env: { TZ: 'Asia/Bangkok' },
    include: ['src/**/*.test.{ts,tsx}'],
    // The logos' look rules are read by their test; the other styles stay out.
    css: { include: [/logos\.css/] },
  },
});
