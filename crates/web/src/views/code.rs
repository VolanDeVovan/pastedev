//! `/c/:slug` — public code snippet view.

use dioxus::prelude::*;
use pastedev_core::SnippetType;

use crate::api;
use crate::components::Shell;
use crate::editor::highlight;
use crate::route::Route;
use crate::views::shared::ViewHeader;

#[derive(Props, PartialEq, Clone)]
pub struct ViewCodeProps {
    pub slug: String,
}

#[component]
pub fn ViewCode(props: ViewCodeProps) -> Element {
    let slug = props.slug.clone();
    let snippet = use_resource(move || {
        let slug = slug.clone();
        async move { api::snippets::get(&slug).await }
    });

    // Type-mismatch: redirect to the right prefix.
    use_effect(move || {
        if let Some(Ok(s)) = snippet.read().as_ref() {
            if s.kind != SnippetType::Code {
                use_navigator().replace(Route::for_snippet_kind(s.kind, &s.slug));
            }
        }
    });

    let inner = match snippet.read().as_ref() {
        Some(Ok(s)) if s.kind == SnippetType::Code => {
            let result = highlight::highlight(&s.body, SnippetType::Code);
            let lines = s.body.split('\n').count().max(1);
            let gutter: String = (1..=lines)
                .map(|n| format!("{n}\n"))
                .collect();
            let snip = s.clone();
            let lang = result.language.unwrap_or_else(|| "plain".into());
            rsx! {
                ViewHeader { snippet: snip, language: Some(lang) }
                div { class: "flex border border-border rounded-sm overflow-hidden bg-bg-deep",
                    pre {
                        class: "px-3 py-3 text-right text-[12px] text-text-faint font-mono select-none border-r border-border min-w-[3em]",
                        "{gutter}"
                    }
                    pre {
                        class: "flex-1 px-3 py-3 text-[12px] font-mono text-text overflow-auto",
                        code { class: "hljs", dangerous_inner_html: "{result.html}" }
                    }
                }
            }
        }
        Some(Err(e)) => rsx! {
            div { class: "text-danger text-sm", "{e.message()}" }
        },
        _ => rsx! {
            div { class: "text-text-muted text-sm", "loading…" }
        },
    };

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-4 md:pt-6", {inner} }
        }
    }
}
