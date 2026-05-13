//! `/setup` — first-admin wizard (env check → create admin).

use dioxus::prelude::*;
use pastedev_core::SetupAdminRequest;

use crate::api;
use crate::components::{FormField, Shell};
use crate::route::Route;
use crate::state::use_auth;

#[component]
pub fn SetupPage() -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    let mut step = use_signal(|| 1u8);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut submitting = use_signal(|| false);

    // Poll setup status every 2 seconds while in step 1.
    use_future(move || async move {
        loop {
            if *step.read() != 1 {
                gloo_timers::future::TimeoutFuture::new(2_000).await;
                continue;
            }
            if let Ok(s) = api::auth::setup_status().await {
                if !s.needs_setup {
                    nav.replace(Route::Editor { edit: None });
                    break;
                }
                // Push checks into auth.setup so other consumers see them too.
                let mut setup = auth.setup;
                setup.set(Some(s));
            }
            gloo_timers::future::TimeoutFuture::new(2_000).await;
        }
    });

    let checks = auth
        .setup
        .read()
        .as_ref()
        .map(|s| s.checks.clone())
        .unwrap_or_default();
    let all_ok = !checks.is_empty() && checks.iter().all(|c| c.status == "ok");
    let version = auth
        .setup
        .read()
        .as_ref()
        .and_then(|s| s.version.clone())
        .unwrap_or_else(|| "0.1.0".to_string());

    let go_step2 = move |_| {
        if all_ok {
            step.set(2);
        }
    };

    let submit = move |e: FormEvent| {
        e.prevent_default();
        spawn(async move {
            submitting.set(true);
            error.set(None);
            let email_val = email.read().trim().to_string();
            let req = SetupAdminRequest {
                username: username.read().trim().to_string(),
                email: if email_val.is_empty() { None } else { Some(email_val) },
                password: password.read().clone(),
            };
            match auth.setup_admin(req).await {
                Ok(_) => {
                    nav.replace(Route::Editor { edit: None });
                }
                Err(e) => error.set(Some(e.message().into())),
            }
            submitting.set(false);
        });
    };

    rsx! {
        Shell {
            div { class: "max-w-3xl mx-auto px-4 md:px-7 pt-6 pb-12",
                div { class: "mb-6 flex items-baseline justify-between",
                    h1 { class: "text-[22px] tracking-tight", "setup" }
                    span { class: "text-[11px] text-text-faint", "pastedev v{version}" }
                }

                if *step.read() == 1 {
                    p { class: "text-[12px] text-text-muted leading-relaxed mb-4",
                        "environment check. continue is enabled once all checks pass."
                    }
                    div { class: "flex flex-col divide-y divide-border border border-border rounded-sm mb-6",
                        for c in checks.iter() {
                            CheckRow {
                                id: c.id.clone(),
                                status: c.status.clone(),
                                detail: c.detail.clone(),
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        disabled: !all_ok,
                        onclick: go_step2,
                        class: "bg-accent text-bg-deep font-semibold px-3 py-2 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30",
                        "continue →"
                    }
                } else {
                    p { class: "text-[12px] text-text-muted leading-relaxed mb-4",
                        "create the first admin. this account is auto-approved and the next sign-in works immediately."
                    }
                    form { onsubmit: submit,
                        FormField { label: "admin username".to_string(), value: username, autocomplete: Some("username".to_string()), required: true }
                        FormField { label: "email (optional)".to_string(), value: email, kind: Some("email".to_string()), autocomplete: Some("email".to_string()) }
                        FormField { label: "password".to_string(), value: password, kind: Some("password".to_string()), autocomplete: Some("new-password".to_string()), required: true, hint: Some("≥ 8 chars".to_string()) }
                        if let Some(e) = error.read().as_ref() { div { class: "text-[12px] text-danger mb-3", "{e}" } }
                        button {
                            r#type: "submit",
                            disabled: *submitting.read(),
                            class: "bg-accent text-bg-deep font-semibold px-3 py-2 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30",
                            if *submitting.read() { "creating…" } else { "create admin →" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct CheckRowProps {
    id: String,
    status: String,
    detail: String,
}

#[component]
fn CheckRow(props: CheckRowProps) -> Element {
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
