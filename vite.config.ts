import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const API = "https://pc-tweaker-app-production.up.railway.app";

// Port 1421 (not Tauri's default 1420) so this app and PC Tweaker can run
// `tauri dev` side by side on the same machine.
//
// `--mode devprofile` (npm run dev:profile, used by src-tauri/tauri.dev.conf.json)
// is the isolated development profile: port 1431 (PC Tweaker's dev profile
// holds 1430), and API calls go through this dev server, because the
// backend's CORS list only admits the installed app's origin
// (tauri.localhost), 1420 and 5173. Release builds never see any of it.
export default defineConfig(({ mode }) => {
  const profile = mode === "devprofile";
  const port = profile ? 1431 : 1421;
  return {
    plugins: [react()],
    clearScreen: false,
    define: profile
      ? { "import.meta.env.VITE_API_BASE_URL": JSON.stringify(`http://localhost:${String(port)}`) }
      : {},
    server: {
      port,
      strictPort: true,
      proxy: profile ? { "/api": { target: API, changeOrigin: true } } : undefined,
      watch: {
        // Never watch the Rust build output: cargo rewrites the .exe while
        // vite tries to watch it, which crashes the dev server with EBUSY.
        // Same ignore the official Tauri template ships.
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
