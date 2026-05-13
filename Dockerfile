# syntax=docker/dockerfile:1
#
# Three-stage build for pastedev-server. The SPA is the Dioxus/WASM client
# in crates/web/; dx auto-runs Tailwind v4 and wasm-bindgen, then we copy
# the output (plus the vendored highlight.js + worker) into crates/web/dist/
# so the server's rust-embed macro can pick it up at compile time.

# ─── stage 1 · build the Dioxus SPA ─────────────────────────────────────────
FROM rust:1.90-alpine AS web
RUN apk add --no-cache musl-dev openssl-dev pkgconfig binaryen \
    && rustup target add wasm32-unknown-unknown
# Pin dioxus-cli + wasm-bindgen-cli to the same versions the Nix home module
# provides locally, so the wasm-bindgen version pin in crates/web/Cargo.toml
# is honored across both build paths.
RUN cargo install --locked dioxus-cli --version 0.7.9 \
    && cargo install --locked wasm-bindgen-cli --version 0.2.118
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
RUN cd crates/web && dx build --release --platform web
# dx writes the hashed bundle under target/dx/...; reassemble the layout the
# server's rust-embed expects (crates/web/dist/).
RUN rm -rf /src/crates/web/dist \
    && mkdir -p /src/crates/web/dist/assets \
    && cp -r /src/target/dx/pastedev-web/release/web/public/. /src/crates/web/dist/ \
    && cp /src/crates/web/assets/tailwind.css        /src/crates/web/dist/assets/tailwind.css \
    && cp /src/crates/web/assets/highlight.min.js    /src/crates/web/dist/assets/highlight.min.js \
    && cp /src/crates/web/assets/highlight.worker.js /src/crates/web/dist/assets/highlight.worker.js

# ─── stage 2 · build pastedev-server ────────────────────────────────────────
FROM rust:1.90-alpine AS rust
RUN apk add --no-cache musl-dev openssl-dev pkgconfig
WORKDIR /src
COPY --from=web /src ./
COPY .sqlx/ ./.sqlx/
ENV SQLX_OFFLINE=true
RUN cargo build --release -p pastedev-server

# ─── stage 3 · runtime ──────────────────────────────────────────────────────
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=rust /src/target/release/pastedev-server /usr/local/bin/pastedev-server
USER nobody
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/pastedev-server"]
