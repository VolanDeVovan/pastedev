use anyhow::Result;
use sqlx::PgPool;
use tokio::time::{Duration, interval};
use tracing::{error, info};

use crate::snippet::Snippet;

pub async fn start_cleanup_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(300)); // Run every 5 minutes

        loop {
            interval.tick().await;

            match cleanup_expired_snippets(&pool).await {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        info!("Cleaned up {} expired snippets", deleted_count);
                    }
                }
                Err(e) => {
                    error!("Failed to cleanup expired snippets: {}", e);
                }
            }
        }
    });
}

async fn cleanup_expired_snippets(pool: &PgPool) -> Result<u64> {
    Snippet::delete_expired_snippets(pool).await
}
