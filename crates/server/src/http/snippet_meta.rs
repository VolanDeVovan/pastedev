//! Everything that goes into the SPA shell's `<head>` for a specific snippet:
//! the human-facing `<title>`, OpenGraph tags for Telegram/Slack link
//! previews, and the description-extraction logic that turns a snippet body
//! into something readable.
//!
//! [`build`] is called from `serve_snippet_shell`; [`SnippetMeta::to_head_html`]
//! is called from [`super::shell::render`].

use pastedev_core::{SnippetType, Visibility};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use time::OffsetDateTime;

use crate::{http::AppState, snippets::repo};

/// Description previews are capped to this many characters. Telegram and
/// most unfurlers truncate around 160–300, so 200 is a comfortable target
/// that won't get chopped mid-word by their renderer.
const DESCRIPTION_MAX_CHARS: usize = 200;

/// Per-snippet metadata for the SPA shell's `<head>`.
#[derive(Debug, Clone)]
pub struct SnippetMeta {
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub site_name: String,
}

impl SnippetMeta {
    /// Render the `<title>` + meta tags as a single HTML string for splicing
    /// into `<head>`. All user-supplied fields are HTML-escaped.
    pub fn to_head_html(&self) -> String {
        let mut out = String::with_capacity(512);
        write_tag(&mut out, "title", &self.title, /* with_inner = */ true);
        write_meta(&mut out, "property", "og:title", &self.title);
        if let Some(desc) = self.description.as_deref().filter(|s| !s.is_empty()) {
            write_meta(&mut out, "name", "description", desc);
            write_meta(&mut out, "property", "og:description", desc);
        }
        write_meta(&mut out, "property", "og:type", "article");
        write_meta(&mut out, "property", "og:url", &self.url);
        write_meta(&mut out, "property", "og:site_name", &self.site_name);
        write_meta(&mut out, "name", "twitter:card", "summary");
        out
    }
}

/// Build a [`SnippetMeta`] for a given slug, or `None` if the slug is invalid
/// or no snippet exists. DB lookup errors degrade to `None` (logged) so the
/// shell still serves — the SPA will render its own 404 in that case.
pub async fn build(state: &AppState, slug: &str) -> Option<SnippetMeta> {
    if !pastedev_core::is_valid_slug(slug) {
        return None;
    }
    let row = match repo::by_slug(&state.pool, slug).await {
        Ok(Some(r)) => r,
        Ok(None) => return None,
        Err(e) => {
            tracing::warn!(error = ?e, slug, "snippet shell: db lookup failed");
            return None;
        }
    };

    // Don't leak private snippet metadata to unfurlers / anonymous link
    // previews. Don't render meta for expired snippets either — the SPA will
    // serve its own 404 from the shell.
    if row.visibility == Visibility::Private {
        return None;
    }
    if let Some(exp) = row.expires_at {
        if exp <= OffsetDateTime::now_utc() {
            return None;
        }
    }

    let app = &state.config.app_name;
    let kind_label = kind_label(row.kind);
    let title = match trimmed_nonempty(row.name.as_deref()) {
        Some(n) => format!("{n} · {kind_label} · {app}"),
        None => format!("Untitled {kind_label} · {app}"),
    };
    let description = match row.kind {
        // Code bodies make ugly previews (long lines, punctuation soup), so
        // we deliberately skip the description for `/c/` snippets.
        SnippetType::Code => None,
        SnippetType::Markdown => Some(description_from_markdown(&row.body)).filter(|s| !s.is_empty()),
        SnippetType::Html => Some(description_from_html(&row.body)).filter(|s| !s.is_empty()),
    };
    let url = format!("{}{}{}", state.config.public_base_url, url_prefix(row.kind), slug);

    Some(SnippetMeta {
        title,
        description,
        url,
        site_name: app.clone(),
    })
}

fn kind_label(k: SnippetType) -> &'static str {
    match k {
        SnippetType::Code => "Code",
        SnippetType::Markdown => "Markdown",
        SnippetType::Html => "HTML",
    }
}

fn url_prefix(k: SnippetType) -> &'static str {
    match k {
        SnippetType::Code => "/c/",
        SnippetType::Markdown => "/m/",
        SnippetType::Html => "/h/",
    }
}

fn trimmed_nonempty(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

// ---- description extraction -----------------------------------------------

/// Walk a markdown document and accumulate plain text, skipping code blocks.
/// pulldown-cmark already handles fences, link rewriting, and inline markers
/// for us — we just stitch the text events back together with whitespace.
fn description_from_markdown(body: &str) -> String {
    let mut buf = String::with_capacity(body.len().min(1024));
    let mut in_code = false;
    for event in Parser::new(body) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => in_code = true,
            Event::End(TagEnd::CodeBlock) => in_code = false,
            Event::Text(t) | Event::Code(t) if !in_code => append_with_space(&mut buf, &t),
            Event::SoftBreak | Event::HardBreak if !in_code => append_with_space(&mut buf, " "),
            // Treat block boundaries (paragraph end, list item, …) as spaces
            // so adjacent items don't smash together.
            Event::End(TagEnd::Paragraph)
            | Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::Item) => append_with_space(&mut buf, " "),
            _ => {}
        }
        if buf.chars().count() >= DESCRIPTION_MAX_CHARS {
            break;
        }
    }
    truncate_chars(strip_space_before_punct(buf.trim()).trim(), DESCRIPTION_MAX_CHARS)
}

