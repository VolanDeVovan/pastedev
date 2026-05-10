use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub token: String,
    pub username: Option<String>,
    pub base_url: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub stored_at: String,
}

pub fn path() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("PASTE_CREDENTIALS") {
        return Ok(PathBuf::from(custom));
    }
    let dir = dirs::config_dir().context("could not resolve config dir")?;
    Ok(dir.join("paste").join("credentials"))
}

pub fn load() -> Result<Option<Credentials>> {
    let p = path()?;
    if !p.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
    let creds: Credentials = serde_json::from_str(&raw).context("parsing credentials file")?;
    Ok(Some(creds))
}

pub fn save(creds: &Credentials) -> Result<PathBuf> {
    let p = path()?;
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(creds)?;
    // Write atomically with restrictive permissions (0600 on unix).
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&p)?;
        f.write_all(json.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&p, json)?;
    }
    Ok(p)
}

/// Resolve token + base URL using the documented precedence (flag → env → file).
pub fn resolve(token_flag: Option<&str>, base_url_flag: Option<&str>) -> Result<(String, String, Option<String>)> {
    let stored = load()?;
    let token = token_flag
        .map(String::from)
        .or_else(|| std::env::var("PASTE_API_KEY").ok())
        .or_else(|| stored.as_ref().map(|c| c.token.clone()))
        .context(
            "no API key configured. Run `paste-cli auth --base-url <URL> <TOKEN>` first, \
             or set PASTE_API_KEY.",
        )?;
    let base_url = base_url_flag
        .map(String::from)
        .or_else(|| std::env::var("PASTE_BASE_URL").ok())
        .or_else(|| stored.as_ref().map(|c| c.base_url.clone()))
        .context(
            "no base URL configured. Pass --base-url, set PASTE_BASE_URL, \
             or run `paste-cli auth --base-url <URL> <TOKEN>` first.",
        )?;
    Ok((token, base_url, stored.and_then(|c| c.username)))
}
