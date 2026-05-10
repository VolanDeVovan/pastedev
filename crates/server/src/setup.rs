//! `/api/v1/setup/*` endpoints — environment status + first-admin creation.
//!
//! The "needs setup?" predicate is `SELECT count(*) FROM users = 0`. We cache
//! the boolean for 60s in-process to avoid hammering the DB on every shell
//! request; the cache is invalidated when the first admin is created.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use ipnetwork::IpNetwork;
use paste_core::{Role, UserPublic, UserStatus};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::sync::Mutex;

use crate::{
    audit,
    auth::{self, password, session},
    error::AppError,
    http::AppState,
    users::repo::{self, NewUser},
};

/// In-process cache of "the users table is empty". Atomic for the fast read;
/// the mutex serialises refreshes so we don't run dozens of `SELECT count(*)`
/// queries in parallel under cold-cache load.
#[derive(Debug)]
pub struct SetupGate {
    cached_needs: AtomicBool,
    last_refreshed: Mutex<Option<Instant>>,
}

impl SetupGate {
    pub fn new() -> Self {
        Self {
            cached_needs: AtomicBool::new(true),
            last_refreshed: Mutex::new(None),
        }
    }

    pub async fn needs_setup(&self, pool: &PgPool) -> bool {
        // Fast path: if we've decided "doesn't need setup" once, that's terminal.
        if !self.cached_needs.load(Ordering::Relaxed) {
            return false;
        }
        let mut last = self.last_refreshed.lock().await;
        if last
            .as_ref()
            .is_some_and(|t| t.elapsed() < Duration::from_secs(60))
        {
            return self.cached_needs.load(Ordering::Relaxed);
        }
        let count = repo::count(pool).await.unwrap_or(0);
        let needs = count == 0;
        self.cached_needs.store(needs, Ordering::Relaxed);
        *last = Some(Instant::now());
        needs
    }

