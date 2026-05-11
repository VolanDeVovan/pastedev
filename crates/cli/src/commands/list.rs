use anyhow::Result;
use pastedev_core::SnippetType;

use crate::client::ApiClient;
use crate::credentials::resolve;
use crate::output::{fmt_ago, fmt_size, print, Format};

pub struct Args<'a> {
    pub format: Format,
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub kind: Option<SnippetType>,
    pub limit: Option<u32>,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let client = ApiClient::new(base_url, token)?;
    let list = client.list_snippets(args.kind, None, args.limit).await?;
    print(args.format, &list, || {
        println!(
            "{:<5} {:<8} {:<28} {:<10} {:<7} {:<5}",
            "type", "slug", "name", "created", "size", "views"
        );
        for item in &list.items {
            let name = item.name.as_deref().unwrap_or("(unnamed)");
            let kind = match item.kind {
                SnippetType::Code => "code",
                SnippetType::Markdown => "md",
                SnippetType::Html => "html",
            };
            println!(
                "{:<5} {:<8} {:<28} {:<10} {:<7} {:<5}",
                kind,
                item.slug,
                truncate(name, 28),
                fmt_ago(item.created_at),
                fmt_size(item.size_bytes),
                item.views,
            );
        }
    });
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}
