use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use paste_core::{Role, UserPublic, UserStatus};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit,
    auth::{
        extract::AdminUser,
        password, session,
    },
    error::AppError,
    http::AppState,
    users::repo,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AdminUserView {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub reason: Option<String>,
    pub registration_ip: Option<String>,
    pub status: UserStatus,
    pub role: Role,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub items: Vec<AdminUserView>,
    pub next_cursor: Option<String>,
}

fn to_admin_view(row: &repo::UserRow) -> AdminUserView {
    AdminUserView {
        id: row.id,
        username: row.username.clone(),
        email: row.email.clone(),
        reason: row.reason.clone(),
        registration_ip: row.registration_ip.map(|ip| ip.ip().to_string()),
        status: row.status,
        role: row.role,
        created_at: row.created_at,
    }
}

/// `GET /api/v1/admin/users`
pub async fn list_users(
    AdminUser(_): AdminUser,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let status = q
        .status
        .as_deref()
        .map(|s| s.parse::<UserStatus>().map_err(|_| AppError::Validation("invalid status".into())))
        .transpose()?;
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = repo::list(&state.pool, status, limit).await?;
    Ok(Json(ListResponse {
        items: rows.iter().map(to_admin_view).collect(),
        next_cursor: None,
    }))
}

#[derive(Debug, Serialize)]
pub struct UserMutationResponse {
    pub user: UserPublic,
}

fn to_public(row: &repo::UserRow) -> UserPublic {
    UserPublic {
        id: row.id,
        username: row.username.clone(),
        role: row.role,
        status: row.status,
        created_at: row.created_at,
    }
}

async fn fetch_or_404(state: &AppState, id: Uuid) -> Result<repo::UserRow, AppError> {
    repo::by_id(&state.pool, id).await?.ok_or(AppError::NotFound)
}

async fn audit_user(
    state: &AppState,
    actor: Uuid,
    target: Uuid,
    event: &str,
    payload: Option<serde_json::Value>,
) {
    audit::write(
        &state.pool,
        audit::Event {
            event,
            actor_user_id: Some(actor),
            target_user_id: Some(target),
            payload,
            ..Default::default()
        },
    )
    .await;
}

async fn ensure_last_admin_invariant(
    state: &AppState,
    target: &repo::UserRow,
    next_role: Role,
    next_status: UserStatus,
) -> Result<(), AppError> {
    // If the mutation would leave the target as a non-admin or non-approved,
    // make sure another admin remains.
    let was_active_admin = target.role == Role::Admin && target.status == UserStatus::Approved;
    let will_be_active = next_role == Role::Admin && next_status == UserStatus::Approved;
    if !was_active_admin || will_be_active {
        return Ok(());
    }
    let total = repo::count_active_admins(&state.pool).await?;
    if total <= 1 {
        return Err(AppError::Conflict("last admin"));
    }
    Ok(())
}

/// `POST /api/v1/admin/users/:id/approve` — flip status from pending to approved.
pub async fn approve(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let _target = fetch_or_404(&state, id).await?;
    let updated = repo::set_status(&state.pool, id, UserStatus::Approved)
        .await?
        .ok_or(AppError::NotFound)?;
    audit_user(&state, actor.id, id, "user.approve", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

/// `POST /api/v1/admin/users/:id/reject` — mark as rejected and revoke sessions.
pub async fn reject(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let target = fetch_or_404(&state, id).await?;
    ensure_last_admin_invariant(&state, &target, target.role, UserStatus::Rejected).await?;
    let updated = repo::set_status(&state.pool, id, UserStatus::Rejected)
        .await?
        .ok_or(AppError::NotFound)?;
    let _ = session::revoke_all_for_user(&state.pool, id).await;
    audit_user(&state, actor.id, id, "user.reject", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

/// `POST /api/v1/admin/users/:id/suspend` — mark as suspended and revoke sessions.
pub async fn suspend(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let target = fetch_or_404(&state, id).await?;
    ensure_last_admin_invariant(&state, &target, target.role, UserStatus::Suspended).await?;
    let updated = repo::set_status(&state.pool, id, UserStatus::Suspended)
        .await?
        .ok_or(AppError::NotFound)?;
    let _ = session::revoke_all_for_user(&state.pool, id).await;
    audit_user(&state, actor.id, id, "user.suspend", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

/// `POST /api/v1/admin/users/:id/restore` — move a rejected/suspended account back to approved.
pub async fn restore(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let _ = fetch_or_404(&state, id).await?;
    let updated = repo::set_status(&state.pool, id, UserStatus::Approved)
        .await?
        .ok_or(AppError::NotFound)?;
    audit_user(&state, actor.id, id, "user.restore", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

/// `POST /api/v1/admin/users/:id/promote` — grant the admin role.
pub async fn promote(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let _ = fetch_or_404(&state, id).await?;
    let updated = repo::set_role(&state.pool, id, Role::Admin)
        .await?
        .ok_or(AppError::NotFound)?;
    audit_user(&state, actor.id, id, "user.promote", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

/// `POST /api/v1/admin/users/:id/demote` — revoke the admin role. Refuses to demote the last active admin.
pub async fn demote(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserMutationResponse>, AppError> {
    let target = fetch_or_404(&state, id).await?;
    ensure_last_admin_invariant(&state, &target, Role::User, target.status).await?;
    let updated = repo::set_role(&state.pool, id, Role::User)
        .await?
        .ok_or(AppError::NotFound)?;
    audit_user(&state, actor.id, id, "user.demote", None).await;
    Ok(Json(UserMutationResponse { user: to_public(&updated) }))
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

/// `POST /api/v1/admin/users/:id/reset_password` — set a fresh password and revoke sessions.
pub async fn reset_password(
    AdminUser(actor): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Response, AppError> {
    let _ = fetch_or_404(&state, id).await?;
    if req.new_password.len() < 12 {
        return Err(AppError::Validation(
            "new_password must be at least 12 characters".into(),
        ));
    }
    let phc = password::hash(
        &req.new_password,
        state.config.argon2_m_kib,
        state.config.argon2_t_cost,
    )
    .map_err(|e| AppError::Validation(format!("password hashing: {e}")))?;
    repo::set_password_hash(&state.pool, id, &phc).await?;
    let _ = session::revoke_all_for_user(&state.pool, id).await;
    audit_user(&state, actor.id, id, "user.password_reset", None).await;
    Ok((StatusCode::NO_CONTENT, ()).into_response())
}
