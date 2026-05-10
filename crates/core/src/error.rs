use serde::{Deserialize, Serialize};

/// Stable string identifier for every error the server emits. See
/// `plan/05-api.html#errors` for the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ValidationError,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    SetupRequired,
    SetupComplete,
    SnippetTooLarge,
    RateLimited,
    Internal,
}

impl ErrorCode {
    pub fn http_status(self) -> u16 {
        match self {
            ErrorCode::ValidationError => 400,
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::SetupRequired => 403,
            ErrorCode::SetupComplete => 409,
            ErrorCode::SnippetTooLarge => 413,
            ErrorCode::RateLimited => 429,
            ErrorCode::Internal => 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}
