use anyhow::{anyhow, Result};
use pastedev_core::{SettingsRequest, Visibility};

use crate::client::ApiClient;
use crate::credentials::resolve;
use crate::output::{print, Format};

pub struct Args<'a> {
    pub format: Format,
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub slug: &'a str,
    pub visibility: Option<Visibility>,
    /// Three states mirroring the wire format: `Some(Some(n))` set,
    /// `Some(None)` clear, `None` leave alone. main.rs reconciles the
    /// `--lifetime` / `--no-lifetime` flag pair into this.
    pub lifetime_seconds: Option<Option<i32>>,
    pub burn_after_read: Option<bool>,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    if args.visibility.is_none() && args.lifetime_seconds.is_none() && args.burn_after_read.is_none() {
        return Err(anyhow!(
            "specify at least one of --visibility, --lifetime/--no-lifetime, --burn-after-read/--no-burn-after-read",
        ));
    }
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let client = ApiClient::new(base_url, token)?;
    let snippet = client
        .update_settings(
            args.slug,
            &SettingsRequest {
                visibility: args.visibility,
                lifetime_seconds: args.lifetime_seconds,
                burn_after_read: args.burn_after_read,
            },
        )
        .await?;
    print(args.format, &snippet, || {
        println!("{}", snippet.url);
    });
    Ok(())
}
