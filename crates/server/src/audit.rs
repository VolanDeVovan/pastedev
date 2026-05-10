use ipnetwork::IpNetwork;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Append a row to `audit_log`. Best-effort: returns an error but callers can
/// log+ignore — never block a user action because the audit insert failed.
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

pub async fn write(pool: &PgPool, event: Event<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
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
    .await?;
    Ok(())
}

pub fn log_err(action: &str, e: sqlx::Error) {
    tracing::warn!(action, error = ?e, "audit log write failed");
}
