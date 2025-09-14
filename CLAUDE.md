# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PasteDev is a secure paste service built as a monorepo with React frontend and Rust API. Features tiered access with anonymous one-time pastes and registered user accounts.

## Development Commands

### Frontend (pastedev-frontend/)
```bash
deno task build         # Build project. Use for testing
deno task check         # Biome check with auto-fix
deno task format        # Biome format with auto-fix
```

### API (pastedev-api/)
```bash
cargo run               # Run with migrations
cargo build             # Build project. Use for testing
cargo test              # Run tests
cargo check             # Quick compile check
```

## Stack

**Frontend**: React 19 + TailwindCSS v4 + TypeScript + Rsbuild (Biome for linting/formatting)
**API**: Rust + Axum + PostgreSQL + sqlx

## Architecture

- Monorepo with separate frontend/API in dedicated folders
- Frontend uses Rsbuild for bundling with React 19 and TailwindCSS v4
- Biome handles linting/formatting (single quotes, space indents, organize imports)
- API uses Axum with sqlx for database operations and automatic migrations
- Both projects have detailed CLAUDE.md files in their respective folders