set dotenv-load := true

default:
    @just --list

# build SPA, then the Rust binary
build: build-web
    cargo build --release -p paste-server

build-web:
    cd web && npm ci && npm run build

# dev: vite on :5173 for HMR, paste-server on :8080. The browser hits Vite,
# Vite proxies /api/* + /c/m/h/* to the Rust server.
#
# CORS_ALLOWED_ORIGINS allow-lists the Vite origin so the server's origin-check
# middleware accepts state-changing requests forwarded through the proxy.
# Without this the cookie travels but POST/PATCH/DELETE get a 403 forbidden.
dev: build-web-dev
    cd web && npm run dev &
    DATABASE_URL=${DATABASE_URL:-postgres://paste:paste@localhost:5432/paste} \
    PASTE_SECRET=${PASTE_SECRET:-dev-secret-replace-in-production-only-here-for-local} \
    SESSION_COOKIE_SECURE=false \
    CORS_ALLOWED_ORIGINS=http://localhost:5173 \
    RUST_ENV=dev \
        cargo run -p paste-server

build-web-dev:
    cd web && [ -d node_modules ] || npm install

test:
    cargo test --workspace

# stand up the DB only
db-up:
    docker compose up -d db

# tear it all down
down:
    docker compose down

# run sqlx migrations against $DATABASE_URL
migrate:
    cargo sqlx migrate run --source crates/server/migrations

# create a new empty migration: just migrate-new add_foo
migrate-new NAME:
    cargo sqlx migrate add --source crates/server/migrations -r {{NAME}}
