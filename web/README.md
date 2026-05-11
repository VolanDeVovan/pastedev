# web

Vue 3 + Vite SPA. Bundled into `pastedev-server` at compile time via `rust-embed`.

## Scaffolding origin

This project was bootstrapped from the upstream `npm create vite@latest -- --template vue-ts` template, then Tailwind 4 was added per the [Vite install guide](https://tailwindcss.com/docs/installation/using-vite). Deviations from the upstream templates:

- `tailwind.config` is not used — Tailwind 4 reads design tokens from `@theme {}` blocks in `src/style.css`. Mint accent, navy panels, and the `mono` / `sans` font families are declared there to match `plan/design/`.
- `vite.config.ts` registers the `@tailwindcss/vite` plugin and adds a dev proxy from `/api`, `/c`, `/m`, `/h` → `http://localhost:8080` so the Rust server handles those routes during HMR.

That's it. Everything else is at upstream defaults.

## Runtime config

`index.html` (when served by `pastedev-server`) gets a `<script id="pastedev-config">` block injected before the SPA boots. `src/config.ts` reads it once and exposes `apiBaseUrl`, `publicBaseUrl`, `appName`. Same-origin deploys leave `apiBaseUrl` empty; split-origin deploys set it to the absolute API host.

## Build pipeline

`pnpm run build` → `dist/` → embedded into the Rust binary in release builds. In dev, run `vite` on :5173 and the Rust server on :8080; the Rust server detects `RUST_ENV=dev` and proxies non-API requests to Vite (planned for phase 5).
