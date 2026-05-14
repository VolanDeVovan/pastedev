use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnippetType {
    Code,
    Markdown,
    Html,
}

impl SnippetType {
    pub fn as_str(self) -> &'static str {
        match self {
            SnippetType::Code => "code",
            SnippetType::Markdown => "markdown",
            SnippetType::Html => "html",
        }
    }
}

impl FromStr for SnippetType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "code" => Ok(Self::Code),
            "markdown" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            _ => Err(()),
        }
    }
}

/// Snippet visibility. `public` is the default — anyone can fetch the slug.
/// `private` requires the caller to be an authenticated, approved user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Public,
    Private,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        }
    }
}

impl FromStr for Visibility {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            _ => Err(()),
        }
    }
}

/// Bounds for the user-supplied `lifetime_seconds` input on
/// create/`/settings`. The server adds it to `now()` to compute the absolute
/// `expires_at`; the bounds keep that arithmetic sane.
pub const LIFETIME_SECONDS_MIN: i32 = 60;
pub const LIFETIME_SECONDS_MAX: i32 = 365 * 24 * 60 * 60;

/// How long a `burn_after_read` snippet stays readable after the first non-owner
/// view. Hard-coded — exposed as a constant so the frontend timer + server
/// computation agree.
pub const BURN_AFTER_READ_WINDOW_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetOwner {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Uuid,
    pub slug: String,
    #[serde(rename = "type")]
    pub kind: SnippetType,
    pub name: Option<String>,
    pub body: String,
    pub size_bytes: i32,
    pub views: i32,
    pub owner: SnippetOwner,
    pub url: String,
    pub raw_url: String,
    #[serde(default)]
    pub visibility: Visibility,
    /// When true, the first non-owner view stamps `first_viewed_at` and
    /// tightens `expires_at` down to `now() + 15min`. Stays a separate
    /// flag (independent of `expires_at`) so the frontend can label the
    /// snippet "burns after read" before the timer is armed.
    #[serde(default)]
    pub burn_after_read: bool,
    /// When the first non-owner GET landed. `None` until then (only
    /// meaningful when `burn_after_read = true`).
    #[serde(with = "time::serde::rfc3339::option", default, skip_serializing_if = "Option::is_none")]
    pub first_viewed_at: Option<OffsetDateTime>,
    /// Absolute timestamp at which non-owner reads stop resolving. `None`
    /// means the snippet has no expiry (and burn-after-read, if enabled,
    /// hasn't been triggered yet). The frontend ticks a countdown against
    /// this value directly — no client-side combination logic.
    #[serde(with = "time::serde::rfc3339::option", default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetListItem {
    pub slug: String,
    #[serde(rename = "type")]
    pub kind: SnippetType,
    pub name: Option<String>,
    pub size_bytes: i32,
    pub views: i32,
    #[serde(default)]
    pub visibility: Visibility,
    #[serde(default)]
    pub burn_after_read: bool,
    #[serde(with = "time::serde::rfc3339::option", default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnippetRequest {
    #[serde(rename = "type")]
    pub kind: SnippetType,
    pub name: Option<String>,
    pub body: String,
    #[serde(default)]
    pub visibility: Option<Visibility>,
    /// Optional TTL applied at creation — server stores
    /// `expires_at = now() + lifetime_seconds`. `None` (or omitted) = no
    /// fixed expiry.
    #[serde(default)]
    pub lifetime_seconds: Option<i32>,
    /// Burn 15 minutes after the first non-owner view. Independent of
    /// `lifetime_seconds` — both can be set; whichever fires first wins.
    #[serde(default)]
    pub burn_after_read: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatchSnippetRequest {
    pub body: Option<String>,
    pub name: Option<String>,
}

/// Body of `PATCH /api/v1/snippets/:slug/settings`. Any subset of fields may
/// be supplied; missing fields stay as-is.
///
/// `lifetime_seconds` uses `Option<Option<i32>>` so the three states are
/// distinct on the wire and in code:
///   * field omitted (`None`)       — leave the existing expiry alone
///   * `null` (`Some(None)`)        — clear the expiry (no expiration)
///   * integer (`Some(Some(n))`)    — set `expires_at = now() + n`
///
/// The custom deserializer is required because serde's default would collapse
/// missing-vs-null into the same `None`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_lifetime"
    )]
    pub lifetime_seconds: Option<Option<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burn_after_read: Option<bool>,
}

fn deserialize_optional_lifetime<'de, D>(d: D) -> Result<Option<Option<i32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(d)?;
    if v.is_null() {
        return Ok(Some(None));
    }
    let n = v
        .as_i64()
        .ok_or_else(|| serde::de::Error::custom("lifetime_seconds must be an integer or null"))?;
    let n = i32::try_from(n)
        .map_err(|_| serde::de::Error::custom("lifetime_seconds out of range"))?;
    Ok(Some(Some(n)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSnippetsResponse {
    pub items: Vec<SnippetListItem>,
    pub next_cursor: Option<String>,
}
