//! Auth request/response shapes shared between the server and the WASM client.
//!
//! These types live in core (not server) so the SPA can `use` them directly
//! instead of re-declaring the same JSON shape in TypeScript or Rust.

use serde::{Deserialize, Serialize};

use crate::UserPublic;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserEnvelope {
    pub user: UserPublic,
}
