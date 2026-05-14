use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use pastedev_core::{CreateSnippetRequest, SnippetType, Visibility, MAX_SNIPPET_BYTES};

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
    pub visibility: Option<Visibility>,
    /// Parsed duration spec like `15m`, `1h`, `7d`. `None` = no fixed lifetime.
    pub lifetime_seconds: Option<i32>,
    pub burn_after_read: bool,
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
            visibility: args.visibility,
            lifetime_seconds: args.lifetime_seconds,
            burn_after_read: Some(args.burn_after_read),
        })
        .await
        .context("creating snippet")?;
    print(args.format, &snippet, || {
        println!("{}", snippet.url);
    });
    Ok(())
}

/// Parse a duration spec like `15m`, `2h`, `1d`, `1w`, or a plain integer
/// (interpreted as seconds). Returns the value in seconds. Rejects negatives,
/// zero, and any input whose seconds-product overflows i64 / can't fit in i32
/// — failing fast here surfaces a friendlier error than letting the server's
/// `LIFETIME_SECONDS_MAX` bound or the i64 multiplication panic do it.
pub fn parse_duration(s: &str) -> Result<i32> {
    let s = s.trim();
    if s.is_empty() {
        return Err(anyhow!("empty duration"));
    }
    let (num, unit) = match s.chars().last().unwrap() {
        c if c.is_ascii_digit() => (s, "s"),
        _ => (&s[..s.len() - 1], &s[s.len() - 1..]),
    };
    let n: i64 = num
        .parse()
        .map_err(|_| anyhow!("invalid duration: '{s}'"))?;
    if n <= 0 {
        return Err(anyhow!("duration must be positive"));
    }
    let multiplier: i64 = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        "w" => 7 * 24 * 60 * 60,
        other => return Err(anyhow!("unknown duration unit '{other}'; use s/m/h/d/w")),
    };
    let secs = n
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow!("duration too large"))?;
    i32::try_from(secs).map_err(|_| anyhow!("duration too large"))
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
