import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite is only used to build the static frontend that Tauri embeds.
// The dev server port is fixed because tauri.conf.json points at it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/server/**"] },
  },
  build: {
    // WebView2 on Windows 11 is evergreen Chromium, so we can target modern JS.
    target: "chrome110",
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
  },
});
