use anyhow::Result;
use rand::{distributions::Alphanumeric, Rng};
use redis::{AsyncCommands, Client, RedisError};
use tracing::info;

#[derive(Debug, Clone)]
pub struct SnippetManager {
    redis_client: Client,
}

impl SnippetManager {
    pub fn new(redis_client: Client) -> SnippetManager {
        SnippetManager { redis_client }
    }

    pub async fn get_snippet(&self, snippet_id: &str) -> Result<String, RedisError> {
        info!("Get snippet: {}", snippet_id);
        let mut redis_conn = self.redis_client.get_async_connection().await?;

        redis_conn.get(&snippet_id).await
    }

    pub async fn create_snippet(&self, text: &str) -> Result<String, RedisError> {
        let mut redis_conn = self.redis_client.get_async_connection().await?;

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
            .query_async(&mut redis_conn)
            .await?;

        Ok(random_str)
    }
}
