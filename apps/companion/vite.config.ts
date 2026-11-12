import { defineConfig } from 'vite';
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: process.env.TAURI_DEV_HOST || '127.0.0.1' },
  build: { target: ['es2022', 'chrome105'] },
});
