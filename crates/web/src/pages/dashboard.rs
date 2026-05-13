//! `/dashboard` — my snippets, with filter tabs, search, and load-more.

use dioxus::prelude::*;
use pastedev_core::{SnippetListItem, SnippetType};

use crate::api;
use crate::components::Shell;
use crate::lib_util::format::size;
use crate::lib_util::time::ago;
use crate::route::Route;

#[derive(Copy, Clone, PartialEq, Eq)]
enum FilterTab {
    All,
    Code,
    Markdown,
    Html,
}

impl FilterTab {
    fn matches(self, k: SnippetType) -> bool {
        matches!(
            (self, k),
            (FilterTab::All, _)
                | (FilterTab::Code, SnippetType::Code)
                | (FilterTab::Markdown, SnippetType::Markdown)
                | (FilterTab::Html, SnippetType::Html)
        )
    }
}

#[component]
pub fn DashboardPage() -> Element {
    let mut all = use_signal::<Vec<SnippetListItem>>(Vec::new);
    let mut next_cursor = use_signal::<Option<String>>(|| None);
    let mut loading = use_signal(|| false);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut filter = use_signal(|| FilterTab::All);
    let mut query = use_signal(String::new);

    let load_page = move |cursor: Option<String>| {
        spawn(async move {
            loading.set(true);
            error.set(None);
            let opts = api::snippets::ListOpts {
                kind: None,
                cursor,
                limit: Some(50),
            };
            match api::snippets::list(opts).await {
                Ok(resp) => {
                    next_cursor.set(resp.next_cursor.clone());
                    all.write().extend(resp.items.into_iter());
                }
                Err(e) => error.set(Some(e.message().into())),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_page(None);
    });

    let items = all.read().clone();
    let counts = (
        items.len(),
        items.iter().filter(|i| i.kind == SnippetType::Code).count(),
        items.iter().filter(|i| i.kind == SnippetType::Markdown).count(),
        items.iter().filter(|i| i.kind == SnippetType::Html).count(),
    );
    let q = query.read().to_lowercase();
    let q = q.trim().to_string();
    let filtered: Vec<SnippetListItem> = items
        .iter()
        .filter(|i| filter.read().matches(i.kind))
        .filter(|i| {
            q.is_empty()
                || i.name
                    .as_deref()
                    .map(|n| n.to_lowercase().contains(&q))
                    .unwrap_or(false)
                || i.slug.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();

    let total_size: i64 = items.iter().map(|i| i.size_bytes as i64).sum();
    let oldest = items.iter().min_by_key(|i| i.created_at).map(|i| ago(i.created_at));

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-6 pb-10",
                div { class: "flex items-baseline justify-between flex-wrap gap-2 mb-1",
                    h1 { class: "text-[22px] tracking-tight", "my snippets" }
                    div { class: "text-[11px] text-text-faint",
                        "{counts.0} published · {size(total_size)}"
                        if let Some(o) = oldest.as_ref() { ", oldest {o}" }
                    }
                }
                p { class: "text-[12px] text-text-muted mb-5",
                    "snippets you've created. type to filter; switch tabs for code / md / html."
                }
                div { class: "flex flex-wrap items-center gap-2 mb-4",
                    FilterBar { filter, counts }
                    input {
                        class: "bg-bg-deep border border-border-strong rounded-sm px-3 py-1.5 text-[12px] text-text placeholder:text-text-faint flex-1 min-w-[160px]",
                        placeholder: "filter by name or slug",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }

                if let Some(e) = error.read().as_ref() {
                    div { class: "text-[12px] text-danger mb-3", "{e}" }
                }
                if filtered.is_empty() && !*loading.read() {
                    div { class: "text-[12px] text-text-muted py-12 text-center",
                        "no snippets yet. "
                        Link { to: Route::Editor { edit: None }, class: "text-accent hover:opacity-80", "create one →" }
                    }
                } else {
                    div { class: "border border-border rounded-sm divide-y divide-border bg-bg-deep",
                        for item in filtered.iter() {
                            SnippetRow { item: item.clone() }
                        }
                    }
                }

                if next_cursor.read().is_some() {
                    div { class: "mt-4 text-center",
                        button {
                            class: "text-[12px] text-text-muted hover:text-text",
                            disabled: *loading.read(),
                            onclick: move |_| {
                                let cur = next_cursor.read().clone();
                                load_page(cur);
                            },
                            if *loading.read() { "loading…" } else { "load more →" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct FilterBarProps {
    filter: Signal<FilterTab>,
    counts: (usize, usize, usize, usize),
}

#[component]
fn FilterBar(props: FilterBarProps) -> Element {
    let mut filter = props.filter;
    let (all, code, md, html) = props.counts;
    let pill = |active: bool| -> &'static str {
        if active {
            "bg-accent text-bg-deep font-semibold px-2.5 py-1 text-[11px] rounded-sm"
        } else {
            "text-text-muted hover:text-text px-2.5 py-1 text-[11px] rounded-sm border border-border-strong"
        }
    };
    rsx! {
        div { class: "flex flex-wrap gap-1.5",
            button { class: pill(*filter.read() == FilterTab::All),
                onclick: move |_| filter.set(FilterTab::All),
                "all · {all}"
            }
            button { class: pill(*filter.read() == FilterTab::Code),
                onclick: move |_| filter.set(FilterTab::Code),
                "code · {code}"
            }
            button { class: pill(*filter.read() == FilterTab::Markdown),
                onclick: move |_| filter.set(FilterTab::Markdown),
                "md · {md}"
            }
            button { class: pill(*filter.read() == FilterTab::Html),
                onclick: move |_| filter.set(FilterTab::Html),
                "html · {html}"
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct SnippetRowProps {
    item: SnippetListItem,
}

#[component]
fn SnippetRow(props: SnippetRowProps) -> Element {
    let item = props.item;
    let route = Route::for_snippet_kind(item.kind, &item.slug);
    let (type_cls, type_label) = match item.kind {
        SnippetType::Code => ("text-blue-300 bg-blue-300/10 border-blue-300/30", "code"),
        SnippetType::Markdown => ("text-accent bg-accent/10 border-accent/30", "md"),
        SnippetType::Html => ("text-warn bg-warn/10 border-warn/30", "html"),
    };
    let title = item
        .name
        .clone()
        .unwrap_or_else(|| "(untitled)".to_string());
    rsx! {
        Link { to: route, class: "block px-3 py-2 hover:bg-bg/40",
            div { class: "flex flex-wrap items-baseline gap-3 text-[12px]",
                span { class: "{type_cls} border px-1.5 py-px text-[10px] rounded-sm uppercase tracking-wider", "{type_label}" }
                span { class: "text-text-faint font-mono text-[11px]", "{item.slug}" }
                span { class: "text-text", "{title}" }
                span { class: "ml-auto text-text-faint text-[11px]",
                    "{size(item.size_bytes as i64)} · {item.views} views · {ago(item.created_at)}"
                }
            }
        }
    }
}
