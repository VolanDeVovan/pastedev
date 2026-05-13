//! Auth + setup endpoints.

use pastedev_core::{
    LoginRequest, RegisterRequest, SetupAdminRequest, SetupAdminResponse, SetupStatus, UserEnvelope,
    UserPublic,
};
use reqwest::Method;

use crate::api::{call, call_unit, HttpError};

pub async fn setup_status() -> Result<SetupStatus, HttpError> {
    call(Method::GET, "/api/v1/setup/status", None::<&()>).await
}

pub async fn create_first_admin(input: &SetupAdminRequest) -> Result<SetupAdminResponse, HttpError> {
    call(Method::POST, "/api/v1/setup/admin", Some(input)).await
}

pub async fn me() -> Result<UserPublic, HttpError> {
    call(Method::GET, "/api/v1/auth/me", None::<&()>).await
}

pub async fn login(input: &LoginRequest) -> Result<UserPublic, HttpError> {
    let env: UserEnvelope = call(Method::POST, "/api/v1/auth/login", Some(input)).await?;
    Ok(env.user)
}

pub async fn register(input: &RegisterRequest) -> Result<UserPublic, HttpError> {
    let env: UserEnvelope = call(Method::POST, "/api/v1/auth/register", Some(input)).await?;
    Ok(env.user)
}

pub async fn logout() -> Result<(), HttpError> {
    call_unit(Method::POST, "/api/v1/auth/logout", None::<&()>).await
}
