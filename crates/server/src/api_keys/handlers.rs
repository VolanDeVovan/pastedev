use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use paste_core::Scope;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    audit,
    auth::{api_key::ApiKeyRow, extract::SessionUser},
    error::AppError,
    http::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Serialize)]
pub struct KeyView {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<Scope>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize)]
pub struct KeyMintedView {
    #[serde(flatten)]
    pub key: KeyView,
    /// Plaintext token — present only on the create response, never again.
    pub token: String,
}

fn view_of(row: &ApiKeyRow) -> KeyView {
    KeyView {
        id: row.id,
        name: row.name.clone(),
        prefix: row.prefix.clone(),
        scopes: row.scopes.clone(),
        created_at: row.created_at,
        last_used_at: row.last_used_at,
        revoked_at: row.revoked_at,
    }
}

/// `POST /api/v1/keys` — session auth only (don't let a leaked bearer mint more).
pub async fn create(
    SessionUser(user): SessionUser,
    State(state): State<AppState>,
    Json(req): Json<CreateRequest>,
) -> Result<(StatusCode, Json<KeyMintedView>), AppError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    if name.len() > 80 {
        return Err(AppError::Validation("name too long (max 80)".into()));
    }
    let mut scopes = req.scopes.clone();
    if scopes.is_empty() {
        scopes.push(Scope::Publish);
    }
    scopes.sort_by_key(|s| s.as_str());
    scopes.dedup();

    let minted = crate::auth::api_key::insert(&state.pool, user.id, name, &scopes).await?;
    let actor = user.id;
    let key_id = minted.row.id;
    let prefix = minted.row.prefix.clone();
    let scope_strs: Vec<&'static str> = scopes.iter().map(|s| s.as_str()).collect();
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let _ = audit::write(
            &pool,
            audit::Event {
                event: "api_key.create",
                actor_user_id: Some(actor),
                actor_api_key_id: Some(key_id),
                payload: Some(serde_json::json!({"prefix": prefix, "scopes": scope_strs})),
                ..Default::default()
            },
        )
        .await;
    });

    let body = KeyMintedView {
        key: view_of(&minted.row),
        token: minted.token,
    };
    Ok((StatusCode::CREATED, Json(body)))
}

#[derive(Debug, Serialize)]
pub struct KeyList {
    pub items: Vec<KeyView>,
}

/// `GET /api/v1/keys`
pub async fn list(
    SessionUser(user): SessionUser,
    State(state): State<AppState>,
) -> Result<Json<KeyList>, AppError> {
    let rows = crate::auth::api_key::list_for_user(&state.pool, user.id).await?;
    Ok(Json(KeyList {
        items: rows.iter().map(view_of).collect(),
    }))
}

/// `DELETE /api/v1/keys/:id`
pub async fn revoke(
    SessionUser(user): SessionUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let revoked = crate::auth::api_key::revoke(&state.pool, id, user.id).await?;
    if !revoked {
        return Err(AppError::NotFound);
    }
    let actor = user.id;
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let _ = audit::write(
            &pool,
            audit::Event {
                event: "api_key.revoke",
                actor_user_id: Some(actor),
                actor_api_key_id: Some(id),
                ..Default::default()
            },
        )
        .await;
    });
    Ok(StatusCode::NO_CONTENT)
}
