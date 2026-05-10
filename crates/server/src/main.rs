use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::{prelude::*, EnvFilter};

mod api_keys;
mod assets;
mod audit;
mod auth;
mod config;
mod db;
mod error;
mod http;
mod setup;
mod snippets;
mod users;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env().context("failed to load config")?;
    tracing::info!(
        bind = %config.bind_addr,
        public_base_url = %config.public_base_url,
        "paste-server starting"
    );

    let pool = db::init(&config.database_url)
        .await
        .context("failed to init db")?;

    let state = http::AppState {
        config: Arc::new(config.clone()),
        pool,
        setup_gate: setup::shared_gate(),
    };

    let app = http::router(state.clone());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .with_context(|| format!("failed to bind {}", config.bind_addr))?;
    tracing::info!(local_addr = %listener.local_addr()?, "listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("paste_server=info,tower_http=info,sqlx=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ctrl_c handler") };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("sigterm handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
