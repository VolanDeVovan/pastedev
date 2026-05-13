//! Editor page: textarea over hljs-coloured overlay, with a synced line gutter
//! for code/html. Markdown gets a split-pane preview. Cmd/Ctrl+Enter publishes.
//!
//! The overlay is a `<pre>` painted with the hljs-coloured HTML; the textarea
//! sits on top with `color: transparent` and `caret-color: text` so the user
//! types into the textarea while the colors show through. Scroll is synced
//! manually via JS interop (translateY on the gutter, scrollTop on the pre).

pub mod highlight;

use dioxus::prelude::*;
use pastedev_core::{CreateSnippetRequest, PatchSnippetRequest, SnippetType, MAX_SNIPPET_BYTES};
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlTextAreaElement};

use crate::api;
use crate::components::Shell;
use crate::lib_util::{format::size, markdown};
use crate::route::Route;
use crate::state::{use_auth, use_toast};

#[derive(Props, PartialEq, Clone)]
pub struct EditorPageProps {
    pub edit: Option<String>,
}

#[component]
pub fn EditorPage(props: EditorPageProps) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let nav = use_navigator();

    let mut kind = use_signal(|| SnippetType::Code);
    let mut body = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut editing = use_signal::<Option<String>>(|| None);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut submitting = use_signal(|| false);
    let detected_lang = use_signal::<Option<String>>(|| None);

    let edit_slug = props.edit.clone();

    // Edit-mode load: when ?edit=<slug>, fetch & prefill once.
    let loaded = use_resource(move || {
        let slug = edit_slug.clone();
        async move {
            match slug {
                Some(s) => api::snippets::get(&s).await.map(Some),
                None => Ok(None),
            }
        }
    });

    // Push the loaded snippet into form state when it arrives.
    use_effect(move || {
        if let Some(Ok(Some(s))) = loaded.read().as_ref() {
            editing.set(Some(s.slug.clone()));
            kind.set(s.kind);
            body.set(s.body.clone());
            name.set(s.name.clone().unwrap_or_default());
        }
    });

    let do_submit = move || {
        spawn(async move {
            let b = body.read().clone();
            if b.is_empty() {
                error.set(Some("body is empty".to_string()));
                return;
            }
            if b.len() > MAX_SNIPPET_BYTES {
                error.set(Some("body exceeds 1 MB".to_string()));
                return;
            }
            submitting.set(true);
            error.set(None);
            let trimmed_name = name.read().trim().to_string();
            let result = match editing.read().clone() {
                Some(slug) => api::snippets::patch(
                    &slug,
                    &PatchSnippetRequest {
                        body: Some(b),
                        name: if trimmed_name.is_empty() {
                            None
                        } else {
                            Some(trimmed_name)
                        },
                    },
                )
                .await
                .map(|s| (s.kind, s.slug)),
                None => api::snippets::create(&CreateSnippetRequest {
                    kind: kind(),
                    name: if trimmed_name.is_empty() {
                        None
                    } else {
                        Some(trimmed_name)
                    },
                    body: b,
                })
                .await
                .map(|s| (s.kind, s.slug)),
            };
            submitting.set(false);
            match result {
                Ok((k, slug)) => {
                    nav.replace(Route::for_snippet_kind(k, &slug));
                }
                Err(e) => {
                    if e.is_forbidden() {
                        auth.refresh_me().await;
                    }
                    let msg = e.message().to_string();
                    error.set(Some(msg.clone()));
                    toast.error(msg);
                }
            }
        });
    };

    let on_key = {
        move |e: KeyboardEvent| {
            let mods = e.modifiers();
            if (mods.meta() || mods.ctrl()) && e.key() == Key::Enter {
                e.prevent_default();
                do_submit();
            }
        }
    };

    let body_len = body.read().len();
    let over_limit = body_len > MAX_SNIPPET_BYTES;
    let counter_cls = if over_limit { "text-danger" } else { "text-text-faint" };
    let lang = detected_lang.read().clone();
    // The hljs binding bails to plain escaped HTML once a body crosses 1.1 MB
    // (it'd block the main thread otherwise). Surface that to the user.
    const HL_OFF_THRESHOLD: usize = 1_100_000;
    let hl_off = body_len > HL_OFF_THRESHOLD;

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-4 md:pt-6 pb-8",
                EditorToolbar {
                    kind: kind,
                    name: name,
                    editing: editing,
                    submitting: submitting,
                    over_limit: over_limit,
                    on_submit: do_submit,
                }
                EditorBody {
                    kind: kind,
                    body: body,
                    on_key: on_key,
                    detected_lang: detected_lang,
                }
                div { class: "flex items-center justify-between mt-3 text-[11px] {counter_cls}",
                    div { class: "flex items-center gap-3",
                        span { "{size(body_len as i64)} / 1.0 MB" }
                        if hl_off {
                            span { class: "text-warn", "hl off · large file" }
                        } else if let Some(l) = lang.as_ref() {
                            span { class: "text-accent-dim", "lang · {l}" }
                        }
                    }
                    if let Some(e) = error.read().as_ref() {
                        span { class: "text-danger", "{e}" }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct EditorToolbarProps {
    kind: Signal<SnippetType>,
    name: Signal<String>,
    editing: Signal<Option<String>>,
    submitting: Signal<bool>,
    over_limit: bool,
    on_submit: EventHandler<()>,
}

#[component]
fn EditorToolbar(props: EditorToolbarProps) -> Element {
    let kind = props.kind;
    let name = props.name;
    let editing = props.editing;
    let submitting = props.submitting;
    let on_submit = props.on_submit;
    let over_limit = props.over_limit;
    let editing_lock = editing.read().is_some();

    rsx! {
        div { class: "flex flex-wrap items-center gap-2 md:gap-3 mb-3",
            // Type tabs
            div { class: "flex border border-border-strong rounded-sm overflow-hidden",
                TypeTab { kind, this: SnippetType::Code,     label: "code",     locked: editing_lock }
                TypeTab { kind, this: SnippetType::Markdown, label: "md",       locked: editing_lock }
                TypeTab { kind, this: SnippetType::Html,     label: "html",     locked: editing_lock }
            }
            // Filename
            input {
                class: "bg-bg-deep border border-border-strong rounded-sm px-3 py-1.5 text-[12px] text-text placeholder:text-text-faint flex-1 min-w-[140px]",
                placeholder: "name (optional)",
                value: "{name.read()}",
                oninput: move |e| name.clone().set(e.value()),
            }
            // Decorative chips
            span { class: "hidden md:inline text-[11px] text-text-faint", "expires:never" }
            span { class: "hidden md:inline text-[11px] text-text-faint", "visibility:public" }
            // Publish
            button {
                r#type: "button",
                class: "bg-accent text-bg-deep font-semibold px-3 py-1.5 text-[12px] rounded-sm hover:opacity-90 disabled:opacity-30 ml-auto",
                disabled: *submitting.read() || over_limit,
                onclick: move |_| on_submit.call(()),
                if *submitting.read() {
                    "publishing…"
                } else if editing_lock {
                    "save ⌘↵"
                } else {
                    "publish ⌘↵"
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct TypeTabProps {
    kind: Signal<SnippetType>,
    this: SnippetType,
    label: String,
    locked: bool,
}

#[component]
fn TypeTab(props: TypeTabProps) -> Element {
    let mut kind = props.kind;
    let is_active = *kind.read() == props.this;
    let cls = if is_active {
        "bg-accent text-bg-deep px-3 py-1.5 text-[12px] font-semibold"
    } else {
        "bg-bg-deep text-text-muted px-3 py-1.5 text-[12px] hover:text-text"
    };
    let this = props.this;
    let locked = props.locked;
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            disabled: locked,
            onclick: move |_| if !locked { kind.set(this) },
            "{props.label}"
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct EditorBodyProps {
    kind: Signal<SnippetType>,
    body: Signal<String>,
    on_key: EventHandler<KeyboardEvent>,
    detected_lang: Signal<Option<String>>,
}

#[component]
fn EditorBody(props: EditorBodyProps) -> Element {
    let kind = props.kind;
    let body = props.body;
    let on_key = props.on_key;
    let mut detected_lang = props.detected_lang;

    let body_str = body.read().clone();
    let k = *kind.read();

    // Re-highlight on debounced body changes. The closure tracks body+kind
    // signals; use_resource re-runs when either changes and discards the
    // previous future automatically. hljs runs off-main-thread via a Web
    // Worker, so the UI stays responsive on big pastes.
    let highlighted = use_resource(move || {
        let body = body.read().clone();
        let k = *kind.read();
        async move {
            // 150 ms debounce so we don't enqueue a worker job on every keystroke.
            gloo_timers::future::TimeoutFuture::new(150).await;
            if matches!(k, SnippetType::Markdown) {
                return (String::new(), None);
            }
            let r = highlight::request_async(&body, k).await;
            (r.html, r.language)
        }
    });

    // "No flash" cache: hold the most recent highlighted HTML so we keep
    // painting it across keystrokes while the next highlight is in flight.
    // Plan reference: plans/06-editor.html — MVP-3 "cache + fast paths".
    let mut last_good = use_signal::<Option<(String, Option<String>)>>(|| None);
    use_effect(move || {
        if let Some(pair) = highlighted.read().as_ref() {
            last_good.set(Some(pair.clone()));
            detected_lang.set(pair.1.clone());
        }
    });

    let overlay_html = last_good
        .read()
        .as_ref()
        .map(|(h, _)| h.clone())
        .unwrap_or_else(|| crate::lib_util::format::escape_html(&body_str));

    if k == SnippetType::Markdown {
        return rsx! {
            div { class: "border border-border-strong rounded-sm overflow-hidden",
                div { class: "grid grid-cols-1 md:grid-cols-2 min-h-[60vh]",
                    textarea {
                        class: "block w-full bg-bg-deep text-text font-mono text-[13px] p-3 md:p-4 outline-none resize-none min-h-[60vh] border-r border-border",
                        value: "{body_str}",
                        placeholder: "# heading\n\nstart writing markdown — preview renders on the right.",
                        oninput: move |e| body.clone().set(e.value()),
                        onkeydown: move |e| on_key.call(e),
                    }
                    div {
                        class: "md-preview overflow-auto p-3 md:p-4 bg-bg",
                        dangerous_inner_html: "{markdown::render(&body_str)}",
                    }
                }
            }
        };
    }

    // Code / HTML mode: textarea + colored overlay + line-number gutter.
    let lines = body_str.split('\n').count().max(1);
    let gutter: String = (1..=lines).map(|n| format!("{n}\n")).collect();

    let mut textarea_ref = use_signal::<Option<HtmlTextAreaElement>>(|| None);
    let mut overlay_ref = use_signal::<Option<HtmlElement>>(|| None);
    let mut gutter_ref = use_signal::<Option<HtmlElement>>(|| None);

    let sync_scroll = move || {
        let (Some(ta), Some(ov)) = (textarea_ref.peek().clone(), overlay_ref.peek().clone())
        else {
            return;
        };
        ov.set_scroll_top(ta.scroll_top());
        ov.set_scroll_left(ta.scroll_left());
        if let Some(g) = gutter_ref.peek().clone() {
            let _ = g.style().set_property(
                "transform",
                &format!("translateY({}px)", -ta.scroll_top()),
            );
        }
    };

    // Editor metrics — kept identical across gutter / overlay / textarea so
    // line N in the gutter sits at the same Y as line N in the overlay AND
    // in the textarea's caret. Touch all three together or rows drift apart.
    const EDIT_METRICS: &str = "text-[13px] font-mono leading-relaxed";

    rsx! {
        div { class: "border border-border-strong rounded-sm overflow-hidden flex bg-bg-deep min-h-[60vh]",
            // Line gutter
            div { class: "shrink-0 overflow-hidden border-r border-border bg-bg-deep min-w-[3em]",
                pre {
                    class: "px-3 py-3 md:py-4 text-right text-text-faint select-none {EDIT_METRICS}",
                    onmounted: move |c| {
                        if let Some(el) = c.downcast::<web_sys::Element>().and_then(|e| e.clone().dyn_into::<HtmlElement>().ok()) {
                            gutter_ref.set(Some(el));
                        }
                    },
                    "{gutter}"
                }
            }
            // Stacked overlay + textarea
            div { class: "relative flex-1 min-h-[60vh] overflow-hidden",
                pre {
                    class: "absolute inset-0 m-0 px-3 py-3 md:py-4 overflow-auto pointer-events-none whitespace-pre-wrap break-words {EDIT_METRICS}",
                    onmounted: move |c| {
                        if let Some(el) = c.downcast::<web_sys::Element>().and_then(|e| e.clone().dyn_into::<HtmlElement>().ok()) {
                            overlay_ref.set(Some(el));
                        }
                    },
                    code { class: "hljs", dangerous_inner_html: "{overlay_html}" }
                }
                textarea {
                    class: "editor-textarea absolute inset-0 m-0 w-full h-full px-3 py-3 md:py-4 resize-none outline-none whitespace-pre-wrap break-words {EDIT_METRICS}",
                    // Inline because Tailwind utility cascade order doesn't
                    // override the textarea UA color/background reliably across
                    // browsers. The hljs-painted <pre> is what the user actually
                    // sees; the textarea owns caret + selection only.
                    style: "background: transparent; color: transparent; caret-color: var(--color-text);",
                    value: "{body_str}",
                    placeholder: match k {
                        SnippetType::Code => "paste or write code…",
                        SnippetType::Html => "<!doctype html>\n<html>…</html>",
                        SnippetType::Markdown => "",
                    },
                    spellcheck: "false",
                    onmounted: move |c| {
                        if let Some(el) = c.downcast::<web_sys::Element>().and_then(|e| e.clone().dyn_into::<HtmlTextAreaElement>().ok()) {
                            textarea_ref.set(Some(el));
                        }
                    },
                    oninput: move |e| {
                        body.clone().set(e.value());
                        sync_scroll();
                    },
                    onscroll: move |_| sync_scroll(),
                    onkeydown: move |e| on_key.call(e),
                }
            }
        }
    }
}
