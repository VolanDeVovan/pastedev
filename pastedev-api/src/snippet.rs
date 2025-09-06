use anyhow::Result;
use chrono::{Duration, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Uuid,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub ephemeral: bool,
    pub deleted: bool,
}

impl Snippet {
    pub async fn create_snippet(
        pool: &PgPool,
        content: String,
        ephemeral: bool,
    ) -> Result<Snippet> {
        let expires_at = if ephemeral {
            Some((Utc::now() + Duration::days(7)).naive_utc())
        } else {
            None
        };

        let snippet = sqlx::query_as!(
            Snippet,
            r#"
            INSERT INTO snippets (content, expires_at, ephemeral)
            VALUES ($1, $2, $3)
            RETURNING id, content, created_at, expires_at, ephemeral, deleted
            "#,
            content,
            expires_at,
            ephemeral
        )
        .fetch_one(pool)
        .await?;

        Ok(snippet)
    }

    pub async fn get_snippet_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Snippet>> {
        let snippet = sqlx::query_as!(
            Snippet,
            r#"
            SELECT id, content, created_at, expires_at, ephemeral, deleted
            FROM snippets
            WHERE id = $1 AND deleted = false
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(snippet)
    }

    pub async fn mark_snippet_deleted(pool: &PgPool, id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE snippets
            SET deleted = true
            WHERE id = $1 AND ephemeral = true AND deleted = false
            "#,
            id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_expired_snippets(pool: &PgPool) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM snippets
            WHERE expires_at IS NOT NULL AND expires_at <= (CURRENT_TIMESTAMP AT TIME ZONE 'UTC')
            "#
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
