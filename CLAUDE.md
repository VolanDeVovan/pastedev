# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

Self-hostable snippet/page/HTML-share service, shipped as two binaries that share a wire-format crate:

- **`pastedev-server`** — axum + sqlx + Postgres. Embeds the Dioxus SPA into its binary via `rust-embed`.
- **`pastedev-cli`** — clap + reqwest. `pastedev-cli mcp` runs a stdio MCP server exposing `pastedev_{publish,publish_file,get,list,edit,delete,whoami}` for agents.
- **`pastedev-core`** — shared request/response types used by server, CLI, and SPA. Wire format lives here; do not duplicate.
- **`pastedev-web`** — Dioxus 0.7 / WASM client. `dx build` produces a hashed bundle under `target/dx/...`; `just build-web` copies it into `crates/web/dist/` for the server to embed.

```
crates/{core,server,cli,web}/
```

## Commands

- `just dev` — Postgres + `dx serve` (:5173) + Rust server (:8080) under mprocs. Browser hits `:5173`; the dev proxy in `crates/web/Dioxus.toml` forwards `/api/*` and `/{c,m,h}/:slug/raw` to the server.
- `just build` — runs `dx build --release` for the SPA, then `cargo build --release -p pastedev-server`.
- `just build-web` — SPA only.
- `just test` — `cargo test --workspace`.
- `just migrate` / `just migrate-new <name>` — sqlx migrations under `crates/server/migrations/`.
- `just prepare` — **run after any change to a `sqlx::query!` string.** Regenerates `.sqlx/`; CI and the Docker build use `SQLX_OFFLINE=true` and rely on this cache.

`dx` (Dioxus CLI 0.7.x) + `binaryen` (for `wasm-opt`) come from the Nix home module at `~/nixos/parts/modules/home/common/dioxus.nix`. For non-Nix setups: `cargo install dioxus-cli --version 0.7.9` and have `wasm-opt` on PATH.

## Architecture notes that aren't obvious from the tree

- **Snippet URL scheme**: `/c/:slug` code, `/m/:slug` markdown, `/h/:slug` html. The `/{c,m,h}/:slug/raw` variants are server-rendered; the bare paths are SPA views. `/h/*/raw` gets a sandbox CSP (see `http/mod.rs::security_headers`).
- **Auth has two paths into one extractor** (`auth/extract.rs`): cookie sessions for the SPA (origin-checked) and `pds_live_*` bearer tokens for the CLI/MCP (scope-gated via `RequiresScope<S>`).
- **Setup gate**: while `users` is empty, all `/api/v1/*` except `/setup/*` and `/health` return `403 setup_required`. Cached in-process for 60s, invalidated when the first admin is created.
- **Runtime config injection**: the server injects `<script id="pastedev-config">` into `index.html` so the same binary boots same-origin and split-origin deployments — the SPA reads its API base from there, not from a build-time constant.
- **wasm-bindgen pin**: `crates/web/Cargo.toml` pins `wasm-bindgen = "=0.2.118"` to match the version nixpkgs' `dioxus-cli` wraps on PATH. If you bump Dioxus, re-check what version the new `dx` wraps (`cat $(which dx) | grep wasm-bindgen`) and bump both together.
- **Editor overlay**: `crates/web/src/editor/mod.rs` paints a synchronous `paint_overlay()` on every keystroke (append/backspace/middle-edit fast paths) while the hljs Web Worker reply replaces the cache 150 ms later. Truncation logic in `truncate_html_to_plain_bytes` keeps existing colours on shrink.
