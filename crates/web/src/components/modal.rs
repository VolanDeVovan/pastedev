//! Simple modal dialog. Backdrop click + Escape close it. Danger variant
//! draws a rose stripe down the left edge.

use dioxus::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[derive(Props, PartialEq, Clone)]
pub struct ModalProps {
    pub open: Signal<bool>,
    #[props(default)]
    pub title: Option<String>,
    #[props(default)]
    pub danger: bool,
    #[props(default)]
    pub confirm_label: Option<String>,
    pub children: Element,
    #[props(default)]
    pub on_confirm: Option<EventHandler<()>>,
}

#[component]
pub fn Modal(props: ModalProps) -> Element {
    let mut open = props.open;

    // ESC closes.
    use_effect(move || {
        let cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Escape" {
                open.set(false);
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    });

    if !*open.read() {
        return rsx! {};
    }

    let stripe = if props.danger { "bg-danger" } else { "bg-accent" };
    let confirm_label = props.confirm_label.clone().unwrap_or_else(|| "confirm".to_string());
    let confirm_cls = if props.danger {
        "px-3 py-1.5 bg-danger/20 text-danger border border-danger-border hover:bg-danger/30 rounded-sm"
    } else {
        "px-3 py-1.5 bg-accent text-bg-deep font-semibold hover:opacity-90 rounded-sm"
    };
    let on_confirm = props.on_confirm;

    rsx! {
        div {
            class: "modal-enter fixed inset-0 z-50 flex items-center justify-center bg-bg/80 backdrop-blur-sm",
            onmousedown: move |e| {
                // Backdrop click closes; clicks on the panel are stopped below.
                let _ = e;
                open.set(false);
            },
            div {
                class: "modal-panel-enter bg-bg-deep border border-border-strong rounded-sm w-[440px] max-w-[calc(100vw-2rem)] relative overflow-hidden",
                onmousedown: |e| e.stop_propagation(),
                span { class: "absolute left-0 top-0 bottom-0 w-px {stripe}" }
                div { class: "flex items-start justify-between px-5 pt-4 pb-3 border-b border-border",
                    div { class: "text-[13px] text-text",
                        if let Some(t) = props.title.as_ref() { "{t}" }
                    }
                    button {
                        class: "text-text-muted hover:text-text",
                        onclick: move |_| open.set(false),
                        "×"
                    }
                }
                div { class: "px-5 py-4 text-[12px] text-text-dim leading-relaxed", {props.children} }
                div { class: "flex justify-end items-center gap-2 px-4 py-3 border-t border-border bg-bg/40",
                    button {
                        class: "px-3 py-1.5 text-text-muted hover:text-text",
                        onclick: move |_| open.set(false),
                        "cancel"
                    }
                    button {
                        class: "{confirm_cls}",
                        onclick: move |_| {
                            if let Some(h) = on_confirm.as_ref() { h.call(()); }
                            open.set(false);
                        },
                        "{confirm_label}"
                    }
                }
            }
        }
    }
}
