//! Serves the SPA shell with a runtime config block injected before any other
//! script runs. The same binary boots in same-origin and split-origin deployments
//! because the config carries `apiBaseUrl`.

use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
};
use serde_json::json;

use crate::{assets, config::Config};

const FALLBACK_SHELL: &str = include_str!("./fallback_shell.html");

pub fn render(config: &Config) -> Response<Body> {
    let cfg_block = serde_json::to_string(&json!({
        "apiBaseUrl": config.api_base_url,
        "publicBaseUrl": config.public_base_url,
        "appName": config.app_name,
    }))
    .unwrap_or_else(|_| "{}".into());
    let injection = format!(
        "<script id=\"paste-config\" type=\"application/json\">{cfg}</script>",
        cfg = cfg_block
    );

    // Try the embedded `index.html` first; fall back to the dev shell if the SPA
    // hasn't been built. The fallback at least lets `/api/v1/*` curl-ing work
    // before `npm run build` has been run.
    let body_str = if let Some(file) = assets::get("index.html") {
        let content = String::from_utf8_lossy(&file.data);
        if let Some(idx) = content.find("</head>") {
            let mut out = String::with_capacity(content.len() + injection.len());
            out.push_str(&content[..idx]);
            out.push_str(&injection);
            out.push_str(&content[idx..]);
            out
        } else {
            content.into_owned()
        }
    } else {
        FALLBACK_SHELL.replace("<!-- INJECT_CONFIG -->", &injection)
    };

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
