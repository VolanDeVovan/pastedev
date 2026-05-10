//! Append rows to `audit_log`. Best-effort: a failed insert is logged but never
//! surfaced to the handler — auditing never blocks a user action.

use ipnetwork::IpNetwork;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Default)]
pub struct Event<'a> {
    pub event: &'a str,
    pub actor_user_id: Option<Uuid>,
    pub actor_api_key_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub target_snippet_id: Option<Uuid>,
    pub ip: Option<IpNetwork>,
    pub user_agent: Option<&'a str>,
    pub payload: Option<Value>,
}

/// Insert one audit row. Errors are logged inline so callers can use a bare
/// `audit::write(...).await` without a wrapping `if let Err`.
pub async fn write(pool: &PgPool, event: Event<'_>) {
    let action = event.event;
    let result = sqlx::query!(
        "INSERT INTO audit_log
            (event, actor_user_id, actor_api_key_id, target_user_id, target_snippet_id, ip, user_agent, payload)
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, '{}'::jsonb))",
        event.event,
        event.actor_user_id,
        event.actor_api_key_id,
        event.target_user_id,
        event.target_snippet_id,
        event.ip,
        event.user_agent,
        event.payload,
    )
    .execute(pool)
    .await;
    if let Err(e) = result {
        tracing::warn!(action, error = ?e, "audit log write failed");
    }
}

/// Detached version of [`write`] — runs the insert on a fresh tokio task so the
/// caller never awaits the DB round-trip. Used in hot paths where ordering
/// doesn't matter (snippet/key handlers).
pub fn spawn_write(pool: PgPool, event: OwnedEvent) {
    tokio::spawn(async move {
        write(&pool, event.as_event()).await;
    });
}

/// Owned counterpart to [`Event`] for use with [`spawn_write`]. Captures all
/// the borrowed string slices as `String` so the value can be moved across
/// task boundaries.
#[derive(Default)]
pub struct OwnedEvent {
    pub event: &'static str,
    pub actor_user_id: Option<Uuid>,
    pub actor_api_key_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub target_snippet_id: Option<Uuid>,
    pub ip: Option<IpNetwork>,
    pub user_agent: Option<String>,
    pub payload: Option<Value>,
}

impl OwnedEvent {
    fn as_event(&self) -> Event<'_> {
        Event {
            event: self.event,
            actor_user_id: self.actor_user_id,
            actor_api_key_id: self.actor_api_key_id,
            target_user_id: self.target_user_id,
            target_snippet_id: self.target_snippet_id,
            ip: self.ip,
            user_agent: self.user_agent.as_deref(),
            payload: self.payload.clone(),
        }
    }
}
