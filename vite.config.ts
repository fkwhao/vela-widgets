import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  resolve: {
    // texmath's CommonJS fallback and our ESM import must share one KaTeX copy.
    alias: [{ find: /^katex$/, replacement: 'katex/dist/katex.mjs' }],
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
