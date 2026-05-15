use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use base64::Engine;
use pastedev_core::{
    CreateSnippetRequest, ListSnippetsResponse, PatchSnippetRequest, SettingsRequest, Snippet,
    SnippetListItem, SnippetType, Visibility, LIFETIME_SECONDS_MAX, LIFETIME_SECONDS_MIN,
};
use serde::Deserialize;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

/// Audit payloads need a stable serialization for `expires_at` — RFC3339
/// matches what the API itself emits over the wire, so logs and HTTP
/// responses round-trip cleanly. Falls back to Debug-format (which still
/// includes the timestamp) on the impossible case where formatting fails.
fn fmt_ts(t: Option<OffsetDateTime>) -> Option<String> {
    t.map(|t| t.format(&Rfc3339).unwrap_or_else(|_| t.to_string()))
}

use crate::{
    audit,
    auth::{
        extract::{scope_id, try_extract_user, RequiresScope},
        hmac::hmac_sha256_bytes,
    },
    error::AppError,
    http::{client_ip::ClientIp, AppState},
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
    if !pastedev_core::is_valid_slug(slug) {
        return Err(AppError::NotFound);
    }
    Ok(())
}

fn to_dto(row: &SnippetRow, public_base_url: &str) -> Snippet {
    let prefix = match row.kind {
        SnippetType::Code => "/c/",
        SnippetType::Markdown => "/m/",
        SnippetType::Html => "/h/",
    };
    Snippet {
        id: row.id,
        slug: row.slug.clone(),
        kind: row.kind,
        name: row.name.clone(),
        body: row.body.clone(),
        size_bytes: row.size_bytes,
        views: row.views,
        owner: pastedev_core::snippet::SnippetOwner {
            username: row.owner_username.clone(),
        },
        url: format!("{}{}{}", public_base_url, prefix, row.slug),
        raw_url: format!("{}{}{}/raw", public_base_url, prefix, row.slug),
        visibility: row.visibility,
        burn_after_read: row.burn_after_read,
        first_viewed_at: row.first_viewed_at,
        expires_at: row.expires_at,
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
        visibility: row.visibility,
        burn_after_read: row.burn_after_read,
        expires_at: row.expires_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

/// Validate the user-supplied `lifetime_seconds` input and convert it into an
/// absolute `expires_at` timestamp by adding it to `now()`. `Ok(None)` means
/// "no expiry"; `Ok(Some(_))` carries the absolute target. Surfaces 400 for
/// out-of-range inputs so a malformed CLI flag doesn't reach Postgres.
fn lifetime_to_expires_at(s: Option<i32>) -> Result<Option<OffsetDateTime>, AppError> {
    match s {
        None => Ok(None),
        Some(v) if (LIFETIME_SECONDS_MIN..=LIFETIME_SECONDS_MAX).contains(&v) => {
            Ok(Some(OffsetDateTime::now_utc() + Duration::seconds(v as i64)))
        }
        Some(_) => Err(AppError::Validation(format!(
            "lifetime_seconds must be between {} and {}",
            LIFETIME_SECONDS_MIN, LIFETIME_SECONDS_MAX
        ))),
    }
}

/// `POST /api/v1/snippets`
pub async fn create(
    user: RequiresScope<{ scope_id::PUBLISH }>,
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
    let visibility = req.visibility.unwrap_or_default();
    let expires_at = lifetime_to_expires_at(req.lifetime_seconds)?;
    let burn_after_read = req.burn_after_read.unwrap_or(false);
    let draft = SnippetDraft {
        owner_id: user.0.id,
        kind: req.kind,
        name: name.as_deref(),
        body: &req.body,
        visibility,
        expires_at,
        burn_after_read,
    };
    let row = slug::create_with_retry(&state.pool, &draft).await?;
    audit::spawn_write(
        state.pool.clone(),
        audit::OwnedEvent {
            event: "snippet.create",
            actor_user_id: Some(user.0.id),
            target_snippet_id: Some(row.id),
            payload: Some(serde_json::json!({
                "slug": row.slug,
                "type": row.kind.as_str(),
                "size_bytes": row.size_bytes,
                "visibility": row.visibility.as_str(),
                "expires_at": fmt_ts(row.expires_at),
                "burn_after_read": row.burn_after_read,
            })),
            ..Default::default()
        },
    );
    Ok((StatusCode::CREATED, Json(to_dto(&row, &state.config.public_base_url))))
}

/// `GET /api/v1/snippets/:slug`
pub async fn get(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Json<Snippet>, AppError> {
    validate_slug(&slug)?;
    let caller = try_extract_user(&state, &headers).await;
    let caller_id = caller.as_ref().map(|u| u.id);
    let mut row = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    enforce_access(&row, caller_id)?;

    // First non-owner view of a burn-after-read snippet: stamp
    // `first_viewed_at` AND tighten `expires_at` down to `now() + 15min` so
    // the burn window can't outlive any pre-existing lifetime. The owner
    // browsing their own snippet never triggers the timer.
    if row.burn_after_read && row.first_viewed_at.is_none() && caller_id != Some(row.owner_id) {
        match repo::mark_first_view(&state.pool, row.id).await {
            Ok(Some(fresh)) => row = fresh,
            Ok(None) => {}
            Err(e) => tracing::warn!(error = ?e, slug = %row.slug, "mark_first_view failed"),
        }
    }

    // Record unique viewer best-effort. The hash is keyed by PASTEDEV_SECRET so
    // a DB-only leak of `snippet_views.viewer_hash` doesn't reveal which IPs
    // visited which snippets.
    {
        let pool = state.pool.clone();
        let snippet_id = row.id;
        let hash = viewer_hash(&state.config.pastedev_secret, ip, &headers, snippet_id);
        tokio::spawn(async move {
            let _ = repo::record_view(&pool, snippet_id, &hash).await;
        });
    }
    Ok(Json(to_dto(&row, &state.config.public_base_url)))
}

/// Centralised access check for read paths. Enforces visibility (private
/// snippets require an authenticated caller) and expiry (non-owner callers can
/// no longer read past the effective expiry). The owner can always read their
/// own snippet, even after it has expired for everyone else.
///
/// All "you can't see this" outcomes resolve to `NotFound` instead of
/// `Unauthorized`/`Forbidden` so an external scanner can't tell a private
/// slug apart from an absent / expired one. The slug itself is the sharing
/// capability — leaking "this slug exists but is private" leaks structure.
pub fn enforce_access(row: &SnippetRow, caller_id: Option<Uuid>) -> Result<(), AppError> {
    let is_owner = caller_id == Some(row.owner_id);

    if row.visibility == Visibility::Private && !is_owner && caller_id.is_none() {
        return Err(AppError::NotFound);
    }
    // Private + non-owner authed caller is allowed: any approved user can read
    // a private snippet whose slug they've been given. "private" gates against
    // drive-by anonymous access, not against authed lateral sharing.

    if !is_owner {
        if let Some(exp) = row.expires_at {
            if exp <= OffsetDateTime::now_utc() {
                return Err(AppError::NotFound);
            }
        }
    }
    Ok(())
}

/// Build a stable per-(viewer, snippet) HMAC. Material layout:
///   ip_string || 0x00 || user_agent || 0x00 || snippet_id_bytes
/// — null separators avoid any ambiguity between fields (an IP can't contain
/// a NUL, neither can a header value Axum has parsed).
fn viewer_hash(
    secret: &str,
    ip: Option<std::net::IpAddr>,
    headers: &HeaderMap,
    snippet_id: uuid::Uuid,
) -> [u8; 32] {
    let ip_str = ip.map(|i| i.to_string()).unwrap_or_default();
    let ua = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut material = Vec::with_capacity(ip_str.len() + 1 + ua.len() + 1 + 16);
    material.extend_from_slice(ip_str.as_bytes());
    material.push(0);
    material.extend_from_slice(ua.as_bytes());
    material.push(0);
    material.extend_from_slice(snippet_id.as_bytes());
    hmac_sha256_bytes(secret, &material)
}

/// `PATCH /api/v1/snippets/:slug`
pub async fn patch(
    user: RequiresScope<{ scope_id::PUBLISH }>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<PatchSnippetRequest>,
) -> Result<Json<Snippet>, AppError> {
    validate_slug(&slug)?;
    let existing = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.owner_id != user.0.id {
        return Err(AppError::Forbidden(None));
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
    audit::spawn_write(
        state.pool.clone(),
        audit::OwnedEvent {
            event: "snippet.update",
            actor_user_id: Some(user.0.id),
            target_snippet_id: Some(updated.id),
            payload: Some(serde_json::json!({
                "old_size_bytes": old_size,
                "new_size_bytes": updated.size_bytes,
            })),
            ..Default::default()
        },
    );
    Ok(Json(to_dto(&updated, &state.config.public_base_url)))
}

/// `PATCH /api/v1/snippets/:slug/settings` — owner-only sharing-policy mutator.
///
/// Disabling `burn_after_read` also clears `first_viewed_at` so the
/// previously-started 15-min fuse is fully cancelled (otherwise re-enabling
/// it later would re-arm an already-spent timer). It does *not* roll back
/// any `expires_at` that the burn already tightened — that original
/// pre-burn lifetime isn't recorded anywhere. If the owner wants the
/// snippet to live longer again, they have to pass `lifetime_seconds` in
/// the same (or a follow-up) request and re-stamp expiry explicitly.
pub async fn update_settings(
    user: RequiresScope<{ scope_id::PUBLISH }>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<SettingsRequest>,
) -> Result<Json<Snippet>, AppError> {
    validate_slug(&slug)?;

    let existing = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.owner_id != user.0.id {
        return Err(AppError::Forbidden(None));
    }

    if req.visibility.is_none() && req.lifetime_seconds.is_none() && req.burn_after_read.is_none() {
        return Err(AppError::Validation("at least one field is required".into()));
    }

    // Convert the user's `lifetime_seconds` choice into an absolute
    // `expires_at = now() + lifetime` so picking "15 min" on an
    // already-expired snippet means "alive for 15 more minutes from now",
    // not "15 minutes after creation" (which would still be in the past).
    let expires_at = match req.lifetime_seconds {
        Some(opt) => Some(lifetime_to_expires_at(opt)?),
        None => None,
    };

    let patch = repo::SettingsPatch {
        visibility: req.visibility,
        expires_at,
        burn_after_read: req.burn_after_read,
    };
    let updated = repo::update_settings(&state.pool, &slug, user.0.id, patch)
        .await?
        .ok_or(AppError::NotFound)?;
    audit::spawn_write(
        state.pool.clone(),
        audit::OwnedEvent {
            event: "snippet.settings",
            actor_user_id: Some(user.0.id),
            target_snippet_id: Some(updated.id),
            payload: Some(serde_json::json!({
                "old": {
                    "visibility": existing.visibility.as_str(),
                    "expires_at": fmt_ts(existing.expires_at),
                    "burn_after_read": existing.burn_after_read,
                },
                "new": {
                    "visibility": updated.visibility.as_str(),
                    "expires_at": fmt_ts(updated.expires_at),
                    "burn_after_read": updated.burn_after_read,
                },
            })),
            ..Default::default()
        },
    );
    Ok(Json(to_dto(&updated, &state.config.public_base_url)))
}

/// `DELETE /api/v1/snippets/:slug`
pub async fn delete(
    user: RequiresScope<{ scope_id::DELETE }>,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, AppError> {
    validate_slug(&slug)?;
    let existing = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.owner_id != user.0.id {
        return Err(AppError::Forbidden(None));
    }
    let removed = repo::delete(&state.pool, &slug, user.0.id).await?;
    if !removed {
        return Err(AppError::NotFound);
    }
    audit::spawn_write(
        state.pool.clone(),
        audit::OwnedEvent {
            event: "snippet.delete",
            actor_user_id: Some(user.0.id),
            target_snippet_id: Some(existing.id),
            payload: Some(serde_json::json!({ "kind": "soft" })),
            ..Default::default()
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/snippets` — caller's own snippets.
pub async fn list(
    user: RequiresScope<{ scope_id::READ }>,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListSnippetsResponse>, AppError> {
    let kind = q
        .kind
        .as_deref()
        .map(|s| s.parse::<SnippetType>().map_err(|_| AppError::Validation("invalid type".into())))
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

#[derive(Debug, Deserialize)]
pub struct PasteQuery {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub visibility: Option<String>,
    pub lifetime_seconds: Option<i32>,
    pub burn_after_read: Option<bool>,
}

/// `POST /paste` — curl-friendly alias for snippet creation.
///
/// Body is the raw text (`Content-Type` ignored), defaults to `type=code`,
/// optional `?type=markdown|html`. Response is plain text with the snippet URL
/// and a trailing newline so it composes cleanly in shell pipelines.
pub async fn paste_raw(
    user: RequiresScope<{ scope_id::PUBLISH }>,
    State(state): State<AppState>,
    Query(q): Query<PasteQuery>,
    body: String,
) -> Result<Response, AppError> {
    let kind = q
        .kind
        .as_deref()
        .map(|s| s.parse::<SnippetType>().map_err(|_| AppError::Validation("invalid type".into())))
        .transpose()?
        .unwrap_or(SnippetType::Code);
    if body.is_empty() {
        return Err(AppError::Validation("body is required".into()));
    }
    if body.len() > state.config.snippet_max_bytes {
        return Err(AppError::SnippetTooLarge {
            size: body.len(),
            limit: state.config.snippet_max_bytes,
        });
    }
    let visibility = q
        .visibility
        .as_deref()
        .map(|s| s.parse::<Visibility>().map_err(|_| AppError::Validation("invalid visibility".into())))
        .transpose()?
        .unwrap_or_default();
    let expires_at = lifetime_to_expires_at(q.lifetime_seconds)?;
    let burn_after_read = q.burn_after_read.unwrap_or(false);
    let draft = SnippetDraft {
        owner_id: user.0.id,
        kind,
        name: None,
        body: &body,
        visibility,
        expires_at,
        burn_after_read,
    };
    let row = slug::create_with_retry(&state.pool, &draft).await?;
    audit::spawn_write(
        state.pool.clone(),
        audit::OwnedEvent {
            event: "snippet.create",
            actor_user_id: Some(user.0.id),
            target_snippet_id: Some(row.id),
            payload: Some(serde_json::json!({
                "slug": row.slug,
                "type": row.kind.as_str(),
                "size_bytes": row.size_bytes,
                "visibility": row.visibility.as_str(),
                "expires_at": fmt_ts(row.expires_at),
                "burn_after_read": row.burn_after_read,
                "via": "paste",
            })),
            ..Default::default()
        },
    );
    let prefix = match row.kind {
        SnippetType::Code => "/c/",
        SnippetType::Markdown => "/m/",
        SnippetType::Html => "/h/",
    };
    let url = format!("{}{}{}\n", state.config.public_base_url, prefix, row.slug);
    let mut response = Response::new(Body::from(url));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    Ok(response)
}

/// Raw `/c/:slug/raw` and `/m/:slug/raw` — `text/plain`.
pub async fn raw_text(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Response, AppError> {
    validate_slug(&slug)?;
    let caller = try_extract_user(&state, &headers).await;
    let caller_id = caller.as_ref().map(|u| u.id);
    let mut row = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    enforce_access(&row, caller_id)?;

    // Same first-view stamping as the JSON GET: raw fetches by non-owners also
    // start the burn timer so `curl /c/<slug>/raw` doesn't bypass it.
    if row.burn_after_read && row.first_viewed_at.is_none() && caller_id != Some(row.owner_id) {
        if let Ok(Some(fresh)) = repo::mark_first_view(&state.pool, row.id).await {
            row = fresh;
        }
    }
    // The prefix in the URL is informational; we don't enforce it. The HTML
    // sandbox route is a separate handler below — anything not html lands here.
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

/// Exact CSP value the `/h/:slug/raw` route emits. Kept as a constant so the
/// regression test can assert byte-for-byte equality.
pub const HTML_SANDBOX_CSP: &str = "sandbox allow-scripts allow-popups";

/// Posts the document's content dimensions to `parent` so the SPA's `<iframe>`
/// can grow to fit its content in both axes. Harmless when `/h/:slug/raw` is
/// opened in a top-level tab — `parent === window` and the message is
/// delivered to self.
///
/// Wire format: `{ type: 'pastedev:size', height: number, width: number }`.
///
/// Height uses `max(documentElement.scrollHeight, body.scrollHeight)`:
/// `documentElement` (the `<html>` element) sizes to at least the iframe's
/// viewport, so when the body content is shorter the iframe keeps a small
/// strip of empty space below — visually preferred over a frame that hugs
/// the last line. The SPA's hysteresis (SIZE_HYSTERESIS) absorbs the
/// resulting 1-cycle bounce so it does not turn into a growth loop.
///
/// Width stays on `body.scrollWidth`: `documentElement.scrollWidth` is at
/// least the iframe's outer width, which would suppress the horizontal
/// scroll wrapper for wide content. `body` defaults to `width: auto` and
/// reports the content's intrinsic extent, overflow-driven only.
const HTML_SIZE_REPORTER: &str = "<script>(function(){function p(){try{var de=document.documentElement,b=document.body;if(!de||!b)return;var h=Math.max(de.scrollHeight,b.scrollHeight);parent.postMessage({type:'pastedev:size',height:h,width:b.scrollWidth},'*')}catch(e){}}if(document.readyState==='complete')p();else window.addEventListener('load',p);if(typeof ResizeObserver==='function')new ResizeObserver(p).observe(document.documentElement);else setInterval(p,500)})();</script>";

fn inject_size_reporter(body: String) -> String {
    // Splice before </body> when present — leaves the user's <head> intact and
    // doesn't break documents that depend on body-end script order. Falls back
    // to append for fragments that omit the boilerplate.
    if let Some(idx) = body.to_ascii_lowercase().rfind("</body>") {
        let mut out = String::with_capacity(body.len() + HTML_SIZE_REPORTER.len());
        out.push_str(&body[..idx]);
        out.push_str(HTML_SIZE_REPORTER);
        out.push_str(&body[idx..]);
        out
    } else {
        let mut out = String::with_capacity(body.len() + HTML_SIZE_REPORTER.len());
        out.push_str(&body);
        out.push_str(HTML_SIZE_REPORTER);
        out
    }
}

/// Raw `/h/:slug/raw` — `text/html` with the sandbox CSP header.
pub async fn raw_html(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Result<Response, AppError> {
    validate_slug(&slug)?;
    let caller = try_extract_user(&state, &headers).await;
    let caller_id = caller.as_ref().map(|u| u.id);
    let mut row = repo::by_slug(&state.pool, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    enforce_access(&row, caller_id)?;

    if row.burn_after_read && row.first_viewed_at.is_none() && caller_id != Some(row.owner_id) {
        if let Ok(Some(fresh)) = repo::mark_first_view(&state.pool, row.id).await {
            row = fresh;
        }
    }
    // Only render as HTML if the snippet is actually html. Wrong type returns
    // 404 so we don't accidentally promote a code snippet into HTML execution.
    if row.kind != SnippetType::Html {
        return Err(AppError::NotFound);
    }
    let body = inject_size_reporter(row.body);
    let mut response = Response::new(Body::from(body));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(HTML_SANDBOX_CSP),
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=0"),
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("SAMEORIGIN"),
    );
    Ok(response)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_sandbox_csp_is_exact() {
        // The two flags allowed (scripts + popups) and nothing else. A regression
        // here — extra flag, missing flag, or reordered tokens — breaks the
        // isolation story. The "trap" is allow-same-origin appearing here.
        assert_eq!(HTML_SANDBOX_CSP, "sandbox allow-scripts allow-popups");
        assert!(!HTML_SANDBOX_CSP.contains("allow-same-origin"));
        assert!(!HTML_SANDBOX_CSP.contains("allow-forms"));
        assert!(!HTML_SANDBOX_CSP.contains("allow-top-navigation"));
    }

    #[test]
    fn size_reporter_spliced_before_body_close() {
        let out = inject_size_reporter(
            "<html><body><p>hello</p></body></html>".to_string(),
        );
        // Reporter must land inside <body>, before </body> — otherwise scripts
        // outside <body> can hit parsing quirks in some browsers.
        let r = out.find("pastedev:size").expect("reporter present");
        let c = out.find("</body>").expect("close tag present");
        assert!(r < c, "reporter must be spliced before </body>");
        assert!(out.ends_with("</body></html>"));
    }

    #[test]
    fn size_reporter_appended_for_fragments() {
        // Body-less fragments (e.g. an MR-style report without <html>/<body>)
        // still need the reporter — append at the very end.
        let out = inject_size_reporter("<div>fragment</div>".to_string());
        assert!(out.starts_with("<div>fragment</div>"));
        assert!(out.contains("pastedev:size"));
    }

    #[test]
    fn size_reporter_uses_last_body_close() {
        // Defend against user html containing the literal string "</body>"
        // earlier in the document (e.g. inside a <pre> code block) — splice
        // before the FINAL closing tag, not the first match.
        let body = "<html><body><pre>&lt;/body&gt;</pre>real</body></html>".to_string();
        let out = inject_size_reporter(body);
        let r = out.find("pastedev:size").unwrap();
        let last_close = out.rfind("</body>").unwrap();
        assert_eq!(out[r..].find("</body>").map(|i| r + i), Some(last_close));
    }

    #[test]
    fn size_reporter_reports_both_dimensions() {
        // The reporter script must measure both axes. Cheap regression guard
        // against accidentally reverting to height-only.
        assert!(HTML_SIZE_REPORTER.contains("scrollHeight"));
        assert!(HTML_SIZE_REPORTER.contains("scrollWidth"));
        assert!(HTML_SIZE_REPORTER.contains("type:'pastedev:size'"));
    }

    #[test]
    fn size_reporter_height_uses_max_of_documentelement_and_body() {
        // Height intentionally maxes with `documentElement.scrollHeight` so
        // short content keeps a visible strip below it — see the
        // `HTML_SIZE_REPORTER` doc comment. Width must NOT touch
        // documentElement, or wide content stops triggering the wrapper's
        // horizontal scroll.
        assert!(HTML_SIZE_REPORTER.contains("de.scrollHeight"));
        assert!(HTML_SIZE_REPORTER.contains("b.scrollHeight"));
        assert!(HTML_SIZE_REPORTER.contains("Math.max"));
        assert!(HTML_SIZE_REPORTER.contains("b.scrollWidth"));
        assert!(!HTML_SIZE_REPORTER.contains("de.scrollWidth"));
    }
}
