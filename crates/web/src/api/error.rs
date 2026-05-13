//! Shape mirrors the server's `ErrorEnvelope { error: { code, message, details? } }`.

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ErrorEnvelope {
    pub error: ApiError,
}

#[derive(Clone, Debug)]
pub struct HttpError {
    pub status: u16,
    pub error: ApiError,
}

impl HttpError {
    pub fn code(&self) -> &str {
        &self.error.code
    }
    pub fn message(&self) -> &str {
        &self.error.message
    }
    pub fn is_forbidden(&self) -> bool {
        self.status == 403
    }
    pub fn network(e: reqwest::Error) -> Self {
        Self {
            status: 0,
            error: ApiError {
                code: "network".to_string(),
                message: format!("network error: {e}"),
                details: None,
            },
        }
    }
    pub fn decode(status: u16, e: serde_json::Error) -> Self {
        Self {
            status,
            error: ApiError {
                code: "decode".to_string(),
                message: format!("decode error: {e}"),
                details: None,
            },
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error.code, self.error.message)
    }
}
