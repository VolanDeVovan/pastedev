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

// Embed the SPA. The `dioxus-spa` feature flag swaps the embed root to the
// new Dioxus dist; the default keeps the legacy Vue path working so a server
// build can succeed against a populated `web/dist/`.
#[cfg(not(feature = "dioxus-spa"))]
#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
#[exclude = ".gitkeep"]
pub struct Assets;

#[cfg(feature = "dioxus-spa")]
#[derive(RustEmbed)]
#[folder = "../web/dist/"]
#[exclude = ".gitkeep"]
pub struct Assets;

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
    // Hashed filenames (dx's `-dxh<hash>.{js,wasm,css}` naming) can be cached
    // forever; everything else still needs to be re-validated so server-side
    // updates take effect on the next page load instead of waiting out the
    // browser cache.
    let immutable = path
        .rsplit_once('.')
        .and_then(|(stem, _)| stem.rsplit_once("-dxh").map(|(_, h)| h))
        .is_some_and(|h| h.len() >= 8 && h.chars().all(|c| c.is_ascii_alphanumeric()));
    let cc = if immutable {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=0, must-revalidate"
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static(cc));
    Some(response)
}

/// Looks up `/<path>` requests under the embedded SPA bundle, stripping the
/// leading slash so it matches the embed root.
pub async fn serve_asset(uri: Uri) -> Response {
    let raw = uri.path();
    let path = raw.trim_start_matches('/');
    if let Some(resp) = asset_response(path) {
        return resp;
    }
    (StatusCode::NOT_FOUND, "asset not found").into_response()
}
