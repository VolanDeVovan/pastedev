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
