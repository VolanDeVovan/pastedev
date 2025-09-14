# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PasteDev API is a secure paste service built with Rust, Axum, and PostgreSQL. The service implements a tiered access system:
- Anonymous uploads with one-time viewing and automatic deletion
- Registered users with admin approval for permanent snippets

## Stack

- **Backend**: Rust with Axum web framework
- **Database**: PostgreSQL with sqlx (no ORM)
- **Migrations**: sqlx migrations in `./migrations/`
- **Configuration**: Clap-based config with environment variables

## Development Commands

```bash
# Database setup
docker-compose up -d postgres  # Start PostgreSQL container

# Build and run
cargo build                    # Build the project
cargo run                      # Run with migrations
cargo check                    # Quick compile check
cargo test                     # Run tests

# Database operations
cargo sqlx migrate add <name>  # Create new migration
cargo sqlx migrate run         # Apply migrations
```

## Architecture

**Core Structure:**
- `src/main.rs`: Entry point with configuration, database setup, and server startup
- `src/routes.rs`: Axum routing with AppState, error handling, and static file serving
- `src/snippet.rs`: Core Snippet model with database operations and unique alias generation
- `src/cleanup.rs`: Background task for deleting expired snippets
- `src/lib.rs`: Module declarations

**API Endpoints:**
- `POST /api/snippets`: Create new snippet (body = content)
- `GET /api/snippets/{alias}`: Retrieve snippet by alias
- Static file serving from `static/` directory with SPA fallback

**Database Model:**
- `snippets` table with UUID primary key, unique 8-character alias, content, timestamps
- Ephemeral snippets auto-expire 15 minutes after first access
- Soft deletion with `deleted` boolean flag

**Configuration (via Clap + env vars):**
- `APP_URL`: Base URL for generating snippet links (required)
- `HOST`: Bind address (default: 0.0.0.0)
- `HTTP_PORT`: HTTP server port (default: 8080)
- `SOCKET_PORT`: Socket server port (default: 9999)
- `DATABASE_URL`: Postgres connection (default: postgres://app:12345q@localhost:5432/app)
