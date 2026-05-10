use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use paste_core::{ErrorBody, ErrorCode, ErrorEnvelope};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    /// `None` is a bare "forbidden"; `Some("reason")` produces "forbidden: reason".
    #[error("{}", forbidden_message(*.0))]
    Forbidden(Option<&'static str>),
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(&'static str),
    #[error("setup_required")]
    SetupRequired,
    #[error("setup_complete")]
    SetupComplete,
    #[error("snippet too large: {size} > {limit}")]
    SnippetTooLarge { size: usize, limit: usize },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

fn forbidden_message(reason: Option<&'static str>) -> String {
    match reason {
        Some(r) => format!("forbidden: {r}"),
        None => "forbidden".into(),
    }
}

impl AppError {
    fn code(&self) -> ErrorCode {
        match self {
            AppError::Validation(_) => ErrorCode::ValidationError,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::Forbidden(_) => ErrorCode::Forbidden,
            AppError::NotFound => ErrorCode::NotFound,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::SetupRequired => ErrorCode::SetupRequired,
            AppError::SetupComplete => ErrorCode::SetupComplete,
            AppError::SnippetTooLarge { .. } => ErrorCode::SnippetTooLarge,
            AppError::Sqlx(_) | AppError::Other(_) => ErrorCode::Internal,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = self.code();
        let status =
            StatusCode::from_u16(code.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let message = match &self {
            AppError::Sqlx(e) => {
                tracing::error!(error = ?e, "sqlx error");
                "internal error".to_string()
            }
            AppError::Other(e) => {
                tracing::error!(error = ?e, "internal error");
                "internal error".to_string()
            }
            _ => self.to_string(),
        };
        let details = match &self {
            AppError::SnippetTooLarge { size, limit } => Some(serde_json::json!({
                "size_bytes": size,
                "limit_bytes": limit,
            })),
            _ => None,
        };
        let body = ErrorEnvelope {
            error: ErrorBody {
                code,
                message,
                details,
            },
        };
        (status, Json(body)).into_response()
    }
}
