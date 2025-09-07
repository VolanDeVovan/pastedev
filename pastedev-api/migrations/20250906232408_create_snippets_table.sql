CREATE TABLE snippets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alias VARCHAR(8) UNIQUE NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP AT TIME ZONE 'UTC'),
    expires_at TIMESTAMP,
    ephemeral BOOLEAN NOT NULL DEFAULT false,
    deleted BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_snippets_expires_at ON snippets(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_snippets_deleted ON snippets(deleted);
CREATE INDEX idx_snippets_ephemeral_deleted ON snippets(ephemeral, deleted);
CREATE UNIQUE INDEX idx_snippets_alias ON snippets(alias);
