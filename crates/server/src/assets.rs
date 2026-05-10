//! Embedded SPA bundle. In release builds, `rust-embed` reads `web/dist/` at
//! compile time. In dev (`RUST_ENV=dev`), the asset folder may be empty — the
//! HTTP layer falls back to a "boot shell" placeholder so the dev loop still
//! works without running `npm run build` first.

use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
#[exclude = ".gitkeep"]
pub struct Assets;

pub fn has_built_spa() -> bool {
    Assets::get("index.html").is_some()
}

pub fn get(path: &str) -> Option<rust_embed::EmbeddedFile> {
    Assets::get(path)
}

pub fn asset_response(path: &str) -> Option<Response> {
    let file = get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut response = Response::new(Body::from(file.data.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Some(response)
}

/// Looks up `/assets/<path>` requests under the embedded SPA bundle.
pub async fn serve_asset(uri: Uri) -> Response {
    let raw = uri.path();
    let path = raw.trim_start_matches('/');
    if let Some(resp) = asset_response(path) {
        return resp;
    }
    (StatusCode::NOT_FOUND, "asset not found").into_response()
}
