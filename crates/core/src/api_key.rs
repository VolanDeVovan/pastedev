//! API-key request/response shapes shared between server and SPA.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Scope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyView {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<Scope>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyMintedView {
    #[serde(flatten)]
    pub key: KeyView,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyList {
    pub items: Vec<KeyView>,
}
