use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderName, HeaderValue, Method, Request, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::{
    api_keys::handlers as key_handlers,
    assets,
    auth::extract::SessionUser,
    config::Config,
    db,
    error::AppError,
    setup::{self, SetupGate},
    snippets::handlers as snippet_handlers,
    users::{admin as user_admin, handlers as user_handlers},
};

pub mod rate_limit;
pub mod shell;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub setup_gate: Arc<SetupGate>,
}

pub fn router(state: AppState) -> Router {
    let api_setup = Router::new()
        .route("/setup/status", get(setup::status))
        .route("/setup/admin", post(setup::create_first_admin))
        .with_state(state.clone());

    let api_auth = Router::new()
        .route(
            "/auth/register",
            post(user_handlers::register).layer(rate_limit::for_register()),
        )
        .route(
            "/auth/login",
            post(user_handlers::login).layer(rate_limit::for_login()),
        )
        .route("/auth/logout", post(user_handlers::logout))
        .route("/auth/me", get(user_handlers::me))
        .with_state(state.clone());

    // The 1 MB limit is also enforced in the snippet handler + DB CHECK; the
    // tower layer rejects oversized bodies before they're buffered.
    let api_snippets = Router::new()
        .route(
            "/snippets",
            post(snippet_handlers::create)
                .layer(rate_limit::for_create_snippet())
                .get(snippet_handlers::list),
        )
        .route(
            "/snippets/{slug}",
            get(snippet_handlers::get)
                .layer(rate_limit::for_read_snippet())
                .patch(snippet_handlers::patch)
                .delete(snippet_handlers::delete),
        )
        .layer(RequestBodyLimitLayer::new(state.config.snippet_max_bytes + 4096))
        .with_state(state.clone());

    let api_keys = Router::new()
        .route("/keys", post(key_handlers::create).get(key_handlers::list))
        .route("/keys/{id}", axum::routing::delete(key_handlers::revoke))
        .with_state(state.clone());

    let api_admin = Router::new()
        .route("/admin/users", get(user_admin::list_users))
        .route("/admin/users/{id}/approve", post(user_admin::approve))
        .route("/admin/users/{id}/reject", post(user_admin::reject))
        .route("/admin/users/{id}/suspend", post(user_admin::suspend))
        .route("/admin/users/{id}/restore", post(user_admin::restore))
        .route("/admin/users/{id}/promote", post(user_admin::promote))
        .route("/admin/users/{id}/demote", post(user_admin::demote))
        .route(
            "/admin/users/{id}/reset_password",
            post(user_admin::reset_password),
        )
        .with_state(state.clone());

    // Health is always on; setup-gate middleware below skips it.
    let api_misc = Router::new()
        .route("/health", get(health))
        .with_state(state.clone());

    let api = api_misc
        .merge(api_setup)
        .merge(api_auth)
        .merge(api_snippets)
        .merge(api_keys)
        .merge(api_admin)
        .layer(middleware::from_fn_with_state(state.clone(), setup_gate_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), origin_check_middleware));

    let raw_routes = Router::new()
        .route("/c/{slug}/raw", get(snippet_handlers::raw_text))
        .route("/m/{slug}/raw", get(snippet_handlers::raw_text))
        .route(
            "/h/{slug}/raw",
            get(snippet_handlers::raw_html).layer(rate_limit::for_html_raw()),
        )
        .with_state(state.clone());

    Router::new()
        .nest("/api/v1", api)
        .merge(raw_routes)
        .route("/assets/{*path}", get(assets::serve_asset))
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

/// Returns `403 setup_required` for all `/api/v1/*` routes except `/setup/*`
/// and `/health` while the users table is empty.
async fn setup_gate_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: middleware::Next,
) -> Response {
    let path = req.uri().path();
    let exempt = path.starts_with("/setup/") || path.starts_with("/api/v1/setup/")
        || path == "/health"
        || path == "/api/v1/health";
    if exempt {
        return next.run(req).await;
    }
    if state.setup_gate.needs_setup(&state.pool).await {
        return AppError::SetupRequired.into_response();
    }
    next.run(req).await
}

/// Origin allow-list check for session-authenticated state-changing requests.
/// Bearer-auth (phase 4) skips this entirely. Same-origin: Origin must match the
/// app's own host. Split-origin: Origin must be in `CORS_ALLOWED_ORIGINS`.
async fn origin_check_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: middleware::Next,
) -> Response {
    let method = req.method();
    let is_state_changing = matches!(
        *method,
        Method::POST | Method::PATCH | Method::DELETE | Method::PUT
    );
    if !is_state_changing {
        return next.run(req).await;
    }
    // If a bearer is supplied, skip this check.
    if req.headers().get("authorization").is_some() {
        return next.run(req).await;
    }
    // If there's no session cookie, we'll fail later with 401; let it through.
    let has_session_cookie = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|raw| {
            raw.split(';')
                .any(|p| p.trim().starts_with(&format!("{}=", paste_core::SESSION_COOKIE_NAME)))
        })
        .unwrap_or(false);
    if !has_session_cookie {
        return next.run(req).await;
    }

    let allow_same = matches_self_origin(req.headers(), &state.config.public_base_url);
    let allow_listed = matches_allowed_origin(req.headers(), &state.config.cors_allowed_origins);
    if allow_same || allow_listed {
        return next.run(req).await;
    }

    // Missing or mismatched Origin on a session-authed state-changing call.
    AppError::ForbiddenWith("origin not allowed").into_response()
}

fn matches_self_origin(headers: &axum::http::HeaderMap, public_base_url: &str) -> bool {
    let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    if public_base_url.is_empty() {
        return false;
    }
    same_origin(origin, public_base_url)
}

fn matches_allowed_origin(headers: &axum::http::HeaderMap, allowed: &[String]) -> bool {
    let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    allowed.iter().any(|a| same_origin(origin, a))
}

fn same_origin(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> Option<(String, Option<u16>)> {
        let parsed = url::Url::parse(s).ok()?;
        let scheme = parsed.scheme().to_string();
        let host = parsed.host_str()?.to_string();
        let port = parsed.port();
        Some((format!("{scheme}://{host}"), port))
    };
    parse(a) == parse(b)
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
        req.headers_mut()
            .insert(HeaderName::from_static("x-request-id"), v.clone());
        let mut resp = next.run(req).await;
        resp.headers_mut()
            .insert(HeaderName::from_static("x-request-id"), v);
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
    if !is_sandboxed_html && !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
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
    if !is_sandboxed_html && !headers.contains_key(HeaderName::from_static("x-frame-options")) {
        headers.insert(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("SAMEORIGIN"),
        );
    }

    resp
}

// Re-export the SessionUser type from auth::extract so callers don't need to
// reach into the deeper module path everywhere.
#[allow(dead_code)]
pub type SessionUserExt = SessionUser;
