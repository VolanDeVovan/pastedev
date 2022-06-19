use anyhow::Result;
use pastedev::SnippetManager;
use std::net::SocketAddr;

use tracing::Level;

mod web;

static HOST: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let redis_connection_str =
        std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".to_string());

    let redis_client = redis::Client::open(redis_connection_str)?;

    let snippet_manager = SnippetManager::new(redis_client);

    let addr: SocketAddr = HOST.parse()?;

    web::run_web(addr, snippet_manager.clone()).await?;

    Ok(())
}