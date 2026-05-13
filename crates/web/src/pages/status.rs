//! `/status` — server-side health checks. 5 s when all ok, 15 s otherwise.

use dioxus::prelude::*;

use crate::api;
use crate::components::Shell;

#[component]
pub fn StatusPage() -> Element {
    let mut data = use_signal::<Option<pastedev_core::SetupStatus>>(|| None);
    let mut last_err = use_signal::<Option<String>>(|| None);

    use_future(move || async move {
        loop {
            match api::auth::setup_status().await {
                Ok(s) => {
                    data.set(Some(s));
                    last_err.set(None);
                }
                Err(e) => last_err.set(Some(e.message().into())),
            }
            let all_ok = data
                .read()
                .as_ref()
                .map(|s| s.checks.iter().all(|c| c.status == "ok"))
                .unwrap_or(false);
            let delay = if all_ok { 5_000 } else { 15_000 };
            gloo_timers::future::TimeoutFuture::new(delay).await;
        }
    });

    let do_refresh = move |_| {
        spawn(async move {
            if let Ok(s) = api::auth::setup_status().await {
                data.set(Some(s));
            }
        });
    };

    let checks = data
        .read()
        .as_ref()
        .map(|s| s.checks.clone())
        .unwrap_or_default();
    let overall = if checks.iter().all(|c| c.status == "ok") {
        ("ok", "text-accent")
    } else if checks.iter().any(|c| c.status == "err") {
        ("error", "text-danger")
    } else {
        ("warn", "text-warn")
    };

    rsx! {
        Shell {
            div { class: "max-w-3xl mx-auto px-4 md:px-7 pt-6 pb-12",
                div { class: "flex items-baseline justify-between mb-6",
                    h1 { class: "text-[20px] tracking-tight", "service status" }
                    div { class: "flex items-center gap-3 text-[12px]",
                        span { class: "{overall.1}", "{overall.0}" }
                        button { class: "text-text-muted hover:text-text", onclick: do_refresh, "refresh now" }
                    }
                }
                if let Some(e) = last_err.read().as_ref() {
                    div { class: "text-[12px] text-danger mb-4", "{e}" }
                }
                div { class: "flex flex-col divide-y divide-border border border-border rounded-sm",
                    for c in checks {
                        StatusRow { id: c.id.clone(), status: c.status.clone(), detail: c.detail.clone() }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct StatusRowProps {
    id: String,
    status: String,
    detail: String,
}

#[component]
fn StatusRow(props: StatusRowProps) -> Element {
    let dot_cls = match props.status.as_str() {
        "ok" => "bg-accent",
        "warn" => "bg-warn",
        "err" => "bg-danger",
        _ => "bg-text-faint",
    };
    rsx! {
        div { class: "flex items-center gap-3 px-3 py-2.5 text-[12px]",
            span { class: "w-2 h-2 rounded-full {dot_cls}" }
            span { class: "text-text-dim min-w-[120px]", "{props.id}" }
            span { class: "text-text-muted truncate", "{props.detail}" }
        }
    }
}
