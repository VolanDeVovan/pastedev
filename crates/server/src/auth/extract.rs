//! Axum extractors for authenticated routes.
//!
//! - `SessionUser` — cookie only.
//! - `AuthUser` — cookie now; bearer extractor lands in phase 4.
//! - `ApprovedUser` — adds the `status === 'approved'` gate.
//! - `AdminUser` — adds the `role === 'admin'` gate.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use paste_core::{Role, UserStatus};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, http::AppState};

#[derive(Debug, Clone)]
pub struct AuthedUser {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub status: UserStatus,
}

/// Cookie-only authenticated user.
pub struct SessionUser(pub AuthedUser);

/// Cookie or bearer (bearer added in phase 4).
pub struct AuthUser(pub AuthedUser);

/// AuthUser + status == approved.
pub struct ApprovedUser(pub AuthedUser);

/// AuthUser + role == admin.
pub struct AdminUser(pub AuthedUser);

/// Optional auth — never errors.
pub struct MaybeAuthUser(pub Option<AuthedUser>);

fn parse_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for piece in raw.split(';') {
        let piece = piece.trim();
        let Some((name, value)) = piece.split_once('=') else {
            continue;
        };
        if name == paste_core::SESSION_COOKIE_NAME {
            return Some(value.to_string());
        }
    }
    None
}

async fn load_user(pool: &PgPool, id: Uuid) -> Result<AuthedUser, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT id, username, role, status FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let Some((id, username, role_s, status_s)) = row else {
        return Err(AppError::Unauthorized);
    };
    let role = Role::from_str_opt(&role_s).ok_or(AppError::Unauthorized)?;
    let status = UserStatus::from_str_opt(&status_s).ok_or(AppError::Unauthorized)?;
    Ok(AuthedUser {
        id,
        username,
        role,
        status,
    })
}

async fn cookie_user(state: &AppState, headers: &HeaderMap) -> Option<AuthedUser> {
    let value = parse_cookie(headers)?;
    let id_bytes = super::session::decode_cookie(&value)?;
    let session = super::session::validate(&state.pool, &id_bytes).await.ok()??;
    let _ = super::session::maybe_renew(&state.pool, &state.config, &session).await;
    load_user(&state.pool, session.user_id).await.ok()
}

#[async_trait]
impl FromRequestParts<AppState> for SessionUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        match cookie_user(state, &parts.headers).await {
            Some(u) => Ok(SessionUser(u)),
            None => Err(AppError::Unauthorized),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        // Phase 4 will add bearer fallback here.
        match cookie_user(state, &parts.headers).await {
            Some(u) => Ok(AuthUser(u)),
            None => Err(AppError::Unauthorized),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for ApprovedUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let AuthUser(u) = AuthUser::from_request_parts(parts, state).await?;
        if u.status != UserStatus::Approved {
            return Err(AppError::ForbiddenWith("not approved"));
        }
        Ok(ApprovedUser(u))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let AuthUser(u) = AuthUser::from_request_parts(parts, state).await?;
        if u.role != Role::Admin {
            return Err(AppError::ForbiddenWith("admin only"));
        }
        if u.status != UserStatus::Approved {
            return Err(AppError::ForbiddenWith("not approved"));
        }
        Ok(AdminUser(u))
    }
}

#[async_trait]
impl FromRequestParts<AppState> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuthUser(cookie_user(state, &parts.headers).await))
    }
}