    pub fn invalidate(&self) {
        self.cached_needs.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub status: &'static str, // "ok" | "warn" | "err" | "pend"
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct SetupStatus {
    pub needs_setup: bool,
    pub version: &'static str,
    pub checks: Vec<Check>,
}

/// `GET /api/v1/setup/status`
pub async fn status(State(state): State<AppState>) -> Json<SetupStatus> {
    let mut checks = Vec::with_capacity(4);

    let db_check = match sqlx::query_scalar!(r#"SELECT version() AS "v!""#)
        .fetch_one(&state.pool)
        .await
    {
        Ok(v) => Check {
            id: "database",
            status: "ok",
            detail: short_pg_version(&v),
        },
        Err(e) => Check {
            id: "database",
            status: "err",
            detail: format!("not reachable: {e}"),
        },
    };
    checks.push(db_check);

    checks.push(Check {
        id: "secret",
        status: if state.config.paste_secret.len() >= 16 {
            "ok"
        } else {
            "err"
        },
        detail: format!("loaded · {} bytes", state.config.paste_secret.len()),
    });

    checks.push(Check {
        id: "public_url",
        status: if !state.config.public_base_url.is_empty() {
            "ok"
        } else {
            "warn"
        },
        detail: state.config.public_base_url.clone(),
    });

    let migrations_check = match sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM _sqlx_migrations"#,
    )
    .fetch_one(&state.pool)
    .await
    {
        Ok(n) => Check {
            id: "migrations",
            status: "ok",
            detail: format!("{n} applied"),
        },
        Err(e) => Check {
            id: "migrations",
            status: "err",
            detail: format!("ledger query failed: {e}"),
        },
    };
    checks.push(migrations_check);

    Json(SetupStatus {
        needs_setup: state.setup_gate.needs_setup(&state.pool).await,
        version: env!("CARGO_PKG_VERSION"),
        checks,
    })
}

#[derive(Debug, Deserialize)]
pub struct AdminRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub user: UserPublic,
}

/// `POST /api/v1/setup/admin`
pub async fn create_first_admin(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<AdminRequest>,
) -> Result<Response, AppError> {
    let username = normalize_username(&req.username)?;
    let email = normalize_email(req.email.as_deref())?;
    validate_password(&req.password)?;

    // Race-safe gate: take an advisory lock on a deterministic key, then count.
    // The lock is held for the duration of the transaction; concurrent submissions
    // serialise on it. Postgres rejects `SELECT count(*) ... FOR UPDATE`, hence the
    // advisory lock instead.
    let mut tx = state.pool.begin().await?;
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext('paste:setup_admin'))")
        .execute(&mut *tx)
        .await?;
    let count_row = sqlx::query!(r#"SELECT count(*) AS "n!" FROM users"#)
        .fetch_one(&mut *tx)
        .await?;
    if count_row.n > 0 {
        return Err(AppError::SetupComplete);
    }

    let phc = password::hash(&req.password, state.config.argon2_m_kib, state.config.argon2_t_cost)
        .map_err(|e| AppError::Validation(format!("password hashing: {e}")))?;
    let ip_addr = auth::client_ip(&headers, Some(peer.ip()));
    let ip_net = ip_addr.map(IpNetwork::from);

    let email_param = email.as_deref();
    let user_row = sqlx::query!(
        "INSERT INTO users (username, email, password_hash, role, status, registration_ip)
         VALUES ($1, $2, $3, 'admin', 'approved', $4)
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at, updated_at",
        username,
        email_param,
        phc,
        ip_net,
    )
    .fetch_one(&mut *tx)
    .await?;
    let user = repo::UserRow {
        id: user_row.id,
        username: user_row.username,
        email: user_row.email,
        password_hash: user_row.password_hash,
        role: Role::from_str_opt(&user_row.role).expect("role from insert"),
        status: UserStatus::from_str_opt(&user_row.status).expect("status from insert"),
        reason: user_row.reason,
        registration_ip: user_row.registration_ip,
        created_at: user_row.created_at,
        updated_at: user_row.updated_at,
    };

    tx.commit().await?;
    state.setup_gate.invalidate();

    let ua = auth::client_user_agent(&headers);
    let cookie_value =
        session::issue(&state.pool, &state.config, user.id, ip_net, ua.as_deref()).await?;
    let set_cookie =
        session::build_cookie(&state.config, &cookie_value, state.config.session_ttl_seconds);

    if let Err(e) = audit::write(
        &state.pool,
        audit::Event {
            event: "user.setup_admin",
            actor_user_id: Some(user.id),
            target_user_id: Some(user.id),
            ip: ip_net,
            user_agent: ua.as_deref(),
            ..Default::default()
        },
    )
    .await
    {
        audit::log_err("user.setup_admin", e);
    }

    let body = Json(AdminResponse {
        user: UserPublic {
            id: user.id,
            username: user.username,
            role: user.role,
            status: user.status,
            created_at: user.created_at,
        },
    });
    let mut response = (StatusCode::CREATED, body).into_response();
    if let Ok(v) = set_cookie.parse() {
        response.headers_mut().insert(header::SET_COOKIE, v);
    }
    Ok(response)
}

fn normalize_username(raw: &str) -> Result<String, AppError> {
    let s = raw.trim().to_ascii_lowercase();
    let valid = s.len() >= 3
        && s.len() <= 40
        && s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.' || c == '-'
        });
    if !valid {
        return Err(AppError::Validation(
            "username must match [a-z0-9_.-]{3,40}".into(),
        ));
    }
    Ok(s)
}

fn normalize_email(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if s.len() > 255 || !s.contains('@') || !s.contains('.') {
        return Err(AppError::Validation("email looks invalid".into()));
    }
    Ok(Some(s.to_string()))
}

fn validate_password(s: &str) -> Result<(), AppError> {
    if s.len() < 12 {
        return Err(AppError::Validation(
            "password must be at least 12 characters".into(),
        ));
    }
    Ok(())
}

fn short_pg_version(version: &str) -> String {
    // e.g. "PostgreSQL 16.2 on x86_64-pc-linux-musl, …"
    let trimmed = version
        .splitn(3, ' ')
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    trimmed
}

// Convenience constructor used by main.
pub fn shared_gate() -> Arc<SetupGate> {
    Arc::new(SetupGate::new())
}
