# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Monorepo with React frontend (TailwindCSS) and Rust API (Axum + PostgreSQL). Frontend uses Deno for development and build tooling.

## Development Commands

### Frontend (pastedev-frontend/)
```bash
deno task build         # Build project. Use for testing
deno task check         # Biome check with auto-fix
deno task format        # Biome format with auto-fix
```

### API (pastedev-api/)
```bash
cargo run             # Run with migrations
cargo build           # Build project. Use for testing
```

## Stack

**Frontend**: React 19 + TailwindCSS v4 + TypeScript (Deno tooling + Biome)
**API**: Rust + Axum + PostgreSQL + sqlx

## Architecture

- Monorepo structure with separate frontend/API folders
- Frontend uses Deno runtime with React and TailwindCSS v4
- Biome handles linting/formatting for frontend (single quotes, space indents)
- API follows existing Rust/Axum patterns documented in pastedev-api/CLAUDE.md