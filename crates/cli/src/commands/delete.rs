use anyhow::Result;
use std::io::Write;

use crate::client::ApiClient;
use crate::credentials::resolve;

pub struct Args<'a> {
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub slug: &'a str,
    pub yes: bool,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    if !args.yes {
        eprint!("delete {}? [y/N]: ", args.slug);
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            eprintln!("aborted");
            return Ok(());
        }
    }
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let client = ApiClient::new(base_url, token)?;
    client.delete_snippet(args.slug).await?;
    eprintln!("→ deleted {}", args.slug);
    Ok(())
}
