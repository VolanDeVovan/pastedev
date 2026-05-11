//! Single source of truth for "what's the client IP for this request?".
//!
//! Used by both the rate-limit key extractor and the audit/session writers so
//! the two never disagree. Behaviour is driven by config:
//!
//! - `TRUST_CLOUDFLARE=true` ⇒ `CF-Connecting-IP` is consulted **first**.
//!   Cloudflare strips client-supplied copies of the header, so when it's
//!   present and the peer is a trusted proxy it's authoritative.
//! - `TRUSTED_CLIENT_IP_HEADER=<name>` ⇒ generic fallback (`X-Forwarded-For`
//!   leftmost, `X-Real-IP`, or RFC 7239 `Forwarded`).
//! - `TRUSTED_PROXIES=<cidrs>` ⇒ if non-empty, both trusted-header paths only
//!   fire when the TCP peer is inside the list. Empty = trust unconditionally
//!   (operator's choice).
//!
//! Default (`None` header + cloudflare off) = TCP peer IP, same as the
//! out-of-the-box `tower_governor::PeerIpKeyExtractor`.

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::{request::Parts, HeaderMap, Request};
use ipnetwork::IpNetwork;
use tower_governor::{key_extractor::KeyExtractor, GovernorError};

use crate::config::{Config, TrustedClientIpHeader};
use crate::http::AppState;

#[derive(Debug, Clone)]
pub struct ClientIpResolver {
    header: TrustedClientIpHeader,
    trust_cloudflare: bool,
    trusted_proxies: Vec<IpNetwork>,
}

impl ClientIpResolver {
    pub fn from_config(c: &Config) -> Self {
        Self {
            header: c.trusted_client_ip_header,
            trust_cloudflare: c.trust_cloudflare,
            trusted_proxies: c.trusted_proxies.clone(),
        }
    }

    /// Resolve the client IP given request headers and the TCP peer address.
    /// Returns `None` only when we have no headers to trust and no peer (e.g.
    /// `ConnectInfo` wasn't wired) — callers that need a non-None fallback
    /// should default to a sentinel like `0.0.0.0` and log.
    pub fn resolve(&self, headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
        if self.peer_is_trusted(peer) {
            if self.trust_cloudflare {
                if let Some(ip) = single_ip_header(headers, "cf-connecting-ip") {
                    return Some(ip);
                }
            }
            match self.header {
                TrustedClientIpHeader::None => {}
                TrustedClientIpHeader::XForwardedFor => {
                    if let Some(ip) = leftmost_xff(headers) {
                        return Some(ip);
                    }
                }
                TrustedClientIpHeader::XRealIp => {
                    if let Some(ip) = single_ip_header(headers, "x-real-ip") {
                        return Some(ip);
                    }
                }
                TrustedClientIpHeader::Forwarded => {
                    if let Some(ip) = leftmost_forwarded(headers) {
                        return Some(ip);
                    }
                }
            }
        }
        peer
    }

    fn peer_is_trusted(&self, peer: Option<IpAddr>) -> bool {
        if self.trusted_proxies.is_empty() {
            return true;
        }
        let Some(peer) = peer else { return false };
        self.trusted_proxies.iter().any(|net| net.contains(peer))
    }
}

fn single_ip_header(headers: &HeaderMap, name: &str) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
}

fn leftmost_xff(headers: &HeaderMap) -> Option<IpAddr> {
    let raw = headers.get("x-forwarded-for")?.to_str().ok()?;
    raw.split(',').next()?.trim().parse().ok()
}

/// Parse RFC 7239 `Forwarded`, leftmost `for=` value. Handles quoted strings
/// (`for="[2001:db8::1]:4711"`) and bracketed IPv6.
fn leftmost_forwarded(headers: &HeaderMap) -> Option<IpAddr> {
    let raw = headers.get("forwarded")?.to_str().ok()?;
    let first = raw.split(',').next()?.trim();
    for part in first.split(';') {
        let kv = part.trim();
        let Some(rest) = kv.strip_prefix("for=").or_else(|| kv.strip_prefix("For=")) else {
            continue;
        };
        let mut s = rest.trim();
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            s = &s[1..s.len() - 1];
        }
        // Bracketed IPv6: `[::1]` or `[::1]:port`
        if let Some(inner) = s.strip_prefix('[') {
            if let Some(end) = inner.find(']') {
                return inner[..end].parse().ok();
            }
        }
        // IPv4 with optional :port — split at the LAST `:` if there's exactly one
        // (IPv6 without brackets has multiple `:` and shouldn't be stripped).
        let candidate = if s.matches(':').count() == 1 {
            s.split(':').next().unwrap_or(s)
        } else {
            s
        };
        return candidate.parse().ok();
    }
    None
}

