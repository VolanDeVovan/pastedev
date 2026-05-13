//! Snippet CRUD + list.

use pastedev_core::{
    CreateSnippetRequest, ListSnippetsResponse, PatchSnippetRequest, Snippet, SnippetType,
};
use reqwest::Method;

use crate::api::{call, call_unit, HttpError};

pub async fn create(input: &CreateSnippetRequest) -> Result<Snippet, HttpError> {
    call(Method::POST, "/api/v1/snippets", Some(input)).await
}

pub async fn get(slug: &str) -> Result<Snippet, HttpError> {
    call(
        Method::GET,
        &format!("/api/v1/snippets/{}", urlencoding(slug)),
        None::<&()>,
    )
    .await
}

pub async fn patch(slug: &str, body: &PatchSnippetRequest) -> Result<Snippet, HttpError> {
    call(
        Method::PATCH,
        &format!("/api/v1/snippets/{}", urlencoding(slug)),
        Some(body),
    )
    .await
}

pub async fn delete(slug: &str) -> Result<(), HttpError> {
    call_unit(
        Method::DELETE,
        &format!("/api/v1/snippets/{}", urlencoding(slug)),
        None::<&()>,
    )
    .await
}

#[derive(Default)]
pub struct ListOpts {
    pub kind: Option<SnippetType>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list(opts: ListOpts) -> Result<ListSnippetsResponse, HttpError> {
    let mut qs: Vec<String> = Vec::new();
    if let Some(k) = opts.kind {
        qs.push(format!("type={}", k.as_str()));
    }
    if let Some(c) = opts.cursor {
        qs.push(format!("cursor={}", urlencoding(&c)));
    }
    if let Some(l) = opts.limit {
        qs.push(format!("limit={l}"));
    }
    let tail = if qs.is_empty() {
        String::new()
    } else {
        format!("?{}", qs.join("&"))
    };
    call(
        Method::GET,
        &format!("/api/v1/snippets{tail}"),
        None::<&()>,
    )
    .await
}

fn urlencoding(s: &str) -> String {
    // Minimal percent-encoder for path/query segments. Slugs are
    // [a-zA-Z0-9_-] in practice, but be defensive.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
