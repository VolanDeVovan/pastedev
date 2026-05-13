# syntax=docker/dockerfile:1
#
# Three-stage build for pastedev-server. The default `SPA_FLAVOR=dioxus` builds
# the Rust/WASM SPA; pass `--build-arg SPA_FLAVOR=vue` to fall back to the
# legacy Vue tree until phase 4 of the migration removes that path.
#
# Phase 4 will collapse this into a single Rust-only build pipeline.

ARG SPA_FLAVOR=dioxus

# ─── stage 1a · Vue SPA (legacy path) ───────────────────────────────────────
FROM node:22-alpine AS web-vue
RUN corepack enable
WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm run build
RUN mkdir -p /spa-out && cp -r dist/. /spa-out/

# ─── stage 1b · Dioxus SPA ──────────────────────────────────────────────────
# dx auto-runs the Tailwind CLI and bundles the WASM. nixpkgs / a `nix shell`
# is the cleanest way to get a matching dioxus-cli + wasm-bindgen-cli pair;
# for a vanilla Docker build we use `cargo install` and accept the slower
# first-stage cold start in exchange for not pulling Nix in.
FROM rust:1.90-alpine AS web-dioxus
RUN apk add --no-cache musl-dev openssl-dev pkgconfig binaryen \
    && rustup target add wasm32-unknown-unknown
# Install the same dioxus-cli + wasm-bindgen-cli pair the local Nix module
# provides so the wasm-bindgen version pin in crates/web/Cargo.toml is honored.
RUN cargo install --locked dioxus-cli --version 0.7.9 \
    && cargo install --locked wasm-bindgen-cli --version 0.2.118
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
RUN cd crates/web && dx build --release --platform web
RUN mkdir -p /spa-out \
    && cp -r /src/target/dx/pastedev-web/release/web/public/. /spa-out/ \
    && mkdir -p /spa-out/assets \
    && cp /src/crates/web/assets/tailwind.css /spa-out/assets/tailwind.css

# ─── stage 1 · select flavor ────────────────────────────────────────────────
FROM web-${SPA_FLAVOR} AS web

# ─── stage 2 · build pastedev-server ────────────────────────────────────────
FROM rust:1.90-alpine AS rust
RUN apk add --no-cache musl-dev openssl-dev pkgconfig
ARG SPA_FLAVOR=dioxus
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
COPY .sqlx/ ./.sqlx/
# rust-embed needs the dist tree on disk at the path its #[folder] points at,
# selected by the dioxus-spa feature flag. Lay it out for both flavors so the
# same Cargo.toml works either way.
COPY --from=web /spa-out /spa/dist
RUN if [ "$SPA_FLAVOR" = "dioxus" ]; then \
        mkdir -p ./crates/web/dist && cp -r /spa/dist/. ./crates/web/dist/; \
    else \
        mkdir -p ./web/dist && cp -r /spa/dist/. ./web/dist/; \
    fi
ENV SQLX_OFFLINE=true
RUN if [ "$SPA_FLAVOR" = "dioxus" ]; then \
        cargo build --release -p pastedev-server --features dioxus-spa; \
    else \
        cargo build --release -p pastedev-server; \
    fi

# ─── stage 3 · runtime ──────────────────────────────────────────────────────
FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=rust /src/target/release/pastedev-server /usr/local/bin/pastedev-server
USER nobody
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/pastedev-server"]
