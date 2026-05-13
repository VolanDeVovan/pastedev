//! Bottom-right toast queue, fed by ToastQueue context.
//!
//! Each toast picks up the `toast-enter` class which kicks off the
//! slide-in keyframe defined in tailwind.css. We don't run a leave
//! animation: dismissal just removes the item — fast and predictable.

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
                div {
                    key: "{t.id}",
                    class: "toast-enter bg-bg-deep border border-border-strong rounded-sm px-3 py-2 text-[12px] text-text-dim shadow {t.kind.css()} cursor-pointer",
                    onclick: move |_| toast.dismiss(t.id),
                    "{t.message}"
                }
            }
        }
    }
}
