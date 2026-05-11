-- API-key hashing switched from `SHA256(token)` to
-- `HMAC-SHA256(PASTEDEV_SECRET, token)`. Existing rows have hashes computed
-- under the old scheme; we don't have the plaintext to recompute them, so the
-- only safe action is to revoke them and have users re-mint via the SPA.
-- Sessions (web auth) are unaffected — they were never hashed in the first place.
UPDATE api_keys SET revoked_at = now() WHERE revoked_at IS NULL;
