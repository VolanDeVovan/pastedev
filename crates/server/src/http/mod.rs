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
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{
    api_keys::handlers as key_handlers,
    assets,
    config::Config,
    db,
    error::AppError,
    setup::{self, SetupGate},
    snippets::handlers as snippet_handlers,
    users::{admin as user_admin, handlers as user_handlers},
};

pub mod client_ip;
pub mod rate_limit;
pub mod shell;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub setup_gate: Arc<SetupGate>,
    pub client_ip: Arc<client_ip::ClientIpResolver>,
}

pub fn router(state: AppState) -> Router {
    let api_setup = Router::new()
        .route("/setup/status", get(setup::status))
        .route("/setup/admin", post(setup::create_first_admin))
        .with_state(state.clone());

    let api_auth = Router::new()
        .route(
            "/auth/register",
            post(user_handlers::register).layer(rate_limit::for_register(&state.client_ip)),
        )
        .route(
            "/auth/login",
            post(user_handlers::login).layer(rate_limit::for_login(&state.client_ip)),
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
                .layer(rate_limit::for_create_snippet(&state.client_ip))
                .get(snippet_handlers::list),
        )
        .route(
            "/snippets/{slug}",
            get(snippet_handlers::get)
                .layer(rate_limit::for_read_snippet(&state.client_ip))
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

    // CORS: only enabled when CORS_ALLOWED_ORIGINS is non-empty. Same-origin
    // deploys don't need it (browser never sends cross-site requests to the API).
    let api = if let Some(cors) = build_cors_layer(&state.config) {
        api.layer(cors)
    } else {
        api
    };

    let raw_routes = Router::new()
        .route("/c/{slug}/raw", get(snippet_handlers::raw_text))
        .route("/m/{slug}/raw", get(snippet_handlers::raw_text))
        .route(
            "/h/{slug}/raw",
            get(snippet_handlers::raw_html).layer(rate_limit::for_html_raw(&state.client_ip)),
        )
        .with_state(state.clone());

    // Top-level curl alias: `POST /paste` accepts a raw text body and returns
    // the snippet URL as plain text. Bearer-auth only (no Origin check needed —
    // the middleware in `api` skips Bearer requests anyway).
    let paste_routes = Router::new()
        .route(
            "/paste",
            post(snippet_handlers::paste_raw).layer(rate_limit::for_create_snippet(&state.client_ip)),
        )
        .layer(RequestBodyLimitLayer::new(state.config.snippet_max_bytes + 4096))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), setup_gate_middleware));

    Router::new()
        .nest("/api/v1", api)
        .merge(raw_routes)
        .merge(paste_routes)
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

/// Origin allow-list check for state-changing requests.
///
/// Bearer-auth (CLI/MCP) skips this entirely — those clients have a different
/// security model (the token itself is the credential and isn't replayable
/// cross-site the way an ambient cookie is).
///
/// For everything else — including unauthenticated state-changing endpoints
/// like `/auth/login`, `/auth/register`, and `/setup/admin` — we require an
/// allow-listed Origin. This is stricter than just "session-authed" requests:
/// the previous version soft-failed when no cookie was present and trusted the
/// handler to 401, which would silently open any future state-changing route
/// that forgot to require auth.
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
    if req.headers().get("authorization").is_some() {
        return next.run(req).await;
    }

    let allow_same = matches_self_origin(req.headers(), &state.config.public_base_url);
    let allow_listed = matches_allowed_origin(req.headers(), &state.config.cors_allowed_origins);
    if allow_same || allow_listed {
        return next.run(req).await;
    }

    AppError::Forbidden(Some("origin not allowed")).into_response()
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

/// Builds a CORS layer from `CORS_ALLOWED_ORIGINS`. Returns `None` for
/// same-origin deploys (the layer becomes a no-op there anyway, but skipping
/// avoids emitting `Vary: Origin` on every response).
///
/// Wildcards are intentionally not supported: credentialed CORS (`SameSite=None`
/// cookies + `Authorization`) requires an explicit allow-list per spec.
fn build_cors_layer(config: &Config) -> Option<CorsLayer> {
    if config.cors_allowed_origins.is_empty() {
        return None;
    }
    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    if origins.is_empty() {
        tracing::warn!(
            raw = ?config.cors_allowed_origins,
            "CORS_ALLOWED_ORIGINS was non-empty but no entries parsed; CORS off"
        );
        return None;
    }
    let layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-request-id"),
        ])
        .max_age(std::time::Duration::from_secs(600));
    Some(layer)
}

