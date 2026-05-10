use std::env;

use anyhow::{anyhow, Context};

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
    pub paste_secret: String,
    pub argon2_m_kib: u32,
    pub argon2_t_cost: u32,
    pub rust_env: RustEnv,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustEnv {
    Dev,
    Production,
}

impl RustEnv {
    pub fn is_dev(self) -> bool {
        matches!(self, RustEnv::Dev)
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
        let app_name = env::var("APP_NAME").unwrap_or_else(|_| "paste".into());

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
            .unwrap_or(paste_core::MAX_SNIPPET_BYTES);

        let paste_secret = env::var("PASTE_SECRET")
            .context("PASTE_SECRET is required (generate with `openssl rand -base64 48`)")?;
        if paste_secret.len() < 16 {
            return Err(anyhow!(
                "PASTE_SECRET must be at least 16 characters (got {})",
                paste_secret.len()
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

        let rust_env = match env::var("RUST_ENV")
            .unwrap_or_else(|_| "production".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "dev" | "development" => RustEnv::Dev,
            _ => RustEnv::Production,
        };

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
            paste_secret,
            argon2_m_kib,
            argon2_t_cost,
            rust_env,
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
