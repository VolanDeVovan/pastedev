use anyhow::{anyhow, Context, Result};
use time::OffsetDateTime;

use crate::client::ApiClient;
use crate::credentials::{save, Credentials};

pub struct Args<'a> {
    pub token: &'a str,
    pub base_url_flag: Option<&'a str>,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    let base_url = args
        .base_url_flag
        .map(String::from)
        .or_else(|| std::env::var("PASTEDEV_BASE_URL").ok())
        .ok_or_else(|| {
            anyhow!("no base URL: pass --base-url or set PASTEDEV_BASE_URL before `pastedev-cli auth`")
        })?;
    let token = args.token.trim();
    if !token.starts_with(pastedev_core::API_KEY_TOKEN_PREAMBLE) {
        return Err(anyhow!("token doesn't look like a pds_live_ key"));
    }

    let client = ApiClient::new(base_url.clone(), token.to_string())?;
    eprintln!("→ verifying via {}", base_url);
    let me = client
        .me()
        .await
        .context("token verification failed")?;
    let creds = Credentials {
        token: token.to_string(),
        username: Some(me.username.clone()),
        base_url: base_url.clone(),
        scopes: Vec::new(),
        stored_at: OffsetDateTime::now_utc().to_string(),
    };
    let p = save(&creds)?;
    eprintln!("→ stored in {}", p.display());
    eprintln!("→ verified as {} · status={} @ {}", me.username, me.status.as_str(), base_url);
    Ok(())
}
