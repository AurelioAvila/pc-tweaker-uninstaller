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
  },
});
