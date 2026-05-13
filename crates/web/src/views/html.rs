//! `/h/:slug` — sandboxed HTML preview via iframe + size-reporter postMessage.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use pastedev_core::SnippetType;
use serde::Deserialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::MessageEvent;

use crate::api;
use crate::components::Shell;
use crate::route::Route;
use crate::views::shared::ViewHeader;

// Tracks the smallest size reported recently to avoid feedback loops.
const SIZE_SLACK: u32 = 4;

#[derive(Deserialize, Default)]
struct SizeMsg {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    width: Option<u32>,
}

#[derive(Props, PartialEq, Clone)]
pub struct ViewHtmlProps {
    pub slug: String,
}

#[component]
pub fn ViewHtml(props: ViewHtmlProps) -> Element {
    let slug = props.slug.clone();
    let snippet = use_resource(move || {
        let slug = slug.clone();
        async move { api::snippets::get(&slug).await }
    });

    use_effect(move || {
        if let Some(Ok(s)) = snippet.read().as_ref() {
            if s.kind != SnippetType::Html {
                use_navigator().replace(Route::for_snippet_kind(s.kind, &s.slug));
            }
        }
    });

    let mut height = use_signal(|| 400u32);
    let mut width = use_signal::<Option<u32>>(|| None);

    // window 'message' listener for size reports.
    use_effect(move || {
        let cb = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let data = e.data();
            let parsed: SizeMsg = serde_wasm_bindgen::from_value(data).unwrap_or_default();
            if parsed.r#type != "pastedev:size" {
                return;
            }
            if let Some(h) = parsed.height {
                let cur = *height.read();
                if h + SIZE_SLACK < cur || h > cur + SIZE_SLACK {
                    height.set(h);
                }
            }
            if let Some(w) = parsed.width {
                let cur = width.read().unwrap_or(0);
                if w + SIZE_SLACK < cur || w > cur + SIZE_SLACK {
                    width.set(Some(w));
                }
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("message", cb.as_ref().unchecked_ref());
        }
        // Leak the closure for the page lifetime — we never unmount cleanly during
        // a session and the page reload drops the closure with the document.
        cb.forget();
    });

    let inner = match snippet.read().as_ref() {
        Some(Ok(s)) if s.kind == SnippetType::Html => {
            let snip = s.clone();
            let raw_url = s.raw_url.clone();
            let title = s.name.clone().unwrap_or_else(|| s.slug.clone());
            let h = *height.read();
            let w_opt = *width.read();
            let w_style = w_opt
                .map(|x| format!("{x}px"))
                .unwrap_or_else(|| "100%".to_string());
            let style = format!("height:{h}px;width:{w_style};min-width:100%");
            rsx! {
                ViewHeader { snippet: snip }
                div { class: "mb-2 text-[11px] text-warn", "user-published html · sandboxed (no app-origin access)" }
                div { class: "overflow-x-auto",
                    iframe {
                        src: "{raw_url}",
                        // sandbox is a non-standard Dioxus attr; pass it raw.
                        "sandbox": "allow-scripts allow-popups",
                        "referrerpolicy": "no-referrer",
                        "scrolling": "no",
                        title: "{title}",
                        class: "block bg-white border border-border rounded-sm",
                        style: "{style}",
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

    // Suppress unused warnings on Rc refcells if we add them later.
    let _ = Rc::new(RefCell::new(()));

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-4 md:pt-6 pb-8", {inner} }
        }
    }
}
