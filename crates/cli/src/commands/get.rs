use anyhow::Result;
use std::io::Write;

use crate::client::ApiClient;
use crate::credentials::resolve;
use crate::output::Format;

pub struct Args<'a> {
    pub format: Format,
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub slug: &'a str,
    pub meta: bool,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let client = ApiClient::new(base_url, token)?;
    let snippet = client.get_snippet(args.slug).await?;
    match args.format {
        Format::Json => {
            let json = serde_json::to_string_pretty(&snippet)?;
            println!("{json}");
            return Ok(());
        }
        Format::Human => {}
    }
    if args.meta {
        eprintln!("slug:       {}", snippet.slug);
        eprintln!("type:       {}", snippet.kind.as_str());
        eprintln!("name:       {}", snippet.name.as_deref().unwrap_or("(unnamed)"));
        eprintln!("owner:      {}", snippet.owner.username);
        eprintln!("size:       {} b", snippet.size_bytes);
        eprintln!("views:      {}", snippet.views);
        eprintln!("created_at: {}", snippet.created_at);
        eprintln!("updated_at: {}", snippet.updated_at);
        eprintln!("---");
    }
    // Body to stdout, no decoration — the next thing in the pipe might be diff/jq.
    std::io::stdout().write_all(snippet.body.as_bytes())?;
    if !snippet.body.ends_with('\n') {
        println!();
    }
    Ok(())
}
