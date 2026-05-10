//! DB-backed sessions. The cookie value is base64url of the 32-byte row ID.

use base64::Engine;
use ipnetwork::IpNetwork;
use rand::RngCore;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: Vec<u8>,
    pub user_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
}

const RENEW_AFTER: Duration = Duration::days(1);
const EXTEND_WITHIN: Duration = Duration::days(7);

pub fn encode_cookie(id: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(id)
}

pub fn decode_cookie(value: &str) -> Option<Vec<u8>> {
    let trimmed = value.trim();
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(trimmed)
        .ok()
        .filter(|v| v.len() == 32)
}

pub async fn issue(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    ip: Option<IpNetwork>,
    ua: Option<&str>,
) -> Result<String, sqlx::Error> {
    let mut id = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut id);
    let now = OffsetDateTime::now_utc();
    let expires = now + Duration::seconds(config.session_ttl_seconds);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, created_at, last_seen_at, expires_at, ip, user_agent)
         VALUES ($1, $2, $3, $3, $4, $5, $6)",
    )
    .bind(&id[..])
    .bind(user_id)
    .bind(now)
    .bind(expires)
    .bind(ip)
    .bind(ua)
    .execute(pool)
    .await?;

    Ok(encode_cookie(&id))
}

/// Validate a session id (raw bytes). Returns `Some(SessionRow)` if the row
/// exists, hasn't expired, and the user isn't suspended.
pub async fn validate(pool: &PgPool, id: &[u8]) -> Result<Option<SessionRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, (Vec<u8>, Uuid, OffsetDateTime, OffsetDateTime, OffsetDateTime, String)>(
        "SELECT s.id, s.user_id, s.expires_at, s.last_seen_at, s.created_at, u.status
         FROM sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some((id, user_id, expires_at, last_seen_at, _created_at, status)) = row else {
        return Ok(None);
    };
    let now = OffsetDateTime::now_utc();
    if expires_at <= now {
        // Expired. Clean up async; ignore errors.
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(&id[..])
            .execute(pool)
            .await;
        return Ok(None);
    }
    if status == "suspended" {
        let _ = sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(&id[..])
            .execute(pool)
            .await;
        return Ok(None);
    }

    Ok(Some(SessionRow {
        id,
        user_id,
        expires_at,
        last_seen_at,
    }))
}

/// Sliding renewal: bump last_seen and possibly extend expires_at.
pub async fn maybe_renew(
    pool: &PgPool,
    config: &Config,
    row: &SessionRow,
) -> Result<(), sqlx::Error> {
    let now = OffsetDateTime::now_utc();
    let need_seen_bump = now - row.last_seen_at > RENEW_AFTER;
    let need_expiry_extend = row.expires_at - now < EXTEND_WITHIN;

    if !need_seen_bump && !need_expiry_extend {
        return Ok(());
    }
    let new_expires = if need_expiry_extend {
        now + Duration::seconds(config.session_ttl_seconds)
    } else {
        row.expires_at
    };
    sqlx::query(
        "UPDATE sessions SET last_seen_at = $1, expires_at = $2 WHERE id = $3",
    )
    .bind(now)
    .bind(new_expires)
    .bind(&row.id[..])
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn revoke(pool: &PgPool, id: &[u8]) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_all_for_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn build_cookie(config: &Config, value: &str, max_age_seconds: i64) -> String {
    let secure = if config.session_cookie_secure {
        "; Secure"
    } else {
        ""
    };
    format!(
        "{name}={value}; Path=/; HttpOnly{secure}; SameSite={samesite}; Max-Age={max_age}",
        name = paste_core::SESSION_COOKIE_NAME,
        samesite = config.session_cookie_samesite.as_header_value(),
        max_age = max_age_seconds,
    )
}

pub fn build_clear_cookie(config: &Config) -> String {
    build_cookie(config, "", 0)
}