/// Strip HTML tags and decode common entities. Cheap stand-in for a real
/// parser — fine for a 200-char preview where structure doesn't matter.
fn description_from_html(body: &str) -> String {
    let mut buf = String::with_capacity(body.len());
    let mut in_tag = false;
    for ch in body.chars() {
        match ch {
            '<' => {
                in_tag = true;
                // Tag boundaries are word breaks; without this `</h1><p>`
                // collapses adjacent words ("welcomeThis").
                buf.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => buf.push(ch),
            _ => {}
        }
    }
    let decoded = decode_basic_entities(&buf);
    truncate_chars(strip_space_before_punct(collapse_ws(&decoded).trim()).trim(), DESCRIPTION_MAX_CHARS)
}

fn decode_basic_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Append text with a single space separator, but only when the buffer
/// already ends in a non-space character and the incoming text isn't itself
/// pure whitespace. Lets us push text fragments from disparate markdown
/// events without ending up with `"a  b"` or leading spaces.
fn append_with_space(buf: &mut String, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    if !buf.is_empty() && !buf.ends_with(' ') {
        buf.push(' ');
    }
    buf.push_str(trimmed);
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_ws = true;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_ws {
                out.push(' ');
                last_ws = true;
            }
        } else {
            out.push(ch);
            last_ws = false;
        }
    }
    out
}

/// Drop a space immediately before sentence punctuation. The HTML stripper
/// injects whitespace between adjacent tags, which produces ugly `bold .`
/// instead of `bold.` when a closing tag butts up against punctuation.
fn strip_space_before_punct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ' '
            && matches!(
                chars.peek(),
                Some('.' | ',' | ':' | ';' | '!' | '?')
            )
        {
            continue;
        }
        out.push(ch);
    }
    out
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

// ---- HTML emission helpers ------------------------------------------------

fn write_tag(out: &mut String, tag: &str, content: &str, with_inner: bool) {
    out.push('<');
    out.push_str(tag);
    out.push('>');
    if with_inner {
        push_escaped(out, content);
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }
}

fn write_meta(out: &mut String, attr: &str, name: &str, content: &str) {
    out.push_str("<meta ");
    out.push_str(attr);
    out.push_str("=\"");
    out.push_str(name);
    out.push_str("\" content=\"");
    push_escaped(out, content);
    out.push_str("\">");
}

fn push_escaped(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esc(s: &str) -> String {
        let mut out = String::new();
        push_escaped(&mut out, s);
        out
    }

    #[test]
    fn html_escape_covers_dangerous_chars() {
        assert_eq!(esc(r#"<a href="x">&'"#), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
    }

    #[test]
    fn head_html_omits_description_when_none() {
        let m = SnippetMeta {
            title: "T".into(),
            description: None,
            url: "https://x/y".into(),
            site_name: "pastedev".into(),
        };
        let html = m.to_head_html();
        assert!(html.contains("<title>T</title>"));
        assert!(html.contains(r#"<meta property="og:title" content="T">"#));
        assert!(!html.contains("og:description"));
        assert!(!html.contains(r#"name="description""#));
    }

    #[test]
    fn head_html_includes_description_when_present() {
        let m = SnippetMeta {
            title: "T".into(),
            description: Some("hello".into()),
            url: "u".into(),
            site_name: "s".into(),
        };
        let html = m.to_head_html();
        assert!(html.contains(r#"<meta property="og:description" content="hello">"#));
        assert!(html.contains(r#"<meta name="description" content="hello">"#));
    }

    #[test]
    fn head_html_escapes_quotes_and_brackets() {
        let m = SnippetMeta {
            title: r#"<x"&>"#.into(),
            description: Some(r#""hi""#.into()),
            url: "u".into(),
            site_name: "s".into(),
        };
        let html = m.to_head_html();
        assert!(html.contains("<title>&lt;x&quot;&amp;&gt;</title>"));
        assert!(html.contains(r#"content="&quot;hi&quot;""#));
    }

    #[test]
    fn md_drops_fenced_code_blocks() {
        let body = "Intro paragraph.\n\n```json\n{\"x\": 1}\n```\n\nAnother line.";
        let out = description_from_markdown(body);
        assert_eq!(out, "Intro paragraph. Another line.");
    }

    #[test]
    fn md_strips_inline_syntax_and_links() {
        let body = "## Heading\n\n*bold* and _em_ and `code` and [link](https://example.com).";
        let out = description_from_markdown(body);
        assert_eq!(out, "Heading bold and em and code and link.");
    }

    #[test]
    fn md_drops_list_markers() {
        let body = "- first\n- second\n- third";
        assert_eq!(description_from_markdown(body), "first second third");
    }

    #[test]
    fn md_truncates_with_ellipsis() {
        let body = "x".repeat(300);
        let out = description_from_markdown(&body);
        assert_eq!(out.chars().count(), 200);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn html_strips_tags_and_decodes_entities() {
        let body = "<h1>Hello &amp; welcome</h1><p>This is <b>bold</b>.</p>";
        assert_eq!(
            description_from_html(body),
            "Hello & welcome This is bold."
        );
    }

    #[test]
    fn html_collapses_whitespace() {
        let body = "<p>a   b\n\n  c</p>";
        assert_eq!(description_from_html(body), "a b c");
    }
}
