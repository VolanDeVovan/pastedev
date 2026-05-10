-- Phase 0 initial schema. Tables, indexes, and triggers from plan/04-data-model.html.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id              uuid          PRIMARY KEY DEFAULT gen_random_uuid(),
    username        varchar(40)   NOT NULL CHECK (username ~ '^[a-z0-9_.\-]{3,40}$'),
    email           varchar(255),
    password_hash   text          NOT NULL,
    role            varchar(16)   NOT NULL DEFAULT 'user'
                                  CHECK (role IN ('user', 'admin')),
    status          varchar(16)   NOT NULL DEFAULT 'pending'
                                  CHECK (status IN ('pending', 'approved', 'rejected', 'suspended')),
    reason          text,
    registration_ip inet,
    created_at      timestamptz   NOT NULL DEFAULT now(),
    updated_at      timestamptz   NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_username_uniq ON users (username);
CREATE UNIQUE INDEX users_email_uniq    ON users (lower(email)) WHERE email IS NOT NULL;
CREATE INDEX        users_status_ix     ON users (status);

CREATE TABLE sessions (
    id            bytea         PRIMARY KEY,
    user_id       uuid          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    timestamptz   NOT NULL DEFAULT now(),
    last_seen_at  timestamptz   NOT NULL DEFAULT now(),
    expires_at    timestamptz   NOT NULL,
    ip            inet,
    user_agent    text
);
CREATE INDEX sessions_user_ix    ON sessions (user_id);
CREATE INDEX sessions_expires_ix ON sessions (expires_at);

CREATE TABLE snippets (
    id          uuid          PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        char(7)       NOT NULL,
    owner_id    uuid          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type        varchar(16)   NOT NULL CHECK (type IN ('code', 'markdown', 'html')),
    name        varchar(255),
    body        text          NOT NULL CHECK (octet_length(body) <= 1048576),
    size_bytes  integer       NOT NULL,
    visibility  varchar(16)   NOT NULL DEFAULT 'public'
                              CHECK (visibility IN ('public', 'unlisted', 'private')),
    views       integer       NOT NULL DEFAULT 0,
    created_at  timestamptz   NOT NULL DEFAULT now(),
    updated_at  timestamptz   NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX snippets_slug_uniq ON snippets (slug);
CREATE INDEX        snippets_owner_ix  ON snippets (owner_id, created_at DESC);

CREATE TABLE api_keys (
    id            uuid          PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       uuid          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name          varchar(80)   NOT NULL,
    prefix        char(8)       NOT NULL,
    token_hash    bytea         NOT NULL,
    scopes        text[]        NOT NULL,
    created_at    timestamptz   NOT NULL DEFAULT now(),
    last_used_at  timestamptz,
    revoked_at    timestamptz
);
CREATE UNIQUE INDEX api_keys_prefix_uniq ON api_keys (prefix);
CREATE UNIQUE INDEX api_keys_hash_uniq   ON api_keys (token_hash);
CREATE INDEX        api_keys_user_ix     ON api_keys (user_id);

CREATE OR REPLACE FUNCTION bump_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END; $$ LANGUAGE plpgsql;

CREATE TRIGGER users_bump_updated    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION bump_updated_at();
CREATE TRIGGER snippets_bump_updated BEFORE UPDATE ON snippets
    FOR EACH ROW EXECUTE FUNCTION bump_updated_at();
