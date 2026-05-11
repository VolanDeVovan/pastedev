//! Shared types used by both `pastedev-server` and `pastedev-cli`.
//!
//! The contract here is the over-the-wire JSON shape — keeping it in one place
//! prevents the CLI's bindings drifting from the server's request/response types.

pub mod error;
pub mod scope;
pub mod slug;
pub mod snippet;
pub mod user;

pub use error::{ErrorBody, ErrorCode, ErrorEnvelope};
pub use scope::Scope;
pub use slug::{is_valid_slug, SLUG_ALPHABET, SLUG_LEN};
pub use snippet::{
    CreateSnippetRequest, ListSnippetsResponse, PatchSnippetRequest, Snippet, SnippetListItem,
    SnippetType,
};
pub use user::{Role, UserPublic, UserStatus};

pub const MAX_SNIPPET_BYTES: usize = 1_048_576;

/// `pds_live_<8 char prefix>_<32 char secret>`
pub const API_KEY_PREFIX_LEN: usize = 8;
pub const API_KEY_SECRET_LEN: usize = 32;
pub const API_KEY_TOKEN_PREAMBLE: &str = "pds_live_";

/// The session cookie name. Always the same to keep the CLI / docs honest.
pub const SESSION_COOKIE_NAME: &str = "pds_session";
