//! Setup-wizard request/response shapes shared between server and SPA.

use serde::{Deserialize, Serialize};

use crate::UserPublic;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupCheck {
    pub id: String,
    pub status: String, // "ok" | "warn" | "err" | "pend"
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupStatus {
    pub needs_setup: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<SetupCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupAdminRequest {
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupAdminResponse {
    pub user: UserPublic,
}
