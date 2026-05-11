# paste.dev.su

A snippet host with admin-gated authoring and an API-first surface. Anyone can read by link; only approved users can publish — through the web editor or via an API key from a terminal or an agent.

See `plan/` for the full implementation plan (10 chapters of HTML you can open in a browser).

## Status

Phase 0 (scaffolding). Hello-world Axum + empty Vue SPA + Postgres in Docker Compose. Subsequent phases land per the roadmap in `plan/10-roadmap.html`.

## Quick start (local dev)

```sh
cp .env.example .env
# generate a real secret
echo "PASTE_SECRET=$(openssl rand -base64 48)" >> .env

docker compose up
```

Then open <http://localhost:8080>. The fallback "boot shell" appears until the SPA is built; `GET /api/v1/health` should return `{ "ok": true, "db": "ok" }`.

For a tighter inner loop, run the DB in Docker and the app natively:

```sh
docker compose up -d db
cd web && pnpm install && pnpm run dev   # Vite at :5173
# in another shell:
DATABASE_URL=postgres://paste:paste@localhost:5432/paste \
  PASTE_SECRET=$(openssl rand -base64 48) \
  cargo run -p paste-server             # axum at :8080
```

## Layout

```
crates/
  core/    # shared types (SnippetType, scopes, slug)
  server/  # paste-server binary (axum + sqlx)
  cli/     # paste-cli binary (clap + reqwest + MCP)
web/       # Vue 3 + Vite SPA, embedded into the server binary
plan/      # the full design + implementation plan
```

## License

MIT.
