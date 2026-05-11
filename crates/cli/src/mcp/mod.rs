//! Minimal MCP server over stdio JSON-RPC.
//!
//! Implements three methods:
//! - `initialize`: returns server identity + capabilities (`tools`).
//! - `tools/list`: returns the static tool registry.
//! - `tools/call`: dispatches a tool name + args to the API client.
//!
//! This isn't a complete MCP implementation (no `notifications/initialized`
//! handling beyond no-op, no resource subscription, no logging API), but it
//! is enough for an agent host like Claude Desktop to discover and call the
//! `pastedev_*` tools.

use std::io::Write;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::client::ApiClient;
use crate::credentials::resolve;
use pastedev_core::{CreateSnippetRequest, PatchSnippetRequest, SnippetType};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub async fn run() -> Result<()> {
    let stderr_log = format!(
        "pastedev-cli mcp {} starting on stdio",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("{stderr_log}");

    // Resolve creds lazily — auth-less listing still works because tool calls
    // will fail with the API's 401 envelope.
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("mcp: parse error: {e}; line: {trimmed}");
                continue;
            }
        };
        let reply = dispatch(req).await;
        if let Some(reply) = reply {
            let mut out = std::io::stdout().lock();
            let body = serde_json::to_string(&reply)?;
            writeln!(out, "{body}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

async fn dispatch(req: JsonRpcRequest) -> Option<Value> {
    if req.jsonrpc != "2.0" {
        return req.id.map(|id| error_reply(id, -32_600, "invalid jsonrpc version"));
    }
    let is_notification = req.id.is_none();
    let result = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "pastedev-cli", "version": env!("CARGO_PKG_VERSION") },
        })),
        "notifications/initialized" => return None,
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => handle_tool_call(req.params).await,
        _ => Err((-32_601_i32, format!("method not found: {}", req.method))),
    };
    if is_notification {
        return None;
    }
    let id = req.id.unwrap_or(Value::Null);
    match result {
        Ok(v) => Some(json!({ "jsonrpc": "2.0", "id": id, "result": v })),
        Err((code, message)) => Some(error_reply(id, code, &message)),
    }
}

fn error_reply(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "pastedev_whoami",
            "description": "Return the current identity (username, role, status).",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": "pastedev_publish",
            "description": "Create a new snippet with an in-memory body.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["code", "markdown", "html"] },
                    "body": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["type", "body"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false },
        }),
        json!({
            "name": "pastedev_publish_file",
            "description": "Create a snippet whose body is read from a local file path. \
                            Type is inferred from extension if not provided.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "type": { "type": "string", "enum": ["code", "markdown", "html"] },
                    "name": { "type": "string" }
                },
                "required": ["file_path"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false },
        }),
        json!({
            "name": "pastedev_get",
            "description": "Fetch a snippet by slug.",
            "inputSchema": {
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": "pastedev_list",
            "description": "List the caller's snippets, optionally filtered by type.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "enum": ["code", "markdown", "html"] },
                    "cursor": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 }
                },
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": "pastedev_edit",
            "description": "Edit an existing snippet by slug. Provide `body` to replace the \
                            content and/or `name` to rename (empty string clears the name). \
                            At least one of `body` or `name` must be set.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slug": { "type": "string" },
                    "body": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["slug"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": false },
        }),
        json!({
            "name": "pastedev_delete",
            "description": "Delete a snippet by slug. Destructive — host should confirm.",
            "inputSchema": {
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"],
                "additionalProperties": false
            },
            "annotations": { "readOnlyHint": false, "destructiveHint": true },
        }),
    ]
}

