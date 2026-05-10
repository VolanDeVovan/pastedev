import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';

// During `npm run dev`, the dev server runs on :5173 and proxies /api to the
// Rust server on :8080. In production, the Rust server embeds /web/dist and
// serves it directly.
export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 5173,
    // Fail loudly if :5173 is already taken instead of silently falling back to
    // :5174 — `just dev` allow-lists the exact origin, so a drift breaks auth.
    strictPort: true,
    proxy: {
      '/api': 'http://localhost:8080',
      '/c': 'http://localhost:8080',
      '/m': 'http://localhost:8080',
      '/h': 'http://localhost:8080',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
