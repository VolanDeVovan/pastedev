//! `/m/:slug` — markdown article.

use dioxus::prelude::*;
use pastedev_core::SnippetType;

use crate::api;
use crate::components::Shell;
use crate::lib_util::markdown;
use crate::route::Route;
use crate::views::shared::ViewHeader;

#[derive(Props, PartialEq, Clone)]
pub struct ViewMarkdownProps {
    pub slug: String,
}

#[component]
pub fn ViewMarkdown(props: ViewMarkdownProps) -> Element {
    let slug = props.slug.clone();
    let snippet = use_resource(move || {
        let slug = slug.clone();
        async move { api::snippets::get(&slug).await }
    });

    use_effect(move || {
        if let Some(Ok(s)) = snippet.read().as_ref() {
            if s.kind != SnippetType::Markdown {
                use_navigator().replace(Route::for_snippet_kind(s.kind, &s.slug));
            }
        }
    });

    let inner = match snippet.read().as_ref() {
        Some(Ok(s)) if s.kind == SnippetType::Markdown => {
            let html = markdown::render(&s.body);
            let snip = s.clone();
            rsx! {
                ViewHeader { snippet: snip }
                article { class: "md-preview", dangerous_inner_html: "{html}" }
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
            div { class: "max-w-[720px] mx-auto px-4 md:px-7 pt-4 md:pt-6 pb-12", {inner} }
        }
    }
}
