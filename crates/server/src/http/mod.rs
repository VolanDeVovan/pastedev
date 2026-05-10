use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{assets, config::Config, db};

pub mod shell;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .with_state(state.clone());

    Router::new()
        .nest("/api/v1", api)
        .route("/assets/*path", get(assets::serve_asset))
        .fallback(get(serve_spa_shell))
        .with_state(state.clone())
        .layer(middleware::from_fn(add_request_id))
        .layer(middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http().make_span_with(
            |req: &Request<Body>| {
                let req_id = req
                    .headers()
                    .get("x-request-id")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("-")
                    .to_string();
                tracing::info_span!(
                    "http",
                    method = %req.method(),
                    uri = %req.uri(),
                    request_id = %req_id,
                )
            },
        ))
}

#[derive(serde::Serialize)]
struct HealthBody {
    ok: bool,
    db: &'static str,
}

async fn health(State(state): State<AppState>) -> Response {
    match db::ping(&state.pool).await {
        Ok(()) => (StatusCode::OK, Json(HealthBody { ok: true, db: "ok" })).into_response(),
        Err(e) => {
            tracing::warn!(error = ?e, "health: db ping failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthBody {
                    ok: false,
                    db: "err",
                }),
            )
                .into_response()
        }
    }
}

async fn serve_spa_shell(State(state): State<AppState>) -> Response {
    shell::render(&state.config).into_response()
}

/// Request-id middleware. Generates a v4 UUID if upstream didn't supply one,
/// and echoes it in the response.
async fn add_request_id(mut req: Request<Body>, next: middleware::Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok().map(String::from))
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    if let Ok(v) = HeaderValue::from_str(&id) {
        req.headers_mut().insert(HeaderName::from_static("x-request-id"), v.clone());
        let mut resp = next.run(req).await;
        resp.headers_mut().insert(HeaderName::from_static("x-request-id"), v);
        resp
    } else {
        next.run(req).await
    }
}

/// Adds the same baseline security headers to every response. The HTML view
/// route adds its own CSP (sandbox) on top, so we only set conservative defaults
/// here.
async fn security_headers(req: Request<Body>, next: middleware::Next) -> Response {
    let path = req.uri().path().to_string();
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    let is_sandboxed_html = path.starts_with("/h/") && path.ends_with("/raw");
    if !is_sandboxed_html
        && !headers.contains_key(header::CONTENT_SECURITY_POLICY)
    {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; \
                 script-src 'self'; \
                 style-src 'self' https://fonts.googleapis.com 'unsafe-inline'; \
                 font-src 'self' https://fonts.gstatic.com; \
                 img-src 'self' data:; \
                 connect-src 'self'; \
                 frame-src 'self'; \
                 frame-ancestors 'self'; \
                 base-uri 'none'; \
                 form-action 'self'",
            ),
        );
    }

    // X-Frame-Options is paired with frame-ancestors for older browsers.
    if !is_sandboxed_html && !headers.contains_key(HeaderName::from_static("x-frame-options")) {
        headers.insert(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("SAMEORIGIN"),
        );
    }

    resp
}

#[allow(dead_code)]
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
