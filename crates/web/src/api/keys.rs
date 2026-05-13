//! API keys: create / list / revoke.

use pastedev_core::{CreateKeyRequest, KeyList, KeyMintedView};
use reqwest::Method;
use uuid::Uuid;

use crate::api::{call, call_unit, HttpError};

pub async fn create(input: &CreateKeyRequest) -> Result<KeyMintedView, HttpError> {
    call(Method::POST, "/api/v1/keys", Some(input)).await
}

pub async fn list() -> Result<KeyList, HttpError> {
    call(Method::GET, "/api/v1/keys", None::<&()>).await
}

pub async fn revoke(id: Uuid) -> Result<(), HttpError> {
    call_unit(
        Method::DELETE,
        &format!("/api/v1/keys/{id}"),
        None::<&()>,
    )
    .await
}
