set dotenv-load := true

# NOTE: backend SQL is compile-time-checked with `sqlx::query!` / `query_as!`.
# After changing any SQL string in the server crate, run `just prepare` (with
# the dev DB up) so the offline cache in `.sqlx/` stays in sync. CI and Docker
# builds rely on that cache — they don't have a live database to consult.

default:
    @just --list

# Build the Dioxus SPA, then the server binary that embeds it.
build: build-web
    cargo build --release -p pastedev-server

# Build the Dioxus SPA. Requires `dx` (Dioxus CLI 0.7.x) on PATH — install via
# the Nix home module at parts/modules/home/common/dioxus.nix in ~/nixos, or
# `cargo install dioxus-cli --version 0.7.9` for non-Nix setups.
#
# `dx build` auto-detects crates/web/tailwind.css and runs the Tailwind v4 CLI
# itself, writing the compiled output to crates/web/assets/tailwind.css. We
# re-run tailwindcss synchronously after dx exits because dx spawns it in
# --watch mode and may exit before the watcher's incremental output catches
# up to the latest source.
build-web:
    cd crates/web && dx build --release --platform web
    ~/.local/share/.dx/tools/tailwindcss-v4.1.5/tailwindcss \
        --input crates/web/tailwind.css \
        --output crates/web/assets/tailwind.css --minify
    rm -rf crates/web/dist/*
    cp -r target/dx/pastedev-web/release/web/public/* crates/web/dist/
    # Static <link>'d Tailwind output + vendored highlight.js + the hljs Web
    # Worker. None go through asset!() so dx doesn't ship them automatically.
    mkdir -p crates/web/dist/assets
    cp crates/web/assets/tailwind.css         crates/web/dist/assets/tailwind.css
    cp crates/web/assets/highlight.min.js     crates/web/dist/assets/highlight.min.js
    cp crates/web/assets/highlight.worker.js  crates/web/dist/assets/highlight.worker.js

# dev: dx serve on :5173 with HMR, pastedev-server on :8080. The browser
# hits :5173; the dev-proxy in crates/web/Dioxus.toml forwards /api, /c, /m,
# /h to the server. CORS_ALLOWED_ORIGINS in mprocs.yaml allow-lists :5173 so
# state-changing requests through the proxy carry the session cookie.
#
# Processes run under mprocs — switch with arrow keys, restart with `r`, quit
# with `q`.
dev: db-up _ensure-mprocs
    .tools/bin/mprocs --config mprocs.yaml

# install mprocs into ./.tools (project-local, gitignored) if missing
_ensure-mprocs:
    @[ -x .tools/bin/mprocs ] || cargo install --root .tools --locked mprocs

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