/// Axum extractor — `ClientIp(ip): ClientIp` in a handler signature gives the
/// resolved client IP (or `None` if neither headers nor `ConnectInfo` produced
/// one). Resolution uses the same `ClientIpResolver` from app state that the
/// rate limiter uses, so the two never disagree on "who sent this request".
#[derive(Debug, Clone, Copy)]
pub struct ClientIp(pub Option<IpAddr>);

impl ClientIp {
    /// Convenience: same value as `IpNetwork::from(addr)` for storage in
    /// `audit.ip` / `sessions.ip` columns.
    pub fn as_ipnetwork(&self) -> Option<IpNetwork> {
        self.0.map(IpNetwork::from)
    }
}

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Infallible> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        Ok(ClientIp(state.client_ip.resolve(&parts.headers, peer)))
    }
}

/// `tower_governor` `KeyExtractor` that defers to `ClientIpResolver`. Same
/// behaviour everywhere the resolver runs.
#[derive(Clone)]
pub struct ClientIpKeyExtractor(pub Arc<ClientIpResolver>);

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;

    fn extract<T>(&self, req: &Request<T>) -> Result<IpAddr, GovernorError> {
        let peer = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip());
        self.0
            .resolve(req.headers(), peer)
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};
    use std::str::FromStr;

    fn resolver(
        header: TrustedClientIpHeader,
        trust_cloudflare: bool,
        trusted_proxies: &[&str],
    ) -> ClientIpResolver {
        ClientIpResolver {
            header,
            trust_cloudflare,
            trusted_proxies: trusted_proxies
                .iter()
                .map(|s| s.parse().unwrap())
                .collect(),
        }
    }

    fn ip(s: &str) -> IpAddr {
        IpAddr::from_str(s).unwrap()
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn default_returns_peer() {
        let r = resolver(TrustedClientIpHeader::None, false, &[]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        // No trusted header configured → headers are ignored.
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("10.0.0.1")));
    }

    #[test]
    fn xff_leftmost_when_enabled() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, false, &[]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4, 5.6.7.8")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn x_real_ip() {
        let r = resolver(TrustedClientIpHeader::XRealIp, false, &[]);
        let h = headers(&[("x-real-ip", "1.2.3.4")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn cloudflare_wins_over_xff() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, true, &[]);
        let h = headers(&[
            ("cf-connecting-ip", "9.9.9.9"),
            ("x-forwarded-for", "1.2.3.4, 5.6.7.8"),
        ]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("9.9.9.9")));
    }

    #[test]
    fn cloudflare_falls_through_when_header_missing() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, true, &[]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn trusted_proxies_gate_passes() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, false, &["10.0.0.0/8"]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.5"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn trusted_proxies_gate_blocks_untrusted_peer() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, false, &["10.0.0.0/8"]);
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        // Peer outside trusted CIDRs → ignore headers, fall back to peer.
        assert_eq!(r.resolve(&h, Some(ip("8.8.8.8"))), Some(ip("8.8.8.8")));
    }

    #[test]
    fn cloudflare_also_gated_by_trusted_proxies() {
        let r = resolver(TrustedClientIpHeader::None, true, &["10.0.0.0/8"]);
        let h = headers(&[("cf-connecting-ip", "9.9.9.9")]);
        assert_eq!(r.resolve(&h, Some(ip("8.8.8.8"))), Some(ip("8.8.8.8")));
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("9.9.9.9")));
    }

    #[test]
    fn forwarded_ipv4() {
        let r = resolver(TrustedClientIpHeader::Forwarded, false, &[]);
        let h = headers(&[("forwarded", "for=1.2.3.4;proto=https;by=10.0.0.1")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn forwarded_ipv4_port_stripped() {
        let r = resolver(TrustedClientIpHeader::Forwarded, false, &[]);
        let h = headers(&[("forwarded", "for=1.2.3.4:5555")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("1.2.3.4")));
    }

    #[test]
    fn forwarded_ipv6_bracketed() {
        let r = resolver(TrustedClientIpHeader::Forwarded, false, &[]);
        let h = headers(&[("forwarded", r#"for="[2001:db8::1]:4711""#)]);
        assert_eq!(
            r.resolve(&h, Some(ip("10.0.0.1"))),
            Some(ip("2001:db8::1"))
        );
    }

    #[test]
    fn malformed_header_falls_back_to_peer() {
        let r = resolver(TrustedClientIpHeader::XForwardedFor, false, &[]);
        let h = headers(&[("x-forwarded-for", "not-an-ip")]);
        assert_eq!(r.resolve(&h, Some(ip("10.0.0.1"))), Some(ip("10.0.0.1")));
    }
}
