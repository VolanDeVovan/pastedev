# syntax=docker/dockerfile:1

# stage 1 — build the SPA
FROM node:22-alpine AS web
RUN corepack enable
WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm run build

# stage 2 — build the Rust binary, embedding /web/dist
FROM rust:1.90-alpine AS rust
RUN apk add --no-cache musl-dev openssl-dev pkgconfig
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
COPY .sqlx/ ./.sqlx/
COPY --from=web /web/dist ./web/dist
ENV SQLX_OFFLINE=true
RUN cargo build --release -p pastedev-server

# stage 3 — runtime
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=rust /src/target/release/pastedev-server /usr/local/bin/pastedev-server
USER nobody
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/pastedev-server"]
