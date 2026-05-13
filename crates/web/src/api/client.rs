//! reqwest-WASM wrapper. Every request sends credentials (cookies) because the
//! SPA's auth model is a session cookie; bearer tokens are reserved for the CLI.

use std::sync::OnceLock;

use reqwest::{Client, Method, Response, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

use crate::api::error::{ApiError, ErrorEnvelope, HttpError};
use crate::config::config;

fn client() -> &'static Client {
    static C: OnceLock<Client> = OnceLock::new();
    C.get_or_init(|| Client::builder().build().expect("reqwest::Client::builder"))
}

fn url(path: &str) -> String {
    let base = &config().api_base_url;
    if !base.is_empty() {
        return format!("{base}{path}");
    }
    // Empty api_base_url means same-origin: prepend window.location.origin so
    // reqwest's WASM backend (which requires absolute URLs) is happy.
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    format!("{origin}{path}")
}

/// Generic JSON-in / JSON-out call. Body is optional.
pub async fn call<T, B>(method: Method, path: &str, body: Option<&B>) -> Result<T, HttpError>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let mut req = client()
        .request(method, url(path))
        .fetch_credentials_include();
    if let Some(b) = body {
        req = req.header("content-type", "application/json").json(b);
    }
    let resp = req.send().await.map_err(HttpError::network)?;
    parse(resp).await
}

/// For endpoints that return 204 No Content (logout, delete, …).
pub async fn call_unit<B>(method: Method, path: &str, body: Option<&B>) -> Result<(), HttpError>
where
    B: Serialize + ?Sized,
{
    let mut req = client()
        .request(method, url(path))
        .fetch_credentials_include();
    if let Some(b) = body {
        req = req.header("content-type", "application/json").json(b);
    }
    let resp = req.send().await.map_err(HttpError::network)?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(decode_error(status, &text));
    }
    Ok(())
}

async fn parse<T: DeserializeOwned>(resp: Response) -> Result<T, HttpError> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(decode_error(status, &text));
    }
    if text.is_empty() {
        // 204 with a typed T isn't valid; callers should use call_unit. Surface
        // as a decode error rather than panicking.
        return Err(HttpError {
            status: status.as_u16(),
            error: ApiError {
                code: "empty_body".to_string(),
                message: "expected JSON response, got empty body".to_string(),
                details: None,
            },
        });
    }
    serde_json::from_str(&text).map_err(|e| HttpError::decode(status.as_u16(), e))
}

fn decode_error(status: StatusCode, text: &str) -> HttpError {
    let err: ApiError = serde_json::from_str::<ErrorEnvelope>(text)
        .map(|e| e.error)
        .unwrap_or_else(|_| ApiError {
            code: "unknown".to_string(),
            message: if text.is_empty() {
                format!("HTTP {}", status)
            } else {
                text.to_string()
            },
            details: None,
        });
    HttpError {
        status: status.as_u16(),
        error: err,
    }
}
