pub mod api_key;
pub mod extract;
pub mod password;
pub mod session;

use axum::http::HeaderMap;

/// Truncated User-Agent (max 256 chars) used for audit/session rows.
pub fn client_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.chars().take(256).collect::<String>())
}
