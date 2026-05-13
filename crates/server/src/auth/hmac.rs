//! `HMAC-SHA256(PASTEDEV_SECRET, …)` — the one MAC the server uses to bind
//! sensitive material to a process-wide secret.
//!
//! Two callers today:
//! - API-key minting/verification ([`super::api_key`]) hashes the plaintext
//!   token; a DB-only leak of `api_keys.token_hash` is useless without the key.
//! - The snippet view counter hashes (ip, ua, snippet_id) into a stable
//!   per-viewer identifier so the `snippet_views` table can dedupe without
//!   storing raw IPs.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// HMAC-SHA256 of a string-shaped input. Convenience wrapper around
/// [`hmac_sha256_bytes`] for callers that already have a `&str`.
pub(crate) fn hmac_sha256(server_secret: &str, token: &str) -> [u8; 32] {
    hmac_sha256_bytes(server_secret, token.as_bytes())
}

/// HMAC-SHA256 of arbitrary bytes. Lets callers mix multiple fields
/// (null-separated, etc.) without coercing through `&str`.
pub(crate) fn hmac_sha256_bytes(server_secret: &str, material: &[u8]) -> [u8; 32] {
    // `Hmac::new_from_slice` only fails for zero-length keys; `PASTEDEV_SECRET`
    // is validated to be ≥16 chars at startup (see config.rs), so the `expect`
    // is documenting an invariant rather than a runtime branch.
    let mut mac = HmacSha256::new_from_slice(server_secret.as_bytes())
        .expect("PASTEDEV_SECRET length validated at startup");
    mac.update(material);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depends_on_server_secret() {
        let token = "pds_live_aaaaaaaa_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let a = hmac_sha256("server-secret-one-one-one", token);
        let b = hmac_sha256("server-secret-two-two-two", token);
        assert_ne!(a, b);
        assert_eq!(a, hmac_sha256("server-secret-one-one-one", token));
    }

    #[test]
    fn str_and_bytes_agree() {
        let a = hmac_sha256("server-secret-one-one-one", "abc");
        let b = hmac_sha256_bytes("server-secret-one-one-one", b"abc");
        assert_eq!(a, b);
    }
}
