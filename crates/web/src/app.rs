//! Root component: providers + Router + global ToastDock.

use dioxus::prelude::*;

use crate::components::ToastDock;
use crate::route::Route;
use crate::state::{provide_auth, provide_toast};

#[component]
pub fn App() -> Element {
    // Provide context once at the root; every component grabs them via
    // use_auth() / use_toast(). The Tailwind stylesheet is linked statically
    // from index.html — see the comment there.
    provide_auth();
    provide_toast();

    rsx! {
        Router::<Route> {}
        ToastDock {}
    }
}
