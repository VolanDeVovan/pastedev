-- Visibility + expiry.
--
-- visibility:
--   'public'  -> anyone can read (current default)
--   'private' -> requires an authenticated, approved user
--
-- expires_at is the single source of truth for "when does this stop being
-- readable". Absolute timestamptz, NULL = never expires. The user-facing
-- "1 day / 15 min / never" choice is *input only* — the server converts it
-- to `now() + lifetime` and stamps the column. That keeps the semantics
-- intuitive: picking "1 day" while restoring an expired snippet always means
-- "alive for one more day from now", never "one day from creation" (which
-- would still be in the past).
--
-- burn_after_read: when true, the first non-owner view stamps
-- `first_viewed_at = now()` AND tightens expires_at down to
-- `LEAST(expires_at, now() + 15min)`. Owner views don't trigger the timer.

ALTER TABLE snippets
    ADD COLUMN visibility varchar(16) NOT NULL DEFAULT 'public'
        CHECK (visibility IN ('public', 'private'));

ALTER TABLE snippets
    ADD COLUMN expires_at timestamptz;

ALTER TABLE snippets
    ADD COLUMN burn_after_read boolean NOT NULL DEFAULT false;

ALTER TABLE snippets
    ADD COLUMN first_viewed_at timestamptz;
