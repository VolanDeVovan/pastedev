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

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "code" => Some(Self::Code),
            "markdown" => Some(Self::Markdown),
            "html" => Some(Self::Html),
            _ => None,
        }
    }
}

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
    pub visibility: String,
    pub views: i32,
    pub owner: SnippetOwner,
    pub url: String,
    pub raw_url: String,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatchSnippetRequest {
    pub body: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSnippetsResponse {
    pub items: Vec<SnippetListItem>,
    pub next_cursor: Option<String>,
}
