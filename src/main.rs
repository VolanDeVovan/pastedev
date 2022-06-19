use anyhow::Result;
use pastedev::SnippetManager;
use serde_json::json;
use std::{io, net::SocketAddr};
use axum::{
    extract::Path,
    http::StatusCode,
    response:: IntoResponse,
    routing::{get, get_service, post},
    Extension,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
};
use tracing::{info, Level};

static HOST: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let redis_connection_str =
        std::env::var("REDIS_URL").unwrap_or("redis://127.0.0.1/".to_string());

    let redis_client = redis::Client::open(redis_connection_str)?;

    let snippet_manager = SnippetManager::new(redis_client);

    let app = axum::Router::new()
        .route("/api", post(create_snippet))
        .route("/api/:snippet_id", get(get_snippet))
        .fallback(
            get_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
                .handle_error(handle_error),
        )
        .layer(Extension(snippet_manager))
        .layer(CorsLayer::new().allow_origin(AllowOrigin::any()));

    let addr: SocketAddr = HOST.parse()?;

    info!("Listening on {}", HOST);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn create_snippet(
    Extension(snippet_manager): Extension<SnippetManager>,
    text: String,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match snippet_manager.create_snippet(&text).await {
        Ok(snippet_id) => Ok(axum::Json(json!({ "snippet_id": snippet_id }))),

        Err(_err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )),
    }
}

async fn get_snippet(
    Extension(snippet_manager): Extension<SnippetManager>,
    Path(snippet_id): Path<String>,
) -> Result<impl IntoResponse, impl IntoResponse>{

    match snippet_manager.get_snippet(&snippet_id).await {
        Ok(text) => Ok(text),
        Err(_err) => Err((StatusCode::NOT_FOUND, "404 Not Found")),
    }
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
