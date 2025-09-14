# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

React 19 frontend for PasteDev paste service. Single-page application with syntax highlighting and simple editing interface.

## Stack

- **Build Tool**: Rsbuild with React plugin
- **Styling**: TailwindCSS v4 with PostCSS integration
- **Linting/Formatting**: Biome (single quotes, space indents, organize imports)
- **Code Highlighting**: highlight.js via Web Worker for performance
- **Font Stack**: Fira Code and monospace fallbacks

## Development Commands

```bash
deno task build         # Build project. Use for testing
deno task check         # Biome check with auto-fix
deno task format        # Biome format with auto-fix
```

## Architecture

**Core Components:**
- `src/App.tsx`: Main app with state management and routing
- `src/components/Editor.tsx`: Textarea-based code editor with keyboard shortcuts
- `src/components/Viewer.tsx`: Code viewer with syntax highlighting and line numbers
- `src/components/Menu.tsx`: Top navigation with actions (save, new, edit, raw)

**State Management:**
- `src/hooks/useSnippetApp.ts`: Main app state hook with URL-based routing
- `src/hooks/useKeyboard.ts`: Global keyboard shortcuts (Ctrl+S, Escape, Ctrl+A)

**Services:**
- `src/services/snippetService.ts`: API communication with error handling
- `src/workers/highlight.worker.ts`: Web Worker for syntax highlighting performance

**App States:**
- `edit`: Textarea editor for creating/editing content
- `view`: Syntax-highlighted viewer with line numbers
- `loading`: Loading spinner during API calls

**Routing:**
- `/`: Editor mode for new snippets
- `/{alias}`: Viewer mode for existing snippets
- Client-side routing via History API

**Key Features:**
- Auto-focus on editor
- Keyboard shortcuts: Ctrl+S (save), Escape (home), Ctrl+A (select all in viewer)
- Error handling with temporary error messages
- Web Worker syntax highlighting for performance
- Responsive design with dark theme