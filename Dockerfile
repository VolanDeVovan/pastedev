# Compile rust backend
FROM rust:1.61.0-alpine as backend-builder
RUN apk add --no-cache musl-dev
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Build web 
FROM node:lts-alpine as web-builder
WORKDIR /usr/src/app
COPY web/package.json web/yarn.lock ./
RUN yarn install --silent
COPY web/ .
RUN yarn build 

# Assemble into the final image
FROM alpine:3
WORKDIR /app
COPY --from=backend-builder /usr/src/app/target/release/pastedev .
COPY --from=web-builder /usr/src/app/dist static
CMD ./pastedev
