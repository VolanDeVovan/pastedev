use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use paste_core::{CreateSnippetRequest, SnippetType, MAX_SNIPPET_BYTES};

use crate::client::ApiClient;
use crate::credentials::resolve;
use crate::output::{print, Format};

pub struct Args<'a> {
    pub format: Format,
    pub token: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub kind: Option<SnippetType>,
    pub name: Option<String>,
    pub file: Option<PathBuf>,
}

pub async fn run(args: Args<'_>) -> Result<()> {
    let (token, base_url, _) = resolve(args.token, args.base_url)?;
    let (body, inferred_kind, inferred_name) = read_body(args.file.as_ref())?;
    if body.is_empty() {
        return Err(anyhow!("body is empty — nothing to publish"));
    }
    if body.len() > MAX_SNIPPET_BYTES {
        return Err(anyhow!(
            "body is {} bytes, max is {}",
            body.len(),
            MAX_SNIPPET_BYTES
        ));
    }
    let kind = args.kind.or(inferred_kind).unwrap_or(SnippetType::Code);
    let name = args.name.or(inferred_name);

    let client = ApiClient::new(base_url, token)?;
    let snippet = client
        .create_snippet(&CreateSnippetRequest {
            kind,
            name: name.clone(),
            body,
        })
        .await
        .context("creating snippet")?;
    print(args.format, &snippet, || {
        println!("→ {}", snippet.url);
        println!(
            "  type={} · {} b · {}",
            snippet.kind.as_str(),
            snippet.size_bytes,
            snippet.raw_url,
        );
    });
    Ok(())
}

fn read_body(file: Option<&PathBuf>) -> Result<(String, Option<SnippetType>, Option<String>)> {
    if let Some(path) = file {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let body = String::from_utf8(bytes).context("file is not valid UTF-8")?;
        let kind = path
            .extension()
            .and_then(|os| os.to_str())
            .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
                "md" | "markdown" => Some(SnippetType::Markdown),
                "html" | "htm" => Some(SnippetType::Html),
                _ => None,
            });
        let name = path
            .file_name()
            .and_then(|os| os.to_str())
            .map(String::from);
        return Ok((body, kind, name));
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading stdin")?;
    Ok((buf, None, None))
}
