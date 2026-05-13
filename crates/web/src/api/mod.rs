//! WASM HTTP client. One thin reqwest wrapper, one error type that mirrors
//! the server's error envelope, one helper per endpoint.

pub mod admin;
pub mod auth;
pub mod client;
pub mod error;
pub mod keys;
pub mod snippets;

pub use client::{call, call_unit};
pub use error::HttpError;
