-- Unique-viewers counter. The handler `snippets::handlers::get` now records a
-- 32-byte HMAC of (ip, user-agent, snippet_id) into `snippet_views` with
-- ON CONFLICT DO NOTHING; the existing `snippets.views` column is bumped only
-- when a row is actually inserted, so it now stores unique viewers (forever)
-- rather than total GETs.

CREATE TABLE snippet_views (
    snippet_id    uuid          NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
    viewer_hash   bytea         NOT NULL,
    first_seen_at timestamptz   NOT NULL DEFAULT now(),
    PRIMARY KEY (snippet_id, viewer_hash)
);

-- Reset the column to 0 because its prior semantics (total GETs) is no longer
-- comparable to the new one (unique viewers). New views will accumulate
-- correctly going forward.
UPDATE snippets SET views = 0;
