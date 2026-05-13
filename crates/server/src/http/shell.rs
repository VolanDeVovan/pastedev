//! Serves the SPA shell with a runtime config block injected before any other
//! script runs. The same binary boots in same-origin and split-origin deploys
//! because the config carries `apiBaseUrl`.
//!
//! When called with a [`SnippetMeta`], the shell also injects a per-snippet
//! `<title>` and OpenGraph/Twitter meta tags so that links unfurl with useful
//! previews on Telegram, Slack, etc. Non-snippet routes pass `None` and keep
//! the static `<title>pastedev</title>`.

use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
};
use serde_json::json;

use crate::{
    assets,
    config::Config,
    http::snippet_meta::SnippetMeta,
};

const FALLBACK_SHELL: &str = include_str!("./fallback_shell.html");

/// The static `<title>` Vite emits into `web/index.html`. When we have a
/// snippet-specific title to inject we strip this so the page doesn't ship
/// with two `<title>` elements.
const STATIC_TITLE: &str = "<title>pastedev</title>";

pub fn render(config: &Config, meta: Option<&SnippetMeta>) -> Response<Body> {
    let injection = build_injection(config, meta);
    let body_str = render_body(&injection, meta.is_some());

    let mut resp = Response::new(Body::from(body_str));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=0, must-revalidate"),
    );
    resp
}

fn build_injection(config: &Config, meta: Option<&SnippetMeta>) -> String {
    let cfg_block = serde_json::to_string(&json!({
        "apiBaseUrl": config.api_base_url,
        "publicBaseUrl": config.public_base_url,
        "appName": config.app_name,
    }))
    .unwrap_or_else(|_| "{}".into());

    let mut out = format!(
        "<script id=\"pastedev-config\" type=\"application/json\">{cfg_block}</script>"
    );
    if let Some(m) = meta {
        out.push_str(&m.to_head_html());
    }
    out
}

fn render_body(injection: &str, strip_static_title: bool) -> String {
    let Some(file) = assets::get("index.html") else {
        // SPA hasn't been built yet — fall back to a tiny boot page so the
        // API is still reachable. The fallback never sees per-snippet meta
        // (no slug-aware routes pass through here), so no title strip needed.
        return FALLBACK_SHELL.replace("<!-- INJECT_CONFIG -->", injection);
    };
    let mut content = String::from_utf8_lossy(&file.data).into_owned();
    if strip_static_title {
        content = content.replacen(STATIC_TITLE, "", 1);
    }
    match content.find("</head>") {
        Some(idx) => {
            let mut out = String::with_capacity(content.len() + injection.len());
            out.push_str(&content[..idx]);
            out.push_str(injection);
            out.push_str(&content[idx..]);
            out
        }
        None => content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_title_constant_matches_vite_output() {
        // If Vite ever changes its default <title>, we'd start emitting two
        // of them. The bin smoke-test that hits / catches that too, but this
        // is cheaper feedback.
        assert_eq!(STATIC_TITLE, "<title>pastedev</title>");
    }
}
