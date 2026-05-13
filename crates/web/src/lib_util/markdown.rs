//! pulldown-cmark wrapper. Escapes raw HTML (we don't enable Options::ENABLE_HTML).
//!
//! Fenced code blocks intercept the event stream: when we hit a code-block
//! Start tag we collect the inner text and run hljs over it, then emit a
//! ready-to-insert `<pre><code class="hljs language-{lang}">...</code></pre>`.
//! This mirrors `markdown-it`'s `highlight` option used by the Vue version
//! (web/src/lib/markdown.ts).

use pastedev_core::SnippetType;
use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::editor::highlight;
use crate::lib_util::format::escape_html;

pub fn render(src: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(src, opts);
    let events = transform_fenced_code(parser);

    let mut out = String::with_capacity(src.len() * 2);
    html::push_html(&mut out, events.into_iter());
    out
}

/// Replaces fenced-code event runs with a single Html event containing the
/// hljs-coloured markup. Pulldown-cmark emits Start(CodeBlock) → Text(s)+ →
/// End(CodeBlock); we collapse that into one block.
fn transform_fenced_code<'a, I>(parser: I) -> Vec<Event<'a>>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut out: Vec<Event<'a>> = Vec::new();
    let mut buf = String::new();
    let mut lang: Option<String> = None;
    let mut in_block = false;

    for ev in parser {
        match ev {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_block = true;
                buf.clear();
                lang = match kind {
                    CodeBlockKind::Fenced(s) => {
                        let s = s.trim();
                        if s.is_empty() { None } else { Some(s.to_string()) }
                    }
                    CodeBlockKind::Indented => None,
                };
            }
            Event::End(TagEnd::CodeBlock) if in_block => {
                in_block = false;
                let body = std::mem::take(&mut buf);
                let html = render_code_block(&body, lang.take());
                out.push(Event::Html(html.into()));
            }
            Event::Text(t) if in_block => {
                buf.push_str(&t);
            }
            other => out.push(other),
        }
    }
    out
}

fn render_code_block(body: &str, lang: Option<String>) -> String {
    // When a language hint is present and hljs recognises it, run the
    // language-specific path. Otherwise auto-detect via the Code path
    // (matches the Vue highlightFence fallback).
    let kind = if lang.is_some() {
        SnippetType::Code
    } else {
        SnippetType::Code
    };

    // We can't directly pass the lang to highlight::highlight (it takes a
    // SnippetType), so route through it for auto-detect and rely on hljs's
    // detection. For an explicit language hint we honour it via a custom
    // call path below.
    let result = if let Some(name) = lang.as_deref() {
        highlight::highlight_with_lang(body, name)
    } else {
        highlight::highlight(body, kind)
    };

    let lang_class = result
        .language
        .as_deref()
        .or(lang.as_deref())
        .map(|l| format!(" language-{}", html_attr(l)))
        .unwrap_or_default();

    if result.html.is_empty() {
        format!(
            "<pre><code class=\"hljs{}\">{}</code></pre>",
            lang_class,
            escape_html(body),
        )
    } else {
        format!(
            "<pre><code class=\"hljs{}\">{}</code></pre>",
            lang_class, result.html,
        )
    }
}

/// Belt-and-braces escape for the `language-...` class — should always be a
/// safe identifier but the wire is user-controlled, so be defensive.
fn html_attr(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.'))
        .collect()
}
