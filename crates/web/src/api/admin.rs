//! Admin queue actions.

use pastedev_core::{
    AdminListResponse, ResetPasswordRequest, UserMutationResponse, UserPublic, UserStatus,
};
use reqwest::Method;
use uuid::Uuid;

use crate::api::{call, call_unit, HttpError};

pub async fn list(status: Option<UserStatus>) -> Result<AdminListResponse, HttpError> {
    let qs = status
        .map(|s| format!("?status={}", s.as_str()))
        .unwrap_or_default();
    call(
        Method::GET,
        &format!("/api/v1/admin/users{qs}"),
        None::<&()>,
    )
    .await
}

async fn user_action(id: Uuid, action: &str) -> Result<UserPublic, HttpError> {
    let env: UserMutationResponse = call(
        Method::POST,
        &format!("/api/v1/admin/users/{id}/{action}"),
        None::<&()>,
    )
    .await?;
    Ok(env.user)
}

pub async fn approve(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "approve").await }
pub async fn reject(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "reject").await }
pub async fn suspend(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "suspend").await }
pub async fn restore(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "restore").await }
pub async fn promote(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "promote").await }
pub async fn demote(id: Uuid) -> Result<UserPublic, HttpError> { user_action(id, "demote").await }

pub async fn reset_password(id: Uuid, new_password: String) -> Result<(), HttpError> {
    call_unit(
        Method::POST,
        &format!("/api/v1/admin/users/{id}/reset_password"),
        Some(&ResetPasswordRequest { new_password }),
    )
    .await
}
