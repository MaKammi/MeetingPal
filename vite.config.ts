import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  optimizeDeps: { entries: ["index.html"] },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/vendor/**", "**/third-party/**", "**/runtime-data/**"],
    },
  },
  clearScreen: false,
  test: { environment: "node", include: ["src/**/*.test.ts"] },
});
