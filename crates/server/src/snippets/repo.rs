use paste_core::SnippetType;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SnippetRow {
    pub id: Uuid,
    pub slug: String,
    pub owner_id: Uuid,
    pub owner_username: String,
    pub kind: SnippetType,
    pub name: Option<String>,
    pub body: String,
    pub size_bytes: i32,
    pub visibility: String,
    pub views: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug)]
pub struct SnippetDraft<'a> {
    pub owner_id: Uuid,
    pub kind: SnippetType,
    pub name: Option<&'a str>,
    pub body: &'a str,
}

type RowTuple = (
    Uuid,
    String,
    Uuid,
    String, // owner_username (joined)
    String,
    Option<String>,
    String,
    i32,
    String,
    i32,
    OffsetDateTime,
    OffsetDateTime,
);

fn map(t: RowTuple) -> Option<SnippetRow> {
    let kind = SnippetType::from_str_opt(&t.4)?;
    Some(SnippetRow {
        id: t.0,
        slug: t.1,
        owner_id: t.2,
        owner_username: t.3,
        kind,
        name: t.5,
        body: t.6,
        size_bytes: t.7,
        visibility: t.8,
        views: t.9,
        created_at: t.10,
        updated_at: t.11,
    })
}

const SELECT_JOIN: &str = "
    SELECT s.id, s.slug, s.owner_id, u.username,
           s.type, s.name, s.body, s.size_bytes, s.visibility, s.views,
           s.created_at, s.updated_at
    FROM snippets s
    JOIN users u ON u.id = s.owner_id
";

pub async fn insert<'a>(
    pool: &PgPool,
    slug: &str,
    draft: &SnippetDraft<'a>,
) -> Result<SnippetRow, sqlx::Error> {
    let size = i32::try_from(draft.body.len()).unwrap_or(i32::MAX);
    let inserted_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO snippets (slug, owner_id, type, name, body, size_bytes)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(slug)
    .bind(draft.owner_id)
    .bind(draft.kind.as_str())
    .bind(draft.name)
    .bind(draft.body)
    .bind(size)
    .fetch_one(pool)
    .await?;
    by_id(pool, inserted_id.0)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

pub async fn by_id(pool: &PgPool, id: Uuid) -> Result<Option<SnippetRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, RowTuple>(&format!("{SELECT_JOIN} WHERE s.id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(map))
}

pub async fn by_slug(pool: &PgPool, slug: &str) -> Result<Option<SnippetRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, RowTuple>(&format!("{SELECT_JOIN} WHERE s.slug = $1"))
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(map))
}

pub async fn delete(pool: &PgPool, slug: &str, owner_id: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM snippets WHERE slug = $1 AND owner_id = $2")
        .bind(slug)
        .bind(owner_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

#[derive(Debug, Default)]
pub struct SnippetPatch<'a> {
    pub body: Option<&'a str>,
    pub name: Option<Option<&'a str>>, // double Option: outer = "did caller supply", inner = the value (NULL allowed)
}

pub async fn update(
    pool: &PgPool,
    slug: &str,
    owner_id: Uuid,
    patch: SnippetPatch<'_>,
) -> Result<Option<SnippetRow>, sqlx::Error> {
    let body = patch.body;
    let size = body.map(|b| i32::try_from(b.len()).unwrap_or(i32::MAX));
    let (set_name, name_value) = match patch.name {
        Some(v) => (true, v),
        None => (false, None),
    };

    // Build a small dynamic update — body/name might be missing.
    let mut sql = String::from("UPDATE snippets SET ");
    let mut parts: Vec<&str> = Vec::new();
    if body.is_some() {
        parts.push("body = $3, size_bytes = $4");
    }
    if set_name {
        parts.push(if body.is_some() { "name = $5" } else { "name = $3" });
    }
    if parts.is_empty() {
        // Nothing to change; just return the current row.
        return by_slug(pool, slug).await.map(|opt| opt.filter(|r| r.owner_id == owner_id));
    }
    sql.push_str(&parts.join(", "));
    sql.push_str(" WHERE slug = $1 AND owner_id = $2");

    let q = sqlx::query(&sql).bind(slug).bind(owner_id);
    let q = match (body, set_name) {
        (Some(b), true) => q.bind(b).bind(size.unwrap()).bind(name_value),
        (Some(b), false) => q.bind(b).bind(size.unwrap()),
        (None, true) => q.bind(name_value),
        (None, false) => q,
    };
    let res = q.execute(pool).await?;
    if res.rows_affected() == 0 {
        return Ok(None);
    }
    by_slug(pool, slug).await
}

pub async fn incr_views(pool: &PgPool, slug: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE snippets SET views = views + 1 WHERE slug = $1")
        .bind(slug)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct ListFilter<'a> {
    pub owner_id: Uuid,
    pub kind: Option<SnippetType>,
    pub cursor: Option<&'a OffsetDateTime>,
    pub limit: i64,
}

pub async fn list_for_user(
    pool: &PgPool,
    filter: ListFilter<'_>,
) -> Result<Vec<SnippetRow>, sqlx::Error> {
    let mut sql = format!("{SELECT_JOIN} WHERE s.owner_id = $1");
    if filter.kind.is_some() {
        sql.push_str(" AND s.type = $2");
    }
    if filter.cursor.is_some() {
        sql.push_str(if filter.kind.is_some() {
            " AND s.created_at < $3"
        } else {
            " AND s.created_at < $2"
        });
    }
    sql.push_str(" ORDER BY s.created_at DESC");
    sql.push_str(if filter.kind.is_some() && filter.cursor.is_some() {
        " LIMIT $4"
    } else if filter.kind.is_some() || filter.cursor.is_some() {
        " LIMIT $3"
    } else {
        " LIMIT $2"
    });

    let mut q = sqlx::query_as::<_, RowTuple>(&sql).bind(filter.owner_id);
    if let Some(k) = filter.kind {
        q = q.bind(k.as_str());
    }
    if let Some(c) = filter.cursor {
        q = q.bind(c);
    }
    let rows = q.bind(filter.limit).fetch_all(pool).await?;
    Ok(rows.into_iter().filter_map(map).collect())
}
