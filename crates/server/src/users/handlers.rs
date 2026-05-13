use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use pastedev_core::{LoginRequest, RegisterRequest, Role, UserEnvelope, UserPublic, UserStatus};

use crate::{
    audit,
    auth::{self, extract::AuthUser, password, session},
    error::AppError,
    http::{client_ip::ClientIp, AppState},
    users::{
        repo::{self, NewUser},
        validate::{normalize_email, normalize_username, validate_password},
    },
};

/// Maps an internal `UserRow` to the public JSON view (drops the hash, IP, etc).
pub fn to_public(row: &repo::UserRow) -> UserPublic {
    UserPublic {
        id: row.id,
        username: row.username.clone(),
        role: row.role,
        status: row.status,
        created_at: row.created_at,
    }
}

/// `POST /api/v1/auth/register`
pub async fn register(
    State(state): State<AppState>,
    client_ip: ClientIp,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<Response, AppError> {
    if !state.config.registration_open {
        return Err(AppError::Forbidden(Some("registration closed")));
    }
    let username = normalize_username(&req.username)?;
    validate_password(&req.password)?;
    let reason = req
        .reason
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.len() < 10 || s.len() > 500 {
                Err(AppError::Validation(
                    "reason must be 10–500 characters".into(),
                ))
            } else {
                Ok(s.to_string())
            }
        })
        .transpose()?;
    let email = normalize_email(req.email.as_deref())?;

    if repo::by_username(&state.pool, &username).await?.is_some() {
        return Err(AppError::Conflict("username already taken"));
    }

    let phc = password::hash(&req.password, state.config.argon2_m_kib, state.config.argon2_t_cost)
        .map_err(|e| AppError::Validation(format!("password hashing: {e}")))?;

    let ip_net = client_ip.as_ipnetwork();
    let user = repo::insert(
        &state.pool,
        NewUser {
            username: &username,
            email: email.as_deref(),
            password_hash: &phc,
            role: Role::User,
            status: UserStatus::Pending,
            reason: reason.as_deref(),
            registration_ip: ip_net,
        },
    )
    .await?;

    let ua = auth::client_user_agent(&headers);
    let cookie_value = session::issue(&state.pool, &state.config, user.id, ip_net, ua.as_deref()).await?;
    let set_cookie = session::build_cookie(&state.config, &cookie_value, state.config.session_ttl_seconds);

    audit::write(
        &state.pool,
        audit::Event {
            event: "user.register",
            actor_user_id: Some(user.id),
            target_user_id: Some(user.id),
            ip: ip_net,
            user_agent: ua.as_deref(),
            payload: Some(serde_json::json!({
                "reason_len": reason.as_ref().map(|s| s.len()).unwrap_or(0)
            })),
            ..Default::default()
        },
    )
    .await;

    let body = Json(UserEnvelope { user: to_public(&user) });
    let mut response = (StatusCode::CREATED, body).into_response();
    if let Ok(v) = set_cookie.parse() {
        response.headers_mut().insert(header::SET_COOKIE, v);
    }
    Ok(response)
}

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    client_ip: ClientIp,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let username = normalize_username(&req.username)?;

    let user = repo::by_username(&state.pool, &username).await?;
    let verified = match &user {
        Some(u) => password::verify(&req.password, &u.password_hash),
        None => {
            // Constant-time-ish behaviour: always do an Argon2 call. We accept the
            // memory burst for the rare miss.
            password::dummy_verify(state.config.argon2_m_kib, state.config.argon2_t_cost);
            false
        }
    };
    if !verified {
        return Err(AppError::Unauthorized);
    }
    let user = user.expect("verified true ⇒ user is Some");

    if user.status == UserStatus::Suspended {
        return Err(AppError::Forbidden(Some("account suspended")));
    }

    let ip_net = client_ip.as_ipnetwork();
    let ua = auth::client_user_agent(&headers);
    let cookie_value = session::issue(&state.pool, &state.config, user.id, ip_net, ua.as_deref()).await?;
    let set_cookie = session::build_cookie(&state.config, &cookie_value, state.config.session_ttl_seconds);

    audit::write(
        &state.pool,
        audit::Event {
            event: "session.login",
            actor_user_id: Some(user.id),
            ip: ip_net,
            user_agent: ua.as_deref(),
            ..Default::default()
        },
    )
    .await;

    let body = Json(UserEnvelope { user: to_public(&user) });
    let mut response = (StatusCode::OK, body).into_response();
    if let Ok(v) = set_cookie.parse() {
        response.headers_mut().insert(header::SET_COOKIE, v);
    }
    Ok(response)
}

/// `POST /api/v1/auth/logout`
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // Best-effort cookie parsing; if anything's off, we still clear it client-side.
    let raw = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut session_value: Option<String> = None;
    for piece in raw.split(';') {
        if let Some((n, v)) = piece.trim().split_once('=') {
            if n == pastedev_core::SESSION_COOKIE_NAME {
                session_value = Some(v.to_string());
            }
        }
    }
    if let Some(v) = session_value {
        if let Some(bytes) = session::decode_cookie(&v) {
            let _ = session::revoke(&state.pool, &bytes).await;
        }
    }
    let mut response = (StatusCode::NO_CONTENT, ()).into_response();
    if let Ok(v) = session::build_clear_cookie(&state.config).parse() {
        response.headers_mut().insert(header::SET_COOKIE, v);
    }
    Ok(response)
}

/// `GET /api/v1/auth/me`
pub async fn me(
    AuthUser(u): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserPublic>, AppError> {
    let row = repo::by_id(&state.pool, u.id).await?.ok_or(AppError::Unauthorized)?;
    Ok(Json(to_public(&row)))
}

