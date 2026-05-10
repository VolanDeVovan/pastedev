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
    #[error("forbidden")]
    Forbidden,
    #[error("forbidden: {0}")]
    ForbiddenWith(&'static str),
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
    #[error("rate limited")]
    RateLimited { retry_after: u64 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    fn code(&self) -> ErrorCode {
        match self {
            AppError::Validation(_) => ErrorCode::ValidationError,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::Forbidden | AppError::ForbiddenWith(_) => ErrorCode::Forbidden,
            AppError::NotFound => ErrorCode::NotFound,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::SetupRequired => ErrorCode::SetupRequired,
            AppError::SetupComplete => ErrorCode::SetupComplete,
            AppError::SnippetTooLarge { .. } => ErrorCode::SnippetTooLarge,
            AppError::RateLimited { .. } => ErrorCode::RateLimited,
            AppError::Sqlx(_) | AppError::Other(_) => ErrorCode::Internal,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = self.code();
        let status = StatusCode::from_u16(code.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
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
            AppError::RateLimited { retry_after } => Some(serde_json::json!({
                "retry_after": retry_after,
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
        let mut response = (status, Json(body)).into_response();
        if let AppError::RateLimited { retry_after } = self {
            if let Ok(v) = retry_after.to_string().parse() {
                response.headers_mut().insert("retry-after", v);
            }
        }
        response
    }
}

pub type AppResult<T> = Result<T, AppError>;
