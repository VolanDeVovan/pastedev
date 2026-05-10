pub mod api_key;
pub mod extract;
pub mod password;
pub mod session;

use std::net::IpAddr;

use axum::http::HeaderMap;

/// Truncated User-Agent (max 256 chars) used for audit/session rows.
pub fn client_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.chars().take(256).collect::<String>())
}

/// Best-guess client IP. Prefers the leftmost `X-Forwarded-For` entry (operator
/// proxy is the source of truth), falling back to the TCP peer address.
pub fn client_ip(headers: &HeaderMap, fallback: Option<IpAddr>) -> Option<IpAddr> {
    // X-Forwarded-For takes precedence (operator's proxy is the source of truth
    // for client IPs). Plain TCP peer address is the fallback for direct connections.
    if let Some(raw) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = raw.split(',').next() {
            if let Ok(parsed) = first.trim().parse::<IpAddr>() {
                return Some(parsed);
            }
        }
    }
    fallback
}
