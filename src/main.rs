use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{self, IntoResponse},
    routing::{get, get_service, post},
    Extension,
};
use rand::{distributions::Alphanumeric, Rng};
use redis::{AsyncCommands, Client};
use serde_json::{json, Value};
use tracing::info;
use std::{io, net::SocketAddr};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
};

static HOST: &str = "0.0.0.0:8080";

// TODO: Add error handler and redis reconnect
// TODO: Add logging

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let redis_connection_str =
        std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".to_string());

    let redis_client = redis::Client::open(redis_connection_str)?;

    let app = axum::Router::new()
        .route("/api", post(create_snippet))
        .route("/api/:snippet_id", get(get_snippet))
        .fallback(
            get_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
                .handle_error(handle_error),
        )
        .layer(Extension(redis_client))
        .layer(CorsLayer::new().allow_origin(AllowOrigin::any()));

    let addr: SocketAddr = HOST.parse()?;

    info!("Listening on {}", HOST);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn create_snippet(
    Extension(redis_client): Extension<Client>,
    text: String,
) -> response::Json<Value> {
    let mut redis_conn = redis_client.get_async_connection().await.unwrap();

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
        .await
        .unwrap();

    axum::Json(json!({ "snippet_id": random_str }))
}

async fn get_snippet(
    Extension(redis_client): Extension<Client>,
    Path(snippet_id): Path<String>,
) -> (StatusCode, String) {
    info!("Get snippet: {}", snippet_id)
    let mut redis_conn = redis_client.get_async_connection().await.unwrap();

    match redis_conn.get(&snippet_id).await {
        Ok(text) => (StatusCode::OK, text),
        Err(_) => (StatusCode::NOT_FOUND, format!("Not found: {}", snippet_id)),
    }
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
