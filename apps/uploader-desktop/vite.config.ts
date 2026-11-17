import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// TAURI_DEV_HOST задается при отладке на мобильном устройстве
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // Tauri сам рисует окно: не очищаем его вывод в терминале
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // Rust-код пересобирает Tauri, Vite за ним не следит
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
