use anyhow::Result;
use thiserror::Error;
use bb8::{Pool, RunError};
use bb8_redis::RedisConnectionManager;
use rand::{distributions::Alphanumeric, Rng};
use redis::{AsyncCommands, RedisError};
use tracing::info;


#[derive(Error, Debug)]
pub enum SnippetManagerError {
    #[error("Redis error")]
    RedisError(#[from] RedisError),

    #[error("Redis pool error")]
    PoolError(#[from] RunError<RedisError>)
}

#[derive(Debug, Clone)]
pub struct SnippetManager {
    redis_pool: Pool<RedisConnectionManager>,
}

impl SnippetManager {
    pub fn new(redis_pool: Pool<RedisConnectionManager>) -> SnippetManager {
        SnippetManager { redis_pool }
    }

    pub async fn get_snippet(&self, snippet_id: &str) -> Result<String, SnippetManagerError> {
        info!("Get snippet: {}", snippet_id);

        let mut redis_conn = self.redis_pool.get().await?;

        Ok(redis_conn.get(&snippet_id).await?)
    }

    pub async fn create_snippet(&self, text: &str) -> Result<String, SnippetManagerError> {
        let mut redis_conn = self.redis_pool.get().await?;

        let random_str: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        info!("Save new snippet: {}", random_str);

        // let duration_secs = 60 * 60 * 24 * 14;

        // let _: () = redis::cmd("SET")
        //     .arg(&random_str)
        //     .arg(&text)
        //     .arg("EX")
        //     .arg(duration_secs)
        //     .query_async(&mut *redis_conn)
        //     .await?;

        redis_conn.set(&random_str, &text).await?;
        

        Ok(random_str)
    }
}
