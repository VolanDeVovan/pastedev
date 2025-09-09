# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Frontend for PasteDev, a secure paste service. Built as a SPA with Svelte 5, TypeScript, and Tailwind CSS. Features anonymous one-time uploads and permanent storage for approved users.

## Stack

- **Frontend Framework**: Svelte 5 with TypeScript
- **Styling**: Tailwind CSS v4
- **Build Tool**: Vite with hot module reloading
- **Syntax Highlighting**: Shiki with Web Workers for performance
- **Virtualization**: @tanstack/svelte-virtual for large code snippets
- **Routing**: Custom lightweight router (src/lib/router.ts)

## Development Commands

```bash
# Build for production
yarn build

# Type checking and validation
yarn check
```

## Architecture

**Core Application (`src/`):**
- `main.ts`: Application entry point
- `App.svelte`: Root component with routing logic
- `style.css`: Global styles including dark theme and Shiki overrides

**API Layer (`src/lib/api.ts`):**
- `ApiClient`: Handles snippet creation and retrieval
- Connects to backend at `http://localhost:8080`
- Implements error handling with custom `ApiError` class

**Routing (`src/lib/router.ts`):**
- Custom SPA router using Svelte stores
- Routes: `/` (home), `/{snippet_id}` (snippet view)
- Browser history API integration

**Syntax Highlighting (`src/lib/highlighter.ts`, `src/workers/`):**
- Shiki-based syntax highlighting with automatic language detection
- Web Worker implementation for performance
- Chunked processing for large files

**Key Components:**
- `UploadForm.svelte`: Text input and submission interface
- `SnippetView.svelte`: Code display with syntax highlighting and virtualization

## Configuration

- **Vite**: Manual chunks for Shiki, ESNext target, history API fallback
- **TypeScript**: Strict mode with Svelte-specific configuration
- **Tailwind**: v4 with dark theme as default
- **Build**: Optimized for modern browsers with code splitting
