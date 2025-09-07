use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;

use crate::snippet::Snippet;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub app_url: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_snippet))
        .route("/api/snippets", post(create_snippet))
        .route("/api/snippets/{id}", get(get_snippet))
        .with_state(state)
}

async fn create_snippet(State(state): State<AppState>, body: String) -> Result<String, AppError> {
    if body.trim().is_empty() {
        return Err(AppError::BadRequest("Content cannot be empty".to_string()));
    }

    let snippet = Snippet::create_snippet(&state.db, body, true).await?;

    let url = format!("{}/{}", state.app_url.trim_end_matches('/'), snippet.alias);
    Ok(url)
}

async fn get_snippet(
    State(state): State<AppState>,
    Path(alias): Path<String>,
) -> Result<String, AppError> {
    let snippet = Snippet::get_snippet_by_alias(&state.db, &alias).await?;

    match snippet {
        Some(snippet) => {
            if snippet.ephemeral && snippet.expires_at.is_none() {
                let expires_at = (Utc::now() + Duration::minutes(15)).naive_utc();
                Snippet::set_expiry_time(&state.db, snippet.id, expires_at).await?;
            }

            Ok(snippet.content)
        }
        None => Err(AppError::NotFound),
    }
}

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound,
    BadRequest(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::BadRequest(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
        };

        let body = match self {
            AppError::Database(err) => format!("Database error: {}", err),
            AppError::NotFound => "Snippet not found".to_string(),
            AppError::BadRequest(msg) => msg,
        };

        (status, body).into_response()
    }
}
