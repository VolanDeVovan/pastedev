use anyhow::Result;
use chrono::NaiveDateTime;
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Uuid,
    pub alias: String,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
    pub ephemeral: bool,
    pub deleted: bool,
}

fn generate_random_alias() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}

async fn generate_unique_alias(pool: &PgPool) -> Result<String> {
    const MAX_ATTEMPTS: u8 = 10;

    for _ in 0..MAX_ATTEMPTS {
        let alias = generate_random_alias();

        // Check if alias exists
        let exists = sqlx::query!("SELECT 1 as exists FROM snippets WHERE alias = $1", alias)
            .fetch_optional(pool)
            .await?
            .is_some();

        if !exists {
            return Ok(alias);
        }
    }

    Err(anyhow::anyhow!(
        "Failed to generate unique alias after {} attempts",
        MAX_ATTEMPTS
    ))
}

impl Snippet {
    pub async fn create_snippet(
        pool: &PgPool,
        content: String,
        ephemeral: bool,
    ) -> Result<Snippet> {
        let expires_at: Option<NaiveDateTime> = None;
        let alias = generate_unique_alias(pool).await?;

        let snippet = sqlx::query_as!(
            Snippet,
            r#"
            INSERT INTO snippets (alias, content, expires_at, ephemeral)
            VALUES ($1, $2, $3, $4)
            RETURNING id, alias, content, created_at, expires_at, ephemeral, deleted
            "#,
            alias,
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
            SELECT id, alias, content, created_at, expires_at, ephemeral, deleted
            FROM snippets
            WHERE id = $1 AND deleted = false
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        Ok(snippet)
    }

    pub async fn get_snippet_by_alias(pool: &PgPool, alias: &str) -> Result<Option<Snippet>> {
        let snippet = sqlx::query_as!(
            Snippet,
            r#"
            SELECT id, alias, content, created_at, expires_at, ephemeral, deleted
            FROM snippets
            WHERE alias = $1 AND deleted = false
            "#,
            alias
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

    pub async fn set_expiry_time(
        pool: &PgPool,
        id: Uuid,
        expires_at: NaiveDateTime,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE snippets
            SET expires_at = $1
            WHERE id = $2 AND ephemeral = true AND expires_at IS NULL
            "#,
            expires_at,
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
