//! `/pending` — three pulsing dots; polls `/auth/me` every 10 s.

use dioxus::prelude::*;
use pastedev_core::UserStatus;

use crate::components::{CenteredCard, Shell};
use crate::route::Route;
use crate::state::use_auth;

#[component]
pub fn PendingPage() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    // Polling loop.
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(10_000).await;
            auth.refresh_me().await;
            let status = auth.user.read().as_ref().map(|u| u.status);
            match status {
                Some(UserStatus::Approved) => {
                    nav.replace(Route::Editor { edit: None });
                    break;
                }
                Some(UserStatus::Rejected) => {
                    nav.replace(Route::Rejected {});
                    break;
                }
                _ => {}
            }
        }
    });

    let do_logout = move |_| {
        spawn(async move {
            let _ = auth.logout().await;
            nav.replace(Route::SignIn { next: None });
        });
    };
    let do_refresh = move |_| {
        spawn(async move {
            auth.refresh_me().await;
        });
    };

    rsx! {
        Shell { CenteredCard { width: Some("420px".to_string()),
            div { class: "flex justify-center mb-6",
                div { class: "flex gap-2",
                    span { class: "w-2 h-2 rounded-full bg-accent", style: "animation: paste-pulse 1.4s infinite both" }
                    span { class: "w-2 h-2 rounded-full bg-accent", style: "animation: paste-pulse 1.4s infinite both; animation-delay: 0.2s" }
                    span { class: "w-2 h-2 rounded-full bg-accent", style: "animation: paste-pulse 1.4s infinite both; animation-delay: 0.4s" }
                }
            }
            h1 { class: "text-[22px] tracking-tight mb-1.5 text-center", "awaiting approval" }
            p { class: "text-[12px] text-text-muted leading-relaxed mb-7 text-center",
                "your request is in the queue. you'll be redirected automatically once an admin acts."
            }
            div { class: "flex justify-center gap-3 text-[12px]",
                button { class: "text-text-muted hover:text-text", onclick: do_refresh, "check status" }
                span { class: "text-text-faint", "·" }
                button { class: "text-text-muted hover:text-text", onclick: do_logout, "sign out" }
            }
        } }
    }
}
