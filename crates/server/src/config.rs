use std::env;

use anyhow::{anyhow, Context};
use ipnetwork::IpNetwork;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub public_base_url: String,
    /// Empty string = same-origin (relative). Injected into SPA shell.
    pub api_base_url: String,
    pub app_name: String,
    pub session_cookie_samesite: SameSite,
    pub session_cookie_secure: bool,
    pub session_ttl_seconds: i64,
    pub cors_allowed_origins: Vec<String>,
    pub snippet_max_bytes: usize,
    pub pastedev_secret: String,
    pub argon2_m_kib: u32,
    pub argon2_t_cost: u32,
    pub trusted_client_ip_header: TrustedClientIpHeader,
    /// When true, `CF-Connecting-IP` is consulted **before** the generic
    /// `TRUSTED_CLIENT_IP_HEADER` — Cloudflare always populates it and strips
    /// client-supplied copies, so it's the authoritative source when present.
    pub trust_cloudflare: bool,
    /// If non-empty, trusted headers (CF or generic) are honoured only when the
    /// TCP peer falls inside one of these CIDRs. Empty list = honour
    /// unconditionally (operator opted in via env). Applies to both CF and the
    /// generic header so a single proxy allow-list works for layered setups.
    pub trusted_proxies: Vec<IpNetwork>,
}

/// Generic "real client IP" header. The operator opts in based on what their
/// fronting proxy sets. Default `None` = use the TCP peer address; safe for
/// direct deploys. Cloudflare's `CF-Connecting-IP` is **not** here — it has its
/// own `TRUST_CLOUDFLARE` knob and takes priority over this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustedClientIpHeader {
    None,
    XForwardedFor,
    XRealIp,
    Forwarded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Lax,
    None,
    Strict,
}

impl SameSite {
    pub fn as_header_value(self) -> &'static str {
        match self {
            SameSite::Lax => "Lax",
            SameSite::None => "None",
            SameSite::Strict => "Strict",
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL is required (postgres://user:pass@host/db)")?;
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
        let public_base_url =
            env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
        let api_base_url = env::var("API_BASE_URL").unwrap_or_default();
        let app_name = env::var("APP_NAME").unwrap_or_else(|_| "pastedev".into());

        let samesite_raw = env::var("SESSION_COOKIE_SAMESITE")
            .unwrap_or_else(|_| "lax".into())
            .to_ascii_lowercase();
        let session_cookie_samesite = match samesite_raw.as_str() {
            "lax" => SameSite::Lax,
            "none" => SameSite::None,
            "strict" => SameSite::Strict,
            other => return Err(anyhow!("invalid SESSION_COOKIE_SAMESITE: {other}")),
        };
        let session_cookie_secure = parse_bool("SESSION_COOKIE_SECURE", true)?;
        if session_cookie_samesite == SameSite::None && !session_cookie_secure {
            return Err(anyhow!(
                "SESSION_COOKIE_SAMESITE=none requires SESSION_COOKIE_SECURE=true"
            ));
        }
        let session_ttl_seconds = env::var("SESSION_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(2_592_000);

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let snippet_max_bytes = env::var("SNIPPET_MAX_BYTES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(pastedev_core::MAX_SNIPPET_BYTES);

        let pastedev_secret = env::var("PASTEDEV_SECRET")
            .context("PASTEDEV_SECRET is required (generate with `openssl rand -base64 48`)")?;
        if pastedev_secret.len() < 16 {
            return Err(anyhow!(
                "PASTEDEV_SECRET must be at least 16 characters (got {})",
                pastedev_secret.len()
            ));
        }

        let argon2_m_kib = env::var("ARGON2_M_KIB")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(65_536);
        let argon2_t_cost = env::var("ARGON2_T_COST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);

        let trusted_client_ip_header = match env::var("TRUSTED_CLIENT_IP_HEADER")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "none" => TrustedClientIpHeader::None,
            "x-forwarded-for" | "xff" => TrustedClientIpHeader::XForwardedFor,
            "x-real-ip" => TrustedClientIpHeader::XRealIp,
            "forwarded" => TrustedClientIpHeader::Forwarded,
            other => return Err(anyhow!(
                "invalid TRUSTED_CLIENT_IP_HEADER: {other} (allowed: none, x-forwarded-for, x-real-ip, forwarded). For Cloudflare set TRUST_CLOUDFLARE=true instead."
            )),
        };

        let trust_cloudflare = parse_bool("TRUST_CLOUDFLARE", false)?;

        let trusted_proxies = env::var("TRUSTED_PROXIES")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<IpNetwork>()
                    .map_err(|e| anyhow!("invalid TRUSTED_PROXIES entry {s:?}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let any_header_trust =
            !matches!(trusted_client_ip_header, TrustedClientIpHeader::None) || trust_cloudflare;
        if !any_header_trust && !trusted_proxies.is_empty() {
            return Err(anyhow!(
                "TRUSTED_PROXIES is set but no trusted header is enabled — set TRUSTED_CLIENT_IP_HEADER or TRUST_CLOUDFLARE"
            ));
        }
        if trust_cloudflare && trusted_proxies.is_empty() {
            tracing::warn!(
                "TRUST_CLOUDFLARE=true with no TRUSTED_PROXIES — CF-Connecting-IP will be \
                 honoured from any peer. Restrict origin to Cloudflare IPs at the network \
                 layer, or set TRUSTED_PROXIES to Cloudflare's published ranges."
            );
        }

        Ok(Self {
            bind_addr,
            database_url,
            public_base_url,
            api_base_url,
            app_name,
            session_cookie_samesite,
            session_cookie_secure,
            session_ttl_seconds,
            cors_allowed_origins,
            snippet_max_bytes,
            pastedev_secret,
            argon2_m_kib,
            argon2_t_cost,
            trusted_client_ip_header,
            trust_cloudflare,
            trusted_proxies,
        })
    }
}

fn parse_bool(name: &str, default: bool) -> anyhow::Result<bool> {
    match env::var(name) {
        Ok(v) => match v.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            other => Err(anyhow!("invalid bool for {name}: {other}")),
        },
        Err(_) => Ok(default),
    }
}
