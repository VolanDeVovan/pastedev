//! `/rejected` — terminal page, sign-out only.

use dioxus::prelude::*;

use crate::components::{CenteredCard, Shell};
use crate::route::Route;
use crate::state::use_auth;

#[component]
pub fn RejectedPage() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    let do_logout = move |_| {
        spawn(async move {
            let _ = auth.logout().await;
            nav.replace(Route::SignIn { next: None });
        });
    };

    rsx! {
        Shell { CenteredCard { width: Some("420px".to_string()),
            h1 { class: "text-[22px] tracking-tight mb-1.5", "your request was declined" }
            p { class: "text-[12px] text-text-muted leading-relaxed mb-7",
                "an admin chose not to approve this account. if you believe this is a mistake, please reach out through another channel."
            }
            button {
                class: "text-[12px] text-text-muted hover:text-text",
                onclick: do_logout,
                "sign out"
            }
        } }
    }
}
