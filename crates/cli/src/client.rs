use anyhow::{anyhow, Context, Result};
use paste_core::{
    CreateSnippetRequest, ErrorEnvelope, ListSnippetsResponse, PatchSnippetRequest, Snippet,
    SnippetType, UserPublic,
};
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;

pub struct ApiClient {
    pub base_url: String,
    pub token: String,
    inner: Client,
}

impl ApiClient {
    pub fn new(base_url: String, token: String) -> Result<Self> {
        let inner = Client::builder()
            .user_agent(concat!("paste-cli/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { base_url, token, inner })
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    async fn send_json<B: serde::Serialize, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<R> {
        let mut req = self
            .inner
            .request(method, self.url(path))
            .bearer_auth(&self.token);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.context("sending request")?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            let body_str = String::from_utf8_lossy(&bytes).to_string();
            // Try to parse the error envelope; fall back to plain text.
            if let Ok(env) = serde_json::from_slice::<ErrorEnvelope>(&bytes) {
                return Err(anyhow!(
                    "{} {}: {} — {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    serde_json::to_string(&env.error.code).unwrap_or_default().trim_matches('"').to_string(),
                    env.error.message,
                ));
            }
            return Err(anyhow!("{}: {}", status, body_str));
        }
        if bytes.is_empty() {
            // For DELETE/204-style responses, R should be `()`. Construct manually if so.
            return serde_json::from_slice::<R>(b"null").context("empty body");
        }
        serde_json::from_slice::<R>(&bytes).context("parsing response")
    }

    pub async fn me(&self) -> Result<UserPublic> {
        self.send_json::<(), UserPublic>(Method::GET, "/api/v1/auth/me", None).await
    }

    pub async fn create_snippet(&self, body: &CreateSnippetRequest) -> Result<Snippet> {
        self.send_json(Method::POST, "/api/v1/snippets", Some(body)).await
    }

    pub async fn get_snippet(&self, slug: &str) -> Result<Snippet> {
        let path = format!("/api/v1/snippets/{}", slug);
        self.send_json::<(), _>(Method::GET, &path, None).await
    }

    pub async fn delete_snippet(&self, slug: &str) -> Result<()> {
        let path = format!("/api/v1/snippets/{}", slug);
        // Manual handling — 204 means success.
        let resp = self
            .inner
            .delete(self.url(&path))
            .bearer_auth(&self.token)
            .send()
            .await?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(());
        }
        let s = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("{}: {}", s, body))
    }

    pub async fn patch_snippet(&self, slug: &str, patch: &PatchSnippetRequest) -> Result<Snippet> {
        let path = format!("/api/v1/snippets/{}", slug);
        self.send_json(Method::PATCH, &path, Some(patch)).await
    }

    pub async fn list_snippets(
        &self,
        kind: Option<SnippetType>,
        cursor: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ListSnippetsResponse> {
        let mut qs: Vec<String> = Vec::new();
        if let Some(k) = kind {
            qs.push(format!("type={}", k.as_str()));
        }
        if let Some(c) = cursor {
            qs.push(format!("cursor={}", urlencoding(c)));
        }
        if let Some(l) = limit {
            qs.push(format!("limit={}", l));
        }
        let path = if qs.is_empty() {
            "/api/v1/snippets".to_string()
        } else {
            format!("/api/v1/snippets?{}", qs.join("&"))
        };
        self.send_json::<(), _>(Method::GET, &path, None).await
    }
}

fn urlencoding(s: &str) -> String {
    // Tiny encoder for the cursor (we control the alphabet — base64url). Avoid
    // pulling in a full URL-encoding crate.
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            other => format!("%{:02X}", other as u32),
        })
        .collect()
}
