//! Axum extractors for authenticated routes.
//!
//! - `SessionUser` — cookie only.
//! - `ApiKeyUser` — bearer only.
//! - `AuthUser` — cookie or bearer.
//! - `ApprovedUser` — adds the `status === 'approved'` gate.
//! - `AdminUser` — adds the `role === 'admin'` gate.
//! - `RequiresScope<S>` — wraps `AuthUser`; for bearer auth, requires the scope.

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderMap},
};
use paste_core::{Role, Scope, UserStatus};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, http::AppState};

#[derive(Debug, Clone)]
pub struct AuthedUser {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub status: UserStatus,
    /// Empty when authenticated by session. For bearer-auth, the scope set
    /// granted to that key.
    pub key_scopes: Vec<Scope>,
    /// True when this auth came from a bearer, not a cookie. Routes that need
    /// to mint new keys reject bearer-auth here.
    pub via_bearer: bool,
    pub key_id: Option<Uuid>,
}

pub struct SessionUser(pub AuthedUser);
pub struct ApiKeyUser(pub AuthedUser);
pub struct AuthUser(pub AuthedUser);
pub struct ApprovedUser(pub AuthedUser);
pub struct AdminUser(pub AuthedUser);
pub struct MaybeAuthUser(pub Option<AuthedUser>);

/// `RequiresScope<S>` — session auth always passes; bearer auth must hold S.
pub struct RequiresScope<const S: u8>(pub AuthedUser);

/// Encoded scope discriminator for `RequiresScope` const generic. Keep in sync
/// with `paste_core::Scope`.
pub mod scope_id {
    pub const PUBLISH: u8 = 1;
    pub const READ: u8 = 2;
    pub const DELETE: u8 = 3;
}

fn scope_for(id: u8) -> Option<Scope> {
    match id {
        scope_id::PUBLISH => Some(Scope::Publish),
        scope_id::READ => Some(Scope::Read),
        scope_id::DELETE => Some(Scope::Delete),
        _ => None,
    }
}

fn parse_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for piece in raw.split(';') {
        let piece = piece.trim();
        if let Some((name, value)) = piece.split_once('=') {
            if name == paste_core::SESSION_COOKIE_NAME {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bearer(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("Bearer ") {
        return Some(rest.trim().to_string());
    }
    if let Some(rest) = raw.strip_prefix("bearer ") {
        return Some(rest.trim().to_string());
    }
    None
}

async fn load_user_skel(pool: &PgPool, id: Uuid) -> Result<AuthedUser, AppError> {
    let row = sqlx::query!(
        "SELECT id, username, role, status FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(AppError::Unauthorized);
    };
    let role = Role::from_str_opt(&row.role).ok_or(AppError::Unauthorized)?;
    let status = UserStatus::from_str_opt(&row.status).ok_or(AppError::Unauthorized)?;
    Ok(AuthedUser {
        id: row.id,
        username: row.username,
        role,
        status,
        key_scopes: Vec::new(),
        via_bearer: false,
        key_id: None,
    })
}

async fn cookie_user(state: &AppState, headers: &HeaderMap) -> Option<AuthedUser> {
    let value = parse_cookie(headers)?;
    let id_bytes = super::session::decode_cookie(&value)?;
    let session = super::session::validate(&state.pool, &id_bytes).await.ok()??;
    let _ = super::session::maybe_renew(&state.pool, &state.config, &session).await;
    load_user_skel(&state.pool, session.user_id).await.ok()
}

async fn bearer_user(state: &AppState, headers: &HeaderMap) -> Option<AuthedUser> {
    let token = parse_bearer(headers)?;
    let verified = super::api_key::verify(&state.pool, &token).await.ok()??;
    let mut user = load_user_skel(&state.pool, verified.user_id).await.ok()?;
    user.via_bearer = true;
    user.key_id = Some(verified.key_id);
    user.key_scopes = verified.scopes;
    if user.status == UserStatus::Suspended || user.status == UserStatus::Rejected {
        return None;
    }
    Some(user)
}


impl FromRequestParts<AppState> for SessionUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        match cookie_user(state, &parts.headers).await {
            Some(u) => Ok(SessionUser(u)),
            None => Err(AppError::Unauthorized),
        }
    }
}


impl FromRequestParts<AppState> for ApiKeyUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        match bearer_user(state, &parts.headers).await {
            Some(u) => Ok(ApiKeyUser(u)),
            None => Err(AppError::Unauthorized),
        }
    }
}


impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        if let Some(u) = cookie_user(state, &parts.headers).await {
            return Ok(AuthUser(u));
        }
        if let Some(u) = bearer_user(state, &parts.headers).await {
            return Ok(AuthUser(u));
        }
        Err(AppError::Unauthorized)
    }
}


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


impl FromRequestParts<AppState> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(u) = cookie_user(state, &parts.headers).await {
            return Ok(MaybeAuthUser(Some(u)));
        }
        if let Some(u) = bearer_user(state, &parts.headers).await {
            return Ok(MaybeAuthUser(Some(u)));
        }
        Ok(MaybeAuthUser(None))
    }
}


impl<const S: u8> FromRequestParts<AppState> for RequiresScope<S> {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let AuthUser(u) = AuthUser::from_request_parts(parts, state).await?;
        if u.status != UserStatus::Approved {
            return Err(AppError::ForbiddenWith("not approved"));
        }
        if !u.via_bearer {
            // Session auth implicitly carries every scope for the user's own resources.
            return Ok(RequiresScope(u));
        }
        let needed = scope_for(S).ok_or(AppError::Forbidden)?;
        if !u.key_scopes.contains(&needed) {
            return Err(AppError::ForbiddenWith("scope missing"));
        }
        Ok(RequiresScope(u))
    }
}
