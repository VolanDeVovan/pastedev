# pastedev

A self-hostable service for sharing pieces of code, Markdown pages, and standalone HTML files behind short links. Read access is public; publishing is gated.

The idea grew out of a very specific itch: when an agent generates a document — a report, a note, a small one-page tool, an HTML mock — there is no good way to actually share that with another human. Pasting walls of Markdown or HTML into a chat is awful. `pastedev` exists so the agent (or you) can push the artifact to a URL in one call, and the recipient just opens a link.

That's why the project ships as two parts:

- **`pastedev-server`** — the hosted side. Renders snippets, runs the editor, owns auth.
- **`pastedev-cli`** — a terminal client with a built-in MCP server. Drop it into Claude Desktop / Claude Code / any MCP-aware agent and "share this as a link" becomes a tool call.

## Install the CLI

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

Linux builds are statically linked (musl), so they run on any distro without glibc surprises.

### Authenticate

```sh
pastedev-cli auth --base-url https://paste.example.com pds_live_...
```

### Wire it into an MCP agent

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

The MCP server exposes `pastedev_publish`, `pastedev_publish_file`, `pastedev_get`, `pastedev_list`, `pastedev_delete`, and `pastedev_whoami` — enough for an agent to publish a result, hand back a link, and clean up after itself later.

## Self-hosting

The server ships as a Docker image and expects a Postgres. `compose.yml` is the reference setup; from there the deployment story is yours to shape — bare VM, Fly, k8s, whatever fits. The only hard requirement is a stable public URL so the short links resolve.

## Layout

```
crates/
  core/    # shared types
  server/  # pastedev-server (axum + sqlx)
  cli/     # pastedev-cli (clap + reqwest + MCP)
web/       # Vue 3 SPA, embedded into the server binary
```

## License

MIT.
