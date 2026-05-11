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
