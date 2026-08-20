import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Port 1421 (not Tauri's default 1420) so this app and PC Tweaker can run
// `tauri dev` side by side on the same machine.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1421,
    strictPort: true,
    watch: {
      // Never watch the Rust build output: cargo rewrites the .exe while
      // vite tries to watch it, which crashes the dev server with EBUSY.
      // Same ignore the official Tauri template ships.
      ignored: ["**/src-tauri/**"],
    },
  },
});
