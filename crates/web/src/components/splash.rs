//! Renders an empty body while a redirect is mid-flight. Keeps the first
//! frame after a guard decision visibly blank instead of flashing the wrong page.

use dioxus::prelude::*;

#[component]
pub fn SplashFallback() -> Element {
    rsx! {
        div { class: "min-h-screen bg-bg" }
    }
}
