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

**Configuration (`src/config.rs`):**
- Environment-based configuration using Clap
- Database URL, app URL, and server ports configurable via env vars
- Default postgres connection: `postgres://app:12345q@localhost:5432/app`

**Key Files:**
- `src/main.rs`: Application entry point with database connection and migrations
- `src/config.rs`: Configuration management via Clap
- `compose.yml`: PostgreSQL development container
- `migrations/`: Database schema migrations

## Environment Variables

- `APP_URL`: Application base URL for generating snippet links
- `HOST`: Bind address (default: 0.0.0.0)
- `HTTP_PORT`: HTTP server port (default: 8080)
- `SOCKET_PORT`: Socket server port (default: 9999)
- `POSTGRES_URL`: Database connection string
