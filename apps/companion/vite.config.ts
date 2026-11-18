import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

const entry = (filename: string) => fileURLToPath(new URL(filename, import.meta.url));
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: process.env.TAURI_DEV_HOST || '127.0.0.1' },
  build: {
    target: ['es2022', 'chrome105'],
    rolldownOptions: {
      input: process.env.TAURI_ENV_PLATFORM === 'android'
        ? { main: entry('./index.html') }
        : { main: entry('./index.html'), pet: entry('./pet.html') },
    },
  },
});
