# Compile rust backend
FROM rust:1.89.0-alpine AS backend-builder
RUN apk add --no-cache musl-dev
WORKDIR /usr/src/app
COPY pastedev-api/Cargo.toml pastedev-api/Cargo.lock ./
COPY pastedev-api/.sqlx ./.sqlx
COPY pastedev-api/migrations ./migrations
COPY pastedev-api/src ./src

RUN SQLX_OFFLINE=true cargo build --release

# Build web 
FROM node:24.7.0-alpine AS web-builder
WORKDIR /usr/src/app
COPY pastedev-frontend/package.json pastedev-frontend/yarn.lock ./
RUN yarn install --silent
COPY pastedev-frontend/ .
RUN yarn build 

# Assemble into the final image
FROM alpine:3
WORKDIR /app
EXPOSE 8080
COPY --from=backend-builder /usr/src/app/target/release/pastedev-api .
COPY --from=web-builder /usr/src/app/dist static
CMD ./pastedev-api