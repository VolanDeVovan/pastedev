use anyhow::Result;
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use rand::{distributions::Alphanumeric, Rng};
use redis::{AsyncCommands, Client, RedisError};
use tracing::info;


#[derive(Debug, Clone)]
pub struct SnippetManager {
    redis_pool: Pool<RedisConnectionManager>,
}

impl SnippetManager {
    pub fn new(redis_pool: Pool<RedisConnectionManager>) -> SnippetManager {
        SnippetManager { redis_pool }
    }

    pub async fn get_snippet(&self, snippet_id: &str) -> Result<String, RedisError> {
        info!("Get snippet: {}", snippet_id);

        let redis_pool = self.redis_pool.clone();
        let mut redis_conn = redis_pool.get().await.unwrap();

        redis_conn.get(&snippet_id).await
    }

    pub async fn create_snippet(&self, text: &str) -> Result<String, RedisError> {
        let redis_pool = self.redis_pool.clone();
        let mut redis_conn = redis_pool.get().await.unwrap();

        let random_str: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        info!("Save new snippet: {}", random_str);

        let duration_secs = 60 * 60 * 24 * 14;

        let _: () = redis::cmd("SET")
            .arg(&random_str)
            .arg(&text)
            .arg("EX")
            .arg(duration_secs)
            .query_async(&mut *redis_conn)
            .await?;

        Ok(random_str)
    }
}
