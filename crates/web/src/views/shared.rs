//! Helpers shared across the three view pages.

use dioxus::prelude::*;
use pastedev_core::Snippet;

use crate::api;
use crate::components::Modal;
use crate::js::copy_text;
use crate::lib_util::format::size;
use crate::lib_util::time::ago;
use crate::route::Route;
use crate::state::{use_auth, use_toast, AuthState};

/// Predicate: is the signed-in user the snippet owner?
pub fn can_edit(s: &Snippet, auth: &AuthState) -> bool {
    auth.user
        .read()
        .as_ref()
        .is_some_and(|u| u.username == s.owner.username)
}

#[derive(Props, PartialEq, Clone)]
pub struct ViewHeaderProps {
    pub snippet: Snippet,
    #[props(default)]
    pub language: Option<String>,
}

#[component]
pub fn ViewHeader(props: ViewHeaderProps) -> Element {
    let snippet = props.snippet.clone();
    let language = props.language.clone();
    let auth = use_auth();
    let toast = use_toast();
    let nav = use_navigator();
    let editable = can_edit(&snippet, &auth);

    let mut delete_open = use_signal(|| false);

    let kind_label = match snippet.kind {
        pastedev_core::SnippetType::Code => "code",
        pastedev_core::SnippetType::Markdown => "markdown",
        pastedev_core::SnippetType::Html => "html",
    };
    let title = snippet.name.clone().unwrap_or_else(|| "(untitled)".to_string());
    let slug = snippet.slug.clone();
    let raw_url = snippet.raw_url.clone();
    let link_url = snippet.url.clone();
    let body_for_copy = snippet.body.clone();
    let slug_for_delete = snippet.slug.clone();

    let copy_link = {
        let url = link_url.clone();
        move |_| {
            let url = url.clone();
            spawn(async move {
                if let Err(e) = copy_text(&url).await {
                    toast.error(e);
                } else {
                    toast.success("link copied");
                }
            });
        }
    };
    let copy_raw = move |_| {
        let body = body_for_copy.clone();
        spawn(async move {
            if let Err(e) = copy_text(&body).await {
                toast.error(e);
            } else {
                toast.success("raw copied");
            }
        });
    };

    let do_delete = move |_| {
        let slug = slug_for_delete.clone();
        spawn(async move {
            match api::snippets::delete(&slug).await {
                Ok(_) => {
                    toast.success("deleted");
                    nav.replace(Route::Dashboard {});
                }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };

    rsx! {
        div { class: "border-b border-border pb-3 mb-4",
            div { class: "flex flex-wrap items-baseline gap-2 mb-1",
                span { class: "text-[11px] text-accent uppercase tracking-wider", "{kind_label}" }
                span { class: "text-text-faint", "·" }
                span { class: "text-[12px] text-text-faint font-mono", "{slug}" }
                div { class: "ml-auto flex items-center gap-2 text-[11px] text-text-muted flex-wrap",
                    button { class: "hover:text-text", onclick: copy_raw, "copy raw" }
                    button { class: "hover:text-text", onclick: copy_link, "copy link" }
                    a {
                        class: "hover:text-text",
                        href: "{raw_url}",
                        target: "_blank",
                        rel: "noopener",
                        "open ↗"
                    }
                    if editable {
                        Link {
                            class: "hover:text-text",
                            to: Route::Editor { edit: Some(slug.clone()) },
                            "edit"
                        }
                        button {
                            class: "hover:text-danger",
                            onclick: move |_| delete_open.set(true),
                            "×"
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-baseline gap-3 text-[13px]",
                span { class: "text-text", "{title}" }
                span { class: "text-text-faint text-[11px]",
                    "{snippet.owner.username} · {ago(snippet.created_at)} · {snippet.views} views · {size(snippet.size_bytes as i64)}"
                    if let Some(l) = language.as_ref() {
                        span { class: "ml-2 text-accent-dim", "· {l}" }
                    }
                }
            }
        }
        Modal {
            open: delete_open,
            title: Some("delete snippet".to_string()),
            danger: true,
            confirm_label: Some("delete".to_string()),
            on_confirm: Some(EventHandler::new(do_delete)),
            "this will permanently remove the snippet. continue?"
        }
    }
}