async fn handle_tool_call(params: Value) -> Result<Value, (i32, String)> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or((-32_602, "tools/call.name is required".into()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let client = build_client().map_err(|e| (-32_000, e.to_string()))?;
    let result = match name {
        "pastedev_whoami" => call_whoami(&client).await,
        "pastedev_publish" => call_publish(&client, args).await,
        "pastedev_publish_file" => call_publish_file(&client, args).await,
        "pastedev_get" => call_get(&client, args).await,
        "pastedev_list" => call_list(&client, args).await,
        "pastedev_edit" => call_edit(&client, args).await,
        "pastedev_delete" => call_delete(&client, args).await,
        other => Err(anyhow!("unknown tool: {other}")),
    };
    match result {
        Ok(v) => Ok(json!({
            "content": [
                { "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() }
            ]
        })),
        Err(e) => Ok(json!({
            "isError": true,
            "content": [{ "type": "text", "text": e.to_string() }]
        })),
    }
}

fn build_client() -> Result<ApiClient> {
    let (token, base_url, _) = resolve(None, None)?;
    ApiClient::new(base_url, token)
}

async fn call_whoami(client: &ApiClient) -> Result<Value> {
    let me = client.me().await?;
    Ok(serde_json::to_value(&me)?)
}

async fn call_publish(client: &ApiClient, args: Value) -> Result<Value> {
    let kind_str = args.get("type").and_then(|v| v.as_str()).context("type is required")?;
    let kind = kind_str.parse::<SnippetType>().map_err(|_| anyhow!("invalid type"))?;
    let body = args.get("body").and_then(|v| v.as_str()).context("body is required")?;
    let name = args.get("name").and_then(|v| v.as_str()).map(String::from);
    let snippet = client
        .create_snippet(&CreateSnippetRequest {
            kind,
            name,
            body: body.to_string(),
        })
        .await?;
    Ok(serde_json::to_value(&snippet)?)
}

async fn call_publish_file(client: &ApiClient, args: Value) -> Result<Value> {
    let path_str = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .context("file_path is required")?;
    let path = std::path::PathBuf::from(path_str);
    let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    if bytes.len() > pastedev_core::MAX_SNIPPET_BYTES {
        return Err(anyhow!(
            "{} is {} bytes, max is {}",
            path.display(),
            bytes.len(),
            pastedev_core::MAX_SNIPPET_BYTES
        ));
    }
    let body = String::from_utf8(bytes).context("file is not valid UTF-8")?;
    let kind = args
        .get("type")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<SnippetType>().ok())
        .or_else(|| infer_kind(&path))
        .unwrap_or(SnippetType::Code);
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| path.file_name().and_then(|n| n.to_str()).map(String::from));
    let snippet = client
        .create_snippet(&CreateSnippetRequest { kind, name, body })
        .await?;
    Ok(serde_json::to_value(&snippet)?)
}

fn infer_kind(path: &std::path::Path) -> Option<SnippetType> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" => Some(SnippetType::Markdown),
        "html" | "htm" => Some(SnippetType::Html),
        _ => Some(SnippetType::Code),
    }
}

async fn call_get(client: &ApiClient, args: Value) -> Result<Value> {
    let slug = args.get("slug").and_then(|v| v.as_str()).context("slug is required")?;
    let snippet = client.get_snippet(slug).await?;
    Ok(serde_json::to_value(&snippet)?)
}

async fn call_list(client: &ApiClient, args: Value) -> Result<Value> {
    let kind = args
        .get("type")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<SnippetType>().ok());
    let cursor = args.get("cursor").and_then(|v| v.as_str()).map(String::from);
    let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
    let list = client.list_snippets(kind, cursor.as_deref(), limit).await?;
    Ok(serde_json::to_value(&list)?)
}

async fn call_edit(client: &ApiClient, args: Value) -> Result<Value> {
    let slug = args.get("slug").and_then(|v| v.as_str()).context("slug is required")?;
    let body = args.get("body").and_then(|v| v.as_str()).map(String::from);
    let name = args.get("name").and_then(|v| v.as_str()).map(String::from);
    if body.is_none() && name.is_none() {
        return Err(anyhow!("at least one of `body` or `name` must be provided"));
    }
    let patch = PatchSnippetRequest { body, name };
    let snippet = client.update_snippet(slug, &patch).await?;
    Ok(serde_json::to_value(&snippet)?)
}

async fn call_delete(client: &ApiClient, args: Value) -> Result<Value> {
    let slug = args.get("slug").and_then(|v| v.as_str()).context("slug is required")?;
    client.delete_snippet(slug).await?;
    Ok(json!({ "slug": slug, "deleted": true }))
}
