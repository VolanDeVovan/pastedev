use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine;
use paste_core::{
    path_prefix_for, CreateSnippetRequest, ListSnippetsResponse, PatchSnippetRequest, Snippet,
    SnippetListItem, SnippetType,
};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{
    audit,
    auth::extract::{ApprovedUser, MaybeAuthUser},
    error::AppError,
    http::AppState,
    snippets::{
        repo::{self, ListFilter, SnippetDraft, SnippetPatch, SnippetRow},
        slug,
    },
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

fn validate_slug(slug: &str) -> Result<(), AppError> {
    if !paste_core::is_valid_slug(slug) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn to_dto(row: &SnippetRow, public_base_url: &str) -> Snippet {
    let prefix = path_prefix_for(row.kind);
    Snippet {
        id: row.id,
        slug: row.slug.clone(),
        kind: row.kind,
        name: row.name.clone(),
        body: row.body.clone(),
        size_bytes: row.size_bytes,
        visibility: row.visibility.clone(),
        views: row.views,
        owner: paste_core::snippet::SnippetOwner {
            username: row.owner_username.clone(),
        },
        url: format!("{}{}{}", public_base_url, prefix, row.slug),
        raw_url: format!("{}{}{}/raw", public_base_url, prefix, row.slug),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn to_list_item(row: &SnippetRow) -> SnippetListItem {
    SnippetListItem {
        slug: row.slug.clone(),
        kind: row.kind,
        name: row.name.clone(),
        size_bytes: row.size_bytes,
        views: row.views,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// `POST /api/v1/snippets`
pub async fn create(
    user: ApprovedUser,
    State(state): State<AppState>,
    Json(req): Json<CreateSnippetRequest>,
) -> Result<(StatusCode, Json<Snippet>), AppError> {
    if req.body.is_empty() {
        return Err(AppError::Validation("body is required".into()));
    }
    if req.body.len() > state.config.snippet_max_bytes {
        return Err(AppError::SnippetTooLarge {
            size: req.body.len(),
            limit: state.config.snippet_max_bytes,
        });
    }
    let name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.len() > 255 {
                Err(AppError::Validation("name too long".into()))
            } else {
                Ok(s.to_string())
            }
        })
        .transpose()?;
    let draft = SnippetDraft {
        owner_id: user.0.id,
        kind: req.kind,
        name: name.as_deref(),
        body: &req.body,
    };
    let row = slug::create_with_retry(&state.pool, &draft).await?;
    let pool = state.pool.clone();
    let event_user = user.0.id;
    let event_slug = row.slug.clone();
    let event_kind = row.kind.as_str();
    let event_size = row.size_bytes;
    let target_id = row.id;
    tokio::spawn(async move {
        let _ = audit::write(
            &pool,
            audit::Event {
                event: "snippet.create",
                actor_user_id: Some(event_user),
                target_snippet_id: Some(target_id),
                payload: Some(serde_json::json!({
                    "slug": event_slug,
                    "type": event_kind,
                    "size_bytes": event_size
                })),
                ..Default::default()
            },
        )
        .await;
    });
    Ok((StatusCode::CREATED, Json(to_dto(&row, &state.config.public_base_url))))
}

/// `GET /api/v1/snippets/:slug`
pub async fn get(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Snippet>, AppError> {
    validate_slug(&slug)?;
    let row = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    // Increment view counter best-effort.
    {
        let pool = state.pool.clone();
        let s = slug.clone();
        tokio::spawn(async move {
            let _ = repo::incr_views(&pool, &s).await;
        });
    }
    Ok(Json(to_dto(&row, &state.config.public_base_url)))
}

/// `PATCH /api/v1/snippets/:slug`
pub async fn patch(
    user: ApprovedUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<PatchSnippetRequest>,
) -> Result<Json<Snippet>, AppError> {
    validate_slug(&slug)?;
    let existing = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    let body_owned = req.body;
    if let Some(b) = body_owned.as_deref() {
        if b.is_empty() {
            return Err(AppError::Validation("body cannot be empty".into()));
        }
        if b.len() > state.config.snippet_max_bytes {
            return Err(AppError::SnippetTooLarge {
                size: b.len(),
                limit: state.config.snippet_max_bytes,
            });
        }
    }
    let name_outer = req.name.map(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let patch = SnippetPatch {
        body: body_owned.as_deref(),
        name: name_outer.as_ref().map(|opt| opt.as_deref()),
    };
    let old_size = existing.size_bytes;
    let updated = repo::update(&state.pool, &slug, user.0.id, patch)
        .await?
        .ok_or(AppError::NotFound)?;
    let pool = state.pool.clone();
    let actor = user.0.id;
    let target = updated.id;
    let new_size = updated.size_bytes;
    tokio::spawn(async move {
        let _ = audit::write(
            &pool,
            audit::Event {
                event: "snippet.update",
                actor_user_id: Some(actor),
                target_snippet_id: Some(target),
                payload: Some(serde_json::json!({
                    "old_size_bytes": old_size,
                    "new_size_bytes": new_size,
                })),
                ..Default::default()
            },
        )
        .await;
    });
    Ok(Json(to_dto(&updated, &state.config.public_base_url)))
}

/// `DELETE /api/v1/snippets/:slug`
pub async fn delete(
    user: ApprovedUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, AppError> {
    validate_slug(&slug)?;
    let existing = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.owner_id != user.0.id {
        return Err(AppError::Forbidden);
    }
    let removed = repo::delete(&state.pool, &slug, user.0.id).await?;
    if !removed {
        return Err(AppError::NotFound);
    }
    let pool = state.pool.clone();
    let actor = user.0.id;
    let target = existing.id;
    tokio::spawn(async move {
        let _ = audit::write(
            &pool,
            audit::Event {
                event: "snippet.delete",
                actor_user_id: Some(actor),
                target_snippet_id: Some(target),
                ..Default::default()
            },
        )
        .await;
    });
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/snippets` — caller's own snippets.
pub async fn list(
    user: ApprovedUser,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListSnippetsResponse>, AppError> {
    let kind = q
        .kind
        .as_deref()
        .map(|s| SnippetType::from_str_opt(s).ok_or(AppError::Validation("invalid type".into())))
        .transpose()?;
    let cursor = q
        .cursor
        .as_deref()
        .map(decode_cursor)
        .transpose()?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let filter = ListFilter {
        owner_id: user.0.id,
        kind,
        cursor: cursor.as_ref(),
        limit: limit + 1, // peek for next-page presence
    };
    let mut rows = repo::list_for_user(&state.pool, filter).await?;
    let mut next_cursor = None;
    if rows.len() as i64 > limit {
        let extra = rows.pop().expect("len > limit ⇒ pop");
        next_cursor = Some(encode_cursor(&extra.created_at));
    }
    Ok(Json(ListSnippetsResponse {
        items: rows.iter().map(to_list_item).collect(),
        next_cursor,
    }))
}

/// Raw `/c/:slug/raw` and `/m/:slug/raw` — `text/plain`.
pub async fn raw_text(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, AppError> {
    validate_slug(&slug)?;
    let row = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    // Don't enforce the type — the prefix is informational. The view route's
    // CSP / iframe story differs only on `/h/:slug/raw` (phase 3).
    let mut response = Response::new(Body::from(row.body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=0"),
    );
    Ok(response)
}

/// `MaybeAuthUser` consumer for the "show a copy-link affordance to the owner"
/// path — currently unused server-side, but reserved.
#[allow(dead_code)]
pub fn _maybe_marker(_: MaybeAuthUser, _: HeaderMap) {}

fn encode_cursor(at: &OffsetDateTime) -> String {
    let nanos = at.unix_timestamp_nanos();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nanos.to_be_bytes())
}

fn decode_cursor(s: &str) -> Result<OffsetDateTime, AppError> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s.trim())
        .map_err(|_| AppError::Validation("invalid cursor".into()))?;
    if bytes.len() != 16 {
        return Err(AppError::Validation("invalid cursor".into()));
    }
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes);
    let nanos = i128::from_be_bytes(buf);
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .map_err(|_| AppError::Validation("invalid cursor".into()))
}
