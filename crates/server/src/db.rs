use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn init(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Quick liveness check used by `/api/v1/health`.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("SELECT 1 AS \"one!\"")
        .fetch_one(pool)
        .await?;
    Ok(())
}
