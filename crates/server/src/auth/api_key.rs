//! API-key minting and verification.
//!
//! Wire format: `pds_live_<8-char prefix>_<32-char secret>`. The prefix is
//! stored plaintext (and indexed) for lookup; the entire token is hashed with
//! `HMAC-SHA256(PASTEDEV_SECRET, token)` and the digest is what we compare
//! against. The HMAC key means a DB-only leak doesn't expose the tokens to
//! rainbow-table lookup or any precomputed-hash attack — an attacker also
//! needs `PASTEDEV_SECRET` to validate guesses.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use constant_time_eq::constant_time_eq;
use hmac::{Hmac, Mac};
use nanoid::nanoid;
use pastedev_core::{Scope, API_KEY_PREFIX_LEN, API_KEY_SECRET_LEN, API_KEY_TOKEN_PREAMBLE};
use sha2::Sha256;
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k', 'l', 'm', 'n', 'p', 'q', 'r', 's', 't', 'u',
    'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P',
    'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '2', '3', '4', '5', '6', '7', '8', '9',
];

#[derive(Debug, Clone)]
pub struct ApiKeyRow {
    pub id: Uuid,
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

/// Generate a fresh `(plaintext, prefix, hmac_digest)` triple. The caller
/// stores the prefix + digest and hands the plaintext back exactly once.
pub fn mint_token(server_secret: &str) -> (String, String, [u8; 32]) {
    let prefix = nanoid!(API_KEY_PREFIX_LEN, ALPHABET);
    let secret = nanoid!(API_KEY_SECRET_LEN, ALPHABET);
    let token = format!("{API_KEY_TOKEN_PREAMBLE}{prefix}_{secret}");
    let digest = hmac_sha256(server_secret, &token);
    (token, prefix, digest)
}

/// `HMAC-SHA256(server_secret, token)`. Keyed by `PASTEDEV_SECRET`, so a
/// dump of `api_keys.token_hash` alone is useless without the key.
fn hmac_sha256(server_secret: &str, token: &str) -> [u8; 32] {
    // `Hmac::new_from_slice` only fails for zero-length keys; `PASTEDEV_SECRET`
    // is validated to be ≥16 chars at startup (see config.rs), so this is
    // infallible in practice. The `expect` documents the invariant.
    let mut mac = HmacSha256::new_from_slice(server_secret.as_bytes())
        .expect("PASTEDEV_SECRET length validated at startup");
    mac.update(token.as_bytes());
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Mint a new API key for `user_id` and persist it. Returns the plaintext
/// token alongside the row; the plaintext is not stored anywhere else.
pub async fn insert(
    pool: &PgPool,
    server_secret: &str,
    user_id: Uuid,
    name: &str,
    scopes: &[Scope],
) -> Result<Minted, sqlx::Error> {
    let (token, prefix, hash) = mint_token(server_secret);
    let scope_strs: Vec<String> = scopes.iter().map(|s| s.as_str().to_string()).collect();
    let hash_slice: &[u8] = &hash[..];
    let row = sqlx::query!(
        "INSERT INTO api_keys (user_id, name, prefix, token_hash, scopes)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, created_at",
        user_id,
        name,
        prefix,
        hash_slice,
        &scope_strs,
    )
    .fetch_one(pool)
    .await?;
    Ok(Minted {
        row: ApiKeyRow {
            id: row.id,
            name: name.to_string(),
            prefix,
            scopes: scopes.to_vec(),
            created_at: row.created_at,
            last_used_at: None,
            revoked_at: None,
        },
        token,
    })
}

/// All keys (active and revoked) belonging to one user, newest first.
pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT id, name, prefix, scopes, created_at, last_used_at, revoked_at
           FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC"#,
        user_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ApiKeyRow {
            id: r.id,
            name: r.name,
            prefix: r.prefix,
            scopes: r.scopes.iter().filter_map(|s| s.parse().ok()).collect(),
            created_at: r.created_at,
            last_used_at: r.last_used_at,
            revoked_at: r.revoked_at,
        })
        .collect())
}

/// Revoke `id` if it belongs to `user_id` and isn't already revoked. Returns
/// `true` when a row was actually updated.
pub async fn revoke(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let r = sqlx::query!(
        "UPDATE api_keys SET revoked_at = now()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        id,
        user_id,
    )
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

/// Look up the row matching `bearer`'s prefix, constant-time-compare the
/// HMAC of the token against the stored digest, and return the verified key
/// if it's still active.
pub async fn verify(
    pool: &PgPool,
    server_secret: &str,
    bearer: &str,
) -> Result<Option<VerifiedKey>, sqlx::Error> {
    let Some(rest) = bearer.strip_prefix(API_KEY_TOKEN_PREAMBLE) else {
        return Ok(None);
    };
    let Some((prefix, _secret)) = rest.split_once('_') else {
        return Ok(None);
    };
    if prefix.len() != API_KEY_PREFIX_LEN {
        return Ok(None);
    }
    let row = sqlx::query!(
        "SELECT id, user_id, token_hash, scopes, revoked_at
         FROM api_keys WHERE prefix = $1",
        prefix,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.revoked_at.is_some() {
        return Ok(None);
    }
    let incoming = hmac_sha256(server_secret, bearer);
    if !constant_time_eq(&incoming, &row.token_hash) {
        return Ok(None);
    }
    touch_last_used(pool, row.id).await;
    let scopes: Vec<Scope> = row
        .scopes
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    Ok(Some(VerifiedKey {
        key_id: row.id,
        user_id: row.user_id,
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
    let _ = sqlx::query!(
        "UPDATE api_keys SET last_used_at = now() WHERE id = $1",
        key_id,
    )
    .execute(pool)
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shape_is_predictable() {
        let (token, prefix, _hash) = mint_token("test-secret-at-least-sixteen-chars");
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

    #[test]
    fn hmac_depends_on_server_secret() {
        let token = "pds_live_aaaaaaaa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let a = hmac_sha256("server-secret-one-one-one", token);
        let b = hmac_sha256("server-secret-two-two-two", token);
        // Different keys → different digests. This is the whole point.
        assert_ne!(a, b);
        // Same key, same input → deterministic.
        let a2 = hmac_sha256("server-secret-one-one-one", token);
        assert_eq!(a, a2);
    }
}
