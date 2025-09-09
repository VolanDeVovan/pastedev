use anyhow::Result;

use clap::Parser;
use sqlx::PgPool;
use tokio::net::TcpListener;

use std::net::IpAddr;
use tracing::{Level, info};
use tracing_subscriber;
use url::Url;

use pastedev_api::cleanup::start_cleanup_task;
use pastedev_api::routes::{AppState, create_router};

#[derive(Parser, Debug)]
struct Config {
    /// Application url. Using to generate full snippet url
    #[clap(long, env)]
    app_url: Url,

    /// Bind address for http and socket servers
    #[clap(env, default_value = "0.0.0.0")]
    host: IpAddr,

    /// Port for http server
    #[clap(env, default_value = "8080")]
    http_port: u16,

    /// Port for socket server
    #[clap(env, default_value = "9999")]
    socket_port: u16,

    /// Postgres connection string
    #[clap(env, default_value = "postgres://app:12345q@localhost:5432/app")]
    database_url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let config = Config::try_parse()?;

    info!("Connecting to database {}", config.database_url.to_string());
    let pool = PgPool::connect(&config.database_url.to_string()).await?;

    info!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;

    let state = AppState {
        db: pool.clone(),
        app_url: config.app_url.to_string(),
    };

    start_cleanup_task(pool.clone()).await;

    let app = create_router(state);

    let bind_addr = format!("{}:{}", config.host, config.http_port);
    info!("Starting HTTP server on {}", bind_addr);

    let listener = TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
