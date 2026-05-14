-- Soft delete for snippets.
--
-- The DELETE handler used to physically remove rows (CASCADE wiping
-- snippet_views with it). Soft delete is friendlier for audit / accidental
-- removal recovery and lets the owner change their mind later. Reads in repo
-- (by_id, by_slug, list_for_user) gate on `deleted_at IS NULL` so soft-deleted
-- snippets stop resolving immediately, the same way hard-delete behaved.
--
-- The unique index on `slug` stays unconditional: a slug points to at most one
-- snippet ever (live or soft-deleted), so an existing link can't suddenly
-- resolve to different content if the same slug is randomly re-issued.

ALTER TABLE snippets ADD COLUMN deleted_at timestamptz;
CREATE INDEX snippets_deleted_at_ix ON snippets (deleted_at)
    WHERE deleted_at IS NOT NULL;
