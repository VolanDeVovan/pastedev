# syntax=docker/dockerfile:1

# stage 1 — build the SPA
FROM node:22-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json* ./
RUN if [ -f package-lock.json ]; then npm ci; else npm install; fi
COPY web/ ./
RUN npm run build

# stage 2 — build the Rust binary, embedding /web/dist
FROM rust:1.85-alpine AS rust
RUN apk add --no-cache musl-dev openssl-dev pkgconfig
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
COPY --from=web /web/dist ./web/dist
ENV SQLX_OFFLINE=true
RUN cargo build --release -p paste-server

# stage 3 — runtime
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=rust /src/target/release/paste-server /usr/local/bin/paste-server
USER nobody
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/paste-server"]
