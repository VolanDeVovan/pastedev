use anyhow::Result;
use clap::Parser;
use pastedev::SnippetManager;
use socket::SocketServer;
use std::{net::{IpAddr, SocketAddr}, sync::Arc};
use tokio::try_join;
use tracing::Level;
use url::Url;

mod http;
mod socket;

#[derive(Parser, Debug)]
struct Config {
    /// Application url. Using to generate full snippet url
    #[clap(long, env)]
    app_url: Url,

    /// Redis uri
    #[clap(long, env, default_value = "redis://127.0.0.1/")]
    redis_uri: Url,

    /// Bind address for http and socket servers
    #[clap(env, default_value = "0.0.0.0")]
    host: IpAddr,

    /// Port for http server
    #[clap(env, default_value = "8080")]
    http_port: u16,

    /// Port for socket server
    #[clap(env, default_value = "9999")]
    socket_port: u16,
}


#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let config = Config::parse();

    let redis_manager = bb8_redis::RedisConnectionManager::new(config.redis_uri)?;
    let redis_pool = bb8::Pool::builder().max_size(5).build(redis_manager).await?;

    let snippet_manager = Arc::new(SnippetManager::new(redis_pool));

    let http_addr = SocketAddr::new(config.host, config.http_port);
    let socket_addr = SocketAddr::new(config.host, config.socket_port);

    let socket_server = SocketServer::new(socket_addr, config.app_url, Arc::clone(&snippet_manager));

    try_join!(
        socket_server.run_socket(),
        http::run_http(http_addr, Arc::clone(&snippet_manager)),
    )?;

    Ok(())
}
