-- The `visibility` column was selected and returned in DTOs but never enforced
-- on the read path (every snippet was readable by slug regardless of value).
-- Rather than implement private/unlisted on top of stale code, drop it; the
-- read-by-slug model with 41.7-bit entropy is the access-control story for v1.
-- Re-add the column with a real check in the read handler if/when a private
-- mode is actually planned.
ALTER TABLE snippets DROP COLUMN visibility;
