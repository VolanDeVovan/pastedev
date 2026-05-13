//! Bottom-right toast queue, fed by ToastQueue context.
//!
//! Each toast picks up the `toast-enter` class on mount for the slide-in
//! keyframe. On dismiss, the toast's id is moved into ToastQueue.leaving
//! and we re-render with `.toast-leave` so the slide-out keyframe runs;
//! the actual removal from items happens 160 ms later (the LEAVE_MS const
//! in state::toast — kept in sync with the @keyframes duration).

use dioxus::prelude::*;

use crate::state::use_toast;

#[component]
pub fn ToastDock() -> Element {
    let toast = use_toast();
    let items = toast.items.read().clone();

    if items.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-[360px]",
            for t in items {
                {
                    let leaving = toast.leaving.read().contains(&t.id);
                    let anim = if leaving { "toast-leave" } else { "toast-enter" };
                    rsx! {
                        div {
                            key: "{t.id}",
                            class: "{anim} bg-bg-deep border border-border-strong rounded-sm px-3 py-2 text-[12px] text-text-dim shadow {t.kind.css()} cursor-pointer",
                            onclick: move |_| toast.dismiss(t.id),
                            "{t.message}"
                        }
                    }
                }
            }
        }
    }
}
