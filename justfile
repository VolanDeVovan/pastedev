set dotenv-load := true

default:
    @just --list

# build SPA, then the Rust binary
build: build-web
    cargo build --release -p paste-server

build-web:
    cd web && npm ci && npm run build

# dev: vite for HMR + cargo run with same DATABASE_URL the server defaults to
dev: build-web-dev
    cd web && npm run dev &
    DATABASE_URL=${DATABASE_URL:-postgres://paste:paste@localhost:5432/paste} \
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
