use anyhow::Result;

use crate::client::ApiClient;
use crate::credentials::resolve;
use crate::output::{print, Format};

pub struct Args<'a> {
    pub format: Format,
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let client = ApiClient::new(base_url.clone(), token)?;
    let me = client.me().await?;
    print(args.format, &me, || {
        println!("{} · {} · @ {}", me.username, me.status.as_str(), base_url);
    });
    Ok(())
}
