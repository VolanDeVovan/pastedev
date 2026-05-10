//! API-key minting and verification.
//!
//! Wire format: `pds_live_<8-char prefix>_<32-char secret>`. The prefix is
//! stored plaintext (and indexed) for lookup; the entire token is hashed
//! (SHA-256) and the hash is what we compare against.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use constant_time_eq::constant_time_eq;
use nanoid::nanoid;
use paste_core::{Scope, API_KEY_PREFIX_LEN, API_KEY_SECRET_LEN, API_KEY_TOKEN_PREAMBLE};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P',
    'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7', '8', '9',
];

#[derive(Debug, Clone)]
pub struct ApiKeyRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<Scope>,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub struct Minted {
    pub row: ApiKeyRow,
    /// Plaintext token, only available at creation. Format `pds_live_<prefix>_<secret>`.
    pub token: String,
}

pub fn mint_token() -> (String, String, [u8; 32]) {
    let prefix = nanoid!(API_KEY_PREFIX_LEN, ALPHABET);
    let secret = nanoid!(API_KEY_SECRET_LEN, ALPHABET);
    let token = format!("{API_KEY_TOKEN_PREAMBLE}{prefix}_{secret}");
    let hash = sha256(&token);
    (token, prefix, hash)
}

fn sha256(s: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub async fn insert(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    scopes: &[Scope],
) -> Result<Minted, sqlx::Error> {
    let (token, prefix, hash) = mint_token();
    let scope_strs: Vec<String> = scopes.iter().map(|s| s.as_str().to_string()).collect();
    let row: (Uuid, OffsetDateTime) = sqlx::query_as(
        "INSERT INTO api_keys (user_id, name, prefix, token_hash, scopes)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, created_at",
    )
    .bind(user_id)
    .bind(name)
    .bind(&prefix)
    .bind(&hash[..])
    .bind(&scope_strs)
    .fetch_one(pool)
    .await?;
    Ok(Minted {
        row: ApiKeyRow {
            id: row.0,
            user_id,
            name: name.to_string(),
            prefix,
            scopes: scopes.to_vec(),
            created_at: row.1,
            last_used_at: None,
            revoked_at: None,
        },
        token,
    })
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    let rows: Vec<(
        Uuid,
        Uuid,
        String,
        String,
        Vec<String>,
        OffsetDateTime,
        Option<OffsetDateTime>,
        Option<OffsetDateTime>,
    )> = sqlx::query_as(
        "SELECT id, user_id, name, prefix, scopes, created_at, last_used_at, revoked_at
         FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ApiKeyRow {
            id: r.0,
            user_id: r.1,
            name: r.2,
            prefix: r.3,
            scopes: r.4.iter().filter_map(|s| Scope::from_str_opt(s)).collect(),
            created_at: r.5,
            last_used_at: r.6,
            revoked_at: r.7,
        })
        .collect())
}

pub async fn revoke(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE api_keys SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

/// Verified bearer. The user's role/status are populated by the extractor;
/// here we just confirm token + scope.
#[derive(Debug, Clone)]
pub struct VerifiedKey {
    pub key_id: Uuid,
    pub user_id: Uuid,
    pub scopes: Vec<Scope>,
}

pub async fn verify(pool: &PgPool, bearer: &str) -> Result<Option<VerifiedKey>, sqlx::Error> {
    let Some(rest) = bearer.strip_prefix(API_KEY_TOKEN_PREAMBLE) else {
        return Ok(None);
    };
    let Some((prefix, _secret)) = rest.split_once('_') else {
        return Ok(None);
    };
    if prefix.len() != API_KEY_PREFIX_LEN {
        return Ok(None);
    }
    let row: Option<(Uuid, Uuid, Vec<u8>, Vec<String>, Option<OffsetDateTime>)> = sqlx::query_as(
        "SELECT id, user_id, token_hash, scopes, revoked_at
         FROM api_keys WHERE prefix = $1",
    )
    .bind(prefix)
    .fetch_optional(pool)
    .await?;
    let Some((id, user_id, stored_hash, scope_strs, revoked_at)) = row else {
        return Ok(None);
    };
    if revoked_at.is_some() {
        return Ok(None);
    }
    let incoming = sha256(bearer);
    if !constant_time_eq(&incoming, &stored_hash) {
        return Ok(None);
    }
    touch_last_used(pool, id).await;
    let scopes: Vec<Scope> = scope_strs.iter().filter_map(|s| Scope::from_str_opt(s)).collect();
    Ok(Some(VerifiedKey {
        key_id: id,
        user_id,
        scopes,
    }))
}

/// Debounced `UPDATE api_keys SET last_used_at`. At most one UPDATE per key per
/// minute, regardless of how many requests fire. The cache is in-process; under
/// horizontally scaled deployments each replica gets its own minute window,
/// which is fine — the column is informational only.
async fn touch_last_used(pool: &PgPool, key_id: Uuid) {
    static CACHE: OnceLock<Mutex<HashMap<Uuid, Instant>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let mut guard = cache.lock().await;
        if let Some(prev) = guard.get(&key_id) {
            if prev.elapsed() < Duration::from_secs(60) {
                return;
            }
        }
        guard.insert(key_id, Instant::now());
    }
    let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = $1")
        .bind(key_id)
        .execute(pool)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shape_is_predictable() {
        let (token, prefix, _hash) = mint_token();
        assert!(token.starts_with("pds_live_"));
        assert_eq!(prefix.len(), 8);
        let parts: Vec<&str> = token.splitn(3, '_').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "pds");
        // The 'live_<prefix>' joins via single underscores too, but our format
        // is `pds_live_<prefix>_<secret>`; splitting on first two _ gives us
        // ("pds", "live", "<prefix>_<secret>"). Verify the suffix is prefix_secret.
        let suffix = parts[2];
        let (got_prefix, got_secret) = suffix.split_once('_').unwrap();
        assert_eq!(got_prefix, prefix);
        assert_eq!(got_secret.len(), 32);
    }
}
