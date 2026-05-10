//! Per-route rate limit configs. Implementation is `tower_governor`.
//!
//! The per-route table is the source of truth — see `plan/03-security.html#rate-limits`.
//! Keys default to peer IP. tower-governor's default extractor uses `ConnectInfo`,
//! which we already wire on the listener.

use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use governor::{clock::QuantaInstant, middleware::NoOpMiddleware};
use paste_core::{ErrorBody, ErrorCode, ErrorEnvelope};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorError,
    GovernorLayer,
};

pub type StdLayer = GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware<QuantaInstant>, Body>;

fn make(per_ms: u64, burst: u32) -> StdLayer {
    let cfg = GovernorConfigBuilder::default()
        .per_millisecond(per_ms)
        .burst_size(burst)
        .finish()
        .expect("rate-limit config");
    GovernorLayer::new(cfg).error_handler(rate_limit_error)
}

fn rate_limit_error(e: GovernorError) -> Response<Body> {
    let retry = match e {
        GovernorError::TooManyRequests { wait_time, .. } => wait_time,
        _ => 60,
    };
    let body = ErrorEnvelope {
        error: ErrorBody {
            code: ErrorCode::RateLimited,
            message: "rate limited".into(),
            details: Some(serde_json::json!({ "retry_after": retry })),
        },
    };
    let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
    if let Ok(v) = HeaderValue::from_str(&retry.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, v);
    }
    response
}

pub fn for_login() -> StdLayer {
    // 10 / minute
    make(6_000, 10)
}

pub fn for_register() -> StdLayer {
    // 5 / hour
    make(720_000, 5)
}

pub fn for_create_snippet() -> StdLayer {
    // 30 / min
    make(2_000, 30)
}

pub fn for_read_snippet() -> StdLayer {
    // 120 / min
    make(500, 120)
}

pub fn for_html_raw() -> StdLayer {
    // 60 / min
    make(1_000, 60)
}
