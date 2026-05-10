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
    pub updated_at: OffsetDateTime,
}

type UserTuple = (
    Uuid,
    String,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    Option<IpNetwork>,
    OffsetDateTime,
    OffsetDateTime,
);

const FIELDS: &str = "id, username, email, password_hash, role, status, reason, registration_ip, created_at, updated_at";

fn map(tuple: UserTuple) -> Option<UserRow> {
    let (id, username, email, password_hash, role_s, status_s, reason, registration_ip, created_at, updated_at) =
        tuple;
    Some(UserRow {
        id,
        username,
        email,
        password_hash,
        role: Role::from_str_opt(&role_s)?,
        status: UserStatus::from_str_opt(&status_s)?,
        reason,
        registration_ip,
        created_at,
        updated_at,
    })
}

pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let (n,) = sqlx::query_as::<_, (i64,)>("SELECT count(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn by_username(pool: &PgPool, username: &str) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "SELECT {FIELDS} FROM users WHERE username = $1"
    ))
    .bind(username)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

pub async fn by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "SELECT {FIELDS} FROM users WHERE id = $1"
    ))
    .bind(id)
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

pub async fn insert(pool: &PgPool, new: NewUser<'_>) -> Result<UserRow, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "INSERT INTO users (username, email, password_hash, role, status, reason, registration_ip)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING {FIELDS}"
    ))
    .bind(new.username)
    .bind(new.email)
    .bind(new.password_hash)
    .bind(new.role.as_str())
    .bind(new.status.as_str())
    .bind(new.reason)
    .bind(new.registration_ip)
    .fetch_one(pool)
    .await?;
    Ok(map(row).expect("valid role/status from insert"))
}

pub async fn set_status(
    pool: &PgPool,
    id: Uuid,
    status: UserStatus,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "UPDATE users SET status = $2 WHERE id = $1 RETURNING {FIELDS}"
    ))
    .bind(id)
    .bind(status.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

pub async fn set_role(
    pool: &PgPool,
    id: Uuid,
    role: Role,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "UPDATE users SET role = $2 WHERE id = $1 RETURNING {FIELDS}"
    ))
    .bind(id)
    .bind(role.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

pub async fn set_password_hash(
    pool: &PgPool,
    id: Uuid,
    hash: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, UserTuple>(&format!(
        "UPDATE users SET password_hash = $2 WHERE id = $1 RETURNING {FIELDS}"
    ))
    .bind(id)
    .bind(hash)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(map))
}

pub async fn count_active_admins(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let (n,) = sqlx::query_as::<_, (i64,)>(
        "SELECT count(*) FROM users WHERE role = 'admin' AND status = 'approved'",
    )
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Lists users, optionally filtered by status.
pub async fn list(
    pool: &PgPool,
    status: Option<UserStatus>,
    limit: i64,
) -> Result<Vec<UserRow>, sqlx::Error> {
    let rows = if let Some(s) = status {
        sqlx::query_as::<_, UserTuple>(&format!(
            "SELECT {FIELDS} FROM users WHERE status = $1 ORDER BY created_at DESC LIMIT $2"
        ))
        .bind(s.as_str())
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, UserTuple>(&format!(
            "SELECT {FIELDS} FROM users ORDER BY created_at DESC LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows.into_iter().filter_map(map).collect())
}
