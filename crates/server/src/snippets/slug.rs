use sqlx::Postgres;

use crate::error::AppError;

use super::repo::{self, SnippetDraft, SnippetRow};

const MAX_RETRIES: usize = 5;

pub async fn create_with_retry<'a>(
    pool: &sqlx::Pool<Postgres>,
    draft: &SnippetDraft<'a>,
) -> Result<SnippetRow, AppError> {
    for attempt in 0..MAX_RETRIES {
        let slug = paste_core::slug::generate();
        match repo::insert(pool, &slug, draft).await {
            Ok(row) => return Ok(row),
            Err(sqlx::Error::Database(db_err))
                if db_err
                    .constraint()
                    .is_some_and(|c| c == "snippets_slug_uniq") =>
            {
                tracing::warn!(attempt, "slug collision, retrying");
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::Conflict("slug space exhausted after retries"))
}
