# paste.dev.su

A snippet host with admin-gated authoring and an API-first surface. Anyone can read by link; only approved users can publish — through the web editor or via an API key from a terminal or an agent.

See `plan/` for the full implementation plan (10 chapters of HTML you can open in a browser).

## Status

Phase 0 (scaffolding). Hello-world Axum + empty Vue SPA + Postgres in Docker Compose. Subsequent phases land per the roadmap in `plan/10-roadmap.html`.

## Quick start (local dev)

```sh
cp .env.example .env
# generate a real secret
echo "PASTEDEV_SECRET=$(openssl rand -base64 48)" >> .env

docker compose up
```

Then open <http://localhost:8080>. The fallback "boot shell" appears until the SPA is built; `GET /api/v1/health` should return `{ "ok": true, "db": "ok" }`.

For a tighter inner loop, run the DB in Docker and the app natively:

```sh
docker compose up -d db
cd web && pnpm install && pnpm run dev   # Vite at :5173
# in another shell:
DATABASE_URL=postgres://pastedev:pastedev@localhost:5432/pastedev \
  PASTEDEV_SECRET=$(openssl rand -base64 48) \
  cargo run -p pastedev-server             # axum at :8080
```

## Layout

```
crates/
  core/    # shared types (SnippetType, scopes, slug)
  server/  # pastedev-server binary (axum + sqlx)
  cli/     # pastedev-cli binary (clap + reqwest + MCP)
web/       # Vue 3 + Vite SPA, embedded into the server binary
plan/      # the full design + implementation plan
```

## Distributing `pastedev-cli`

The CLI is the part end users (and their agents) actually install on their
machines. The server is delivered as a Docker image; the CLI is delivered as
a prebuilt binary so that pointing Claude Desktop / Claude Code at it is a
one-line install. We use [cargo-dist](https://opensource.axo.dev/cargo-dist)
to generate the release pipeline.

### Operator: cutting a release

```sh
# bump the version
sed -i 's/^version = .*/version = "0.2.0"/' Cargo.toml
git commit -am 'pastedev-cli 0.2.0' && git tag pastedev-cli-v0.2.0 && git push --tags
```

The `pastedev-cli-v*` tag triggers `.github/workflows/pastedev-cli-release.yml`,
which builds:

* `x86_64-unknown-linux-musl` (any glibc / alpine / RHEL / Debian)
* `aarch64-unknown-linux-musl` (ARM Linux)
* `x86_64-apple-darwin` (Intel Mac)
* `aarch64-apple-darwin` (Apple Silicon)
* `x86_64-pc-windows-msvc` (Windows)

Linux binaries are statically linked via musl, so they run regardless of
the user's glibc version. Uploads tarballs + checksums + auto-generated
`install.sh` and `install.ps1` to GitHub Releases.

### End user: install

Linux / macOS:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/volandevovan/pastedev/releases/latest/download/pastedev-cli-installer.sh \
  | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/volandevovan/pastedev/releases/latest/download/pastedev-cli-installer.ps1 | iex"
```

If you've got Rust tooling already, `cargo binstall pastedev-cli` works once
the crate ships to crates.io.

### Wiring it into an MCP agent

After install, authenticate once against your instance:

```sh
pastedev-cli auth --base-url https://paste.example.com pds_live_...
```

Then drop the following into your agent's MCP config (e.g.
`~/.config/claude/mcp_servers.json` for Claude Desktop):

```json
{
  "mcpServers": {
    "pastedev": {
      "command": "pastedev-cli",
      "args": ["mcp"]
    }
  }
}
```

`pastedev-cli mcp` speaks JSON-RPC over stdio and exposes the
`pastedev_publish` / `pastedev_get` / `pastedev_list` / `pastedev_delete` /
`pastedev_whoami` tools, with `pastedev_publish_file` for letting the agent
upload bytes from disk without dragging them through its context window.

## License

MIT.
