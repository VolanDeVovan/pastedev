use anyhow::Result;
use pastedev::SnippetManager;
use std::net::SocketAddr;
use tokio::try_join;
use tracing::Level;

mod socket;
mod web;

// TODO: Make ref to snippet manager instead of copy
// Handle connection error
// Create config interface, not hardcode

static HOST: &str = "0.0.0.0:8080";

static APP_URL: &str = "https://paste.dev.su";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let redis_connection_str =
        std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".to_string());

    let redis_client = redis::Client::open(redis_connection_str)?;

    let snippet_manager = SnippetManager::new(redis_client);

    let web_addr: SocketAddr = HOST.parse()?;
    let socket_addr: SocketAddr = "0.0.0.0:9999".parse()?;

    try_join!(
        socket::run_socket(socket_addr, snippet_manager.clone()),
        web::run_web(web_addr, snippet_manager.clone())
    )?;

    Ok(())
}
