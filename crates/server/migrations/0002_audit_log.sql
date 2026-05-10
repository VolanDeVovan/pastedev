-- Audit log. Inline writes from handlers; partial indexes for the common queries.

CREATE TABLE audit_log (
    id                bigserial     PRIMARY KEY,
    occurred_at       timestamptz   NOT NULL DEFAULT now(),
    event             varchar(64)   NOT NULL,
    actor_user_id     uuid          REFERENCES users(id) ON DELETE SET NULL,
    actor_api_key_id  uuid          REFERENCES api_keys(id) ON DELETE SET NULL,
    target_user_id    uuid          REFERENCES users(id) ON DELETE SET NULL,
    target_snippet_id uuid          REFERENCES snippets(id) ON DELETE SET NULL,
    ip                inet,
    user_agent        text,
    payload           jsonb         NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX audit_event_ix       ON audit_log (event, occurred_at DESC);
CREATE INDEX audit_actor_user_ix  ON audit_log (actor_user_id, occurred_at DESC);
CREATE INDEX audit_target_user_ix ON audit_log (target_user_id, occurred_at DESC)
    WHERE target_user_id IS NOT NULL;
