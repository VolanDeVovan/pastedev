set dotenv-load := true

# NOTE: backend SQL is compile-time-checked with `sqlx::query!` / `query_as!`.
# After changing any SQL string in the server crate, run `just prepare` (with
# the dev DB up) so the offline cache in `.sqlx/` stays in sync. CI and Docker
# builds rely on that cache — they don't have a live database to consult.

default:
    @just --list

# build SPA, then the Rust binary
build: build-web
    cargo build --release -p pastedev-server

build-web:
    cd web && pnpm install --frozen-lockfile && pnpm run build

# Dioxus SPA build (rust_frontend branch). Requires `dx` (Dioxus CLI 0.7.x) on
# PATH — install it via the Nix home module at parts/modules/home/common/dioxus.nix
# in ~/nixos, or `cargo install dioxus-cli --version 0.7.9` for non-Nix setups.
#
# `dx build` auto-detects crates/web/tailwind.css and runs the Tailwind v4 CLI
# itself, writing the compiled output to crates/web/assets/tailwind.css. We
# keep an unhashed copy in dist/assets/ so the static <link rel="stylesheet">
# in index.html resolves; dx's own hashed copy is included alongside for
# asset!()-based cache busting if we adopt it later.
build-dioxus:
    cd crates/web && dx build --release --platform web
    # dx spawns tailwindcss in --watch mode and exits before the watcher's
    # incremental output catches up to the latest source. Re-run it once
    # synchronously so the bundle is up-to-date with whatever was last edited.
    ~/.local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss \
        --input crates/web/tailwind.css \
        --output crates/web/assets/tailwind.css --minify
    rm -rf crates/web/dist/*
    cp -r target/dx/pastedev-web/release/web/public/* crates/web/dist/
    mkdir -p crates/web/dist/assets
    cp crates/web/assets/tailwind.css crates/web/dist/assets/tailwind.css
    # Vendored highlight.js + the hljs Web Worker — referenced from runtime
    # code (index.html / new Worker(...)), not via asset!() so dx doesn't ship
    # them automatically.
    cp crates/web/assets/highlight.min.js     crates/web/dist/assets/highlight.min.js
    cp crates/web/assets/highlight.worker.js  crates/web/dist/assets/highlight.worker.js

build-dioxus-server: build-dioxus
    cargo build --release -p pastedev-server --features dioxus-spa

# dev: vite on :5173 for HMR, pastedev-server on :8080. The browser hits Vite,
# Vite proxies /api/* + /c/m/h/* to the Rust server.
#
# Depends on db-up so a fresh `just dev` brings Postgres along with it.
#
# Processes (web, server, db-logs) run under mprocs — switch with arrow keys,
# restart one with `r`, quit with `q`. Env vars for the server live in
# mprocs.yaml; CORS_ALLOWED_ORIGINS there allow-lists the Vite origin so the
# server's origin-check middleware accepts state-changing requests forwarded
# through the proxy.
dev: build-web-dev db-up _ensure-mprocs
    .tools/bin/mprocs --config mprocs.yaml

# install mprocs into ./.tools (project-local, gitignored) if missing
_ensure-mprocs:
    @[ -x .tools/bin/mprocs ] || cargo install --root .tools --locked mprocs

build-web-dev:
    cd web && [ -d node_modules ] || pnpm install

test:
    cargo test --workspace

# stand up the DB only. --wait blocks until the container is healthy
# (compose.yml defines pg_isready as the healthcheck), so the next recipe
# that needs the pool won't race against startup.
db-up:
    docker compose up -d --wait db

# tear it all down
down:
    docker compose down

# run sqlx migrations against $DATABASE_URL
migrate:
    cargo sqlx migrate run --source crates/server/migrations

# create a new empty migration: just migrate-new add_foo
migrate-new NAME:
    cargo sqlx migrate add --source crates/server/migrations -r {{NAME}}

# regenerate `.sqlx/` after any change to server SQL. Requires the dev DB to
# be up (`just db-up`) and DATABASE_URL pointed at it.
prepare:
    cargo sqlx prepare --workspace -- -p pastedev-server
