# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

Self-hostable snippet/page/HTML-share service, shipped as two binaries that share a wire-format crate:

- **`pastedev-server`** — axum + sqlx + Postgres. Embeds the Vue SPA into its binary via `rust-embed`.
- **`pastedev-cli`** — clap + reqwest. `pastedev-cli mcp` runs a stdio MCP server exposing `pastedev_{publish,publish_file,get,list,edit,delete,whoami}` for agents.
- **`pastedev-core`** — shared request/response types. Wire format lives here; do not duplicate in server/CLI.

```
crates/{core,server,cli}/
web/                # Vue 3 SPA, built into web/dist/, embedded by the server
```

## Commands

- `just dev` — Postgres + Vite (:5173) + Rust server (:8080) under mprocs. Browser hits Vite; Vite proxies `/api/*` and `/{c,m,h}/:slug/raw` to the server.
- `just test` — `cargo test --workspace`.
- `just migrate` / `just migrate-new <name>` — sqlx migrations under `crates/server/migrations/`.
- `just prepare` — **run after any change to a `sqlx::query!` string.** Regenerates `.sqlx/`; CI and the Docker build use `SQLX_OFFLINE=true` and rely on this cache.

Web commands run from `web/` with pnpm.

## Architecture notes that aren't obvious from the tree

- **Snippet URL scheme**: `/c/:slug` code, `/m/:slug` markdown, `/h/:slug` html. The `/{c,m,h}/:slug/raw` variants are server-rendered; the bare paths are SPA views. `/h/*/raw` gets a sandbox CSP (see `http/mod.rs::security_headers`).
- **Auth has two paths into one extractor** (`auth/extract.rs`): cookie sessions for the SPA (origin-checked) and `pds_live_*` bearer tokens for the CLI/MCP (scope-gated via `RequiresScope<S>`).
- **Setup gate**: while `users` is empty, all `/api/v1/*` except `/setup/*` and `/health` return `403 setup_required`. Cached in-process for 60s, invalidated when the first admin is created.
- **Runtime config injection**: the server injects `<script id="pastedev-config">` into `index.html` so the same binary boots same-origin and split-origin deployments — the SPA reads its API base from there, not from a build-time constant.
