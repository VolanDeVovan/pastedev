use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service, post},
    Extension,
};
use pastedev::SnippetManager;
use serde_json::json;
use tracing::info;
use std::{net::SocketAddr, io, sync::Arc};

use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
};

pub async fn run_http(addr: SocketAddr, snippet_manager: Arc<SnippetManager>) -> Result<()> {
    info!("Listening http on {}", addr);

    
    let app = axum::Router::new()
        .route("/api", post(create_snippet))
        .route("/api/:snippet_id", get(get_snippet))
        .fallback(
            get_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")))
                .handle_error(handle_error),
        )
        .layer(Extension(snippet_manager))
        .layer(CorsLayer::new().allow_origin(AllowOrigin::any()));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn create_snippet(
    snippet_manager: Extension<Arc<SnippetManager>>,
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
    snippet_manager: Extension<Arc<SnippetManager>>,
    Path(snippet_id): Path<String>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match snippet_manager.get_snippet(&snippet_id).await {
        Ok(text) => Ok(text),
        Err(_err) => Err((StatusCode::NOT_FOUND, "404 Not Found")),
    }
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
