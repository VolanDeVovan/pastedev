import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  worker: {
    format: "es"
  },
  server: {
    // Enable history API fallback for SPA routing
    historyApiFallback: true,
  },
  preview: {
    // Enable history API fallback for preview mode as well
    historyApiFallback: true,
  },
  build: {
    target: "esnext",
    rollupOptions: {
      output: {
        manualChunks: {
          'shiki': ['shiki/bundle/web']
        }
      }
    }
  }
});
