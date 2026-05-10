use ipnetwork::IpNetwork;
use paste_core::{Role, UserStatus};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: Role,
    pub status: UserStatus,
    pub reason: Option<String>,
    pub registration_ip: Option<IpNetwork>,
    pub created_at: OffsetDateTime,
}

/// Raw-row destination for `query_as!`. `role` and `status` are stored as
/// CHECK-constrained `varchar`, so sqlx sees them as `String`; we convert
/// after fetch via `map`.
struct UserRowRaw {
    id: Uuid,
    username: String,
    email: Option<String>,
    password_hash: String,
    role: String,
    status: String,
    reason: Option<String>,
    registration_ip: Option<IpNetwork>,
    created_at: OffsetDateTime,
}

fn map(r: UserRowRaw) -> Option<UserRow> {
    Some(UserRow {
        id: r.id,
        username: r.username,
        email: r.email,
        password_hash: r.password_hash,
        role: r.role.parse().ok()?,
        status: r.status.parse().ok()?,
        reason: r.reason,
        registration_ip: r.registration_ip,
        created_at: r.created_at,
    })
}

/// Total number of rows in the users table. Used by the setup gate.
pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let r = sqlx::query!("SELECT count(*) AS \"n!\" FROM users")
        .fetch_one(pool)
        .await?;
    Ok(r.n)
}

/// Fetch a user by (lowercased) username.
pub async fn by_username(pool: &PgPool, username: &str) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "SELECT id, username, email, password_hash, role, status, reason, registration_ip, created_at
         FROM users WHERE username = $1",
        username,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

/// Fetch a user by primary key.
pub async fn by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "SELECT id, username, email, password_hash, role, status, reason, registration_ip, created_at
         FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: Option<&'a str>,
    pub password_hash: &'a str,
    pub role: Role,
    pub status: UserStatus,
    pub reason: Option<&'a str>,
    pub registration_ip: Option<IpNetwork>,
}

/// Insert a new user and return the materialised row.
pub async fn insert(pool: &PgPool, new: NewUser<'_>) -> Result<UserRow, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "INSERT INTO users (username, email, password_hash, role, status, reason, registration_ip)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at",
        new.username,
        new.email,
        new.password_hash,
        new.role.as_str(),
        new.status.as_str(),
        new.reason,
        new.registration_ip,
    )
    .fetch_one(pool)
    .await?;
    Ok(map(row).expect("valid role/status from insert"))
}

/// Same as [`insert`] but takes a live transaction so callers can serialise the
/// INSERT with other statements (e.g. setup's advisory-locked admin check).
pub async fn insert_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new: NewUser<'_>,
) -> Result<UserRow, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "INSERT INTO users (username, email, password_hash, role, status, reason, registration_ip)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at",
        new.username,
        new.email,
        new.password_hash,
        new.role.as_str(),
        new.status.as_str(),
        new.reason,
        new.registration_ip,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(map(row).expect("valid role/status from insert"))
}

/// Update `status`. `None` means the user id wasn't found.
pub async fn set_status(
    pool: &PgPool,
    id: Uuid,
    status: UserStatus,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "UPDATE users SET status = $2 WHERE id = $1
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at",
        id,
        status.as_str(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

/// Update `role`.
pub async fn set_role(
    pool: &PgPool,
    id: Uuid,
    role: Role,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "UPDATE users SET role = $2 WHERE id = $1
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at",
        id,
        role.as_str(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

/// Replace the stored Argon2 hash. Callers should also revoke active sessions.
pub async fn set_password_hash(
    pool: &PgPool,
    id: Uuid,
    hash: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as!(
        UserRowRaw,
        "UPDATE users SET password_hash = $2 WHERE id = $1
         RETURNING id, username, email, password_hash, role, status, reason, registration_ip, created_at",
        id,
        hash,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

/// Count `role=admin AND status=approved`. Used to guard against demoting or
/// disabling the last active admin.
pub async fn count_active_admins(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let r = sqlx::query!(
        "SELECT count(*) AS \"n!\" FROM users WHERE role = 'admin' AND status = 'approved'",
    )
    .fetch_one(pool)
    .await?;
    Ok(r.n)
}

/// Lists users, optionally filtered by status.
pub async fn list(
    pool: &PgPool,
    status: Option<UserStatus>,
    limit: i64,
) -> Result<Vec<UserRow>, sqlx::Error> {
    // Split into two static queries — sqlx macros can't validate WHERE clauses
    // that are conditionally appended at runtime.
    let rows = if let Some(s) = status {
        sqlx::query_as!(
            UserRowRaw,
            "SELECT id, username, email, password_hash, role, status, reason, registration_ip, created_at
             FROM users WHERE status = $1 ORDER BY created_at DESC LIMIT $2",
            s.as_str(),
            limit,
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            UserRowRaw,
            "SELECT id, username, email, password_hash, role, status, reason, registration_ip, created_at
             FROM users ORDER BY created_at DESC LIMIT $1",
            limit,
        )
        .fetch_all(pool)
        .await?
    };
    Ok(rows.into_iter().filter_map(map).collect())
}
