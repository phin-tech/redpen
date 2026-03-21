import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/svelte/
export default defineConfig(async () => ({
  plugins: [svelte()],

  resolve: {
    alias: {
      $lib: path.resolve("./src/lib"),
    },
  },

  // Vite options tailored for Tauri development
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
