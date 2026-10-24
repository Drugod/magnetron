import { defineConfig } from "vite";

export default defineConfig({
  optimizeDeps: { entries: ["index.html"] },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/.toolchain/**", "**/artifacts/**"],
    },
  },
  clearScreen: false,
});
