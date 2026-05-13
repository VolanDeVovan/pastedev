//! `/register` — username, email (opt), password, reason (≥ 10 chars).

use dioxus::prelude::*;
use pastedev_core::RegisterRequest;

use crate::components::{CenteredCard, FormField, Shell};
use crate::route::Route;
use crate::state::use_auth;

#[component]
pub fn RegisterPage() -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut submitting = use_signal(|| false);

    let submit = move |e: FormEvent| {
        e.prevent_default();
        spawn(async move {
            let r = reason.read().trim().to_string();
            if r.len() < 10 {
                error.set(Some("please share why you'd like access (≥ 10 chars)".to_string()));
                return;
            }
            submitting.set(true);
            error.set(None);
            let email_val = email.read().trim().to_string();
            let req = RegisterRequest {
                username: username.read().trim().to_string(),
                email: if email_val.is_empty() { None } else { Some(email_val) },
                password: password.read().clone(),
                reason: Some(r),
            };
            match auth.register(req).await {
                Ok(_) => { nav.replace(Route::Pending {}); }
                Err(e) => error.set(Some(e.message().into())),
            }
            submitting.set(false);
        });
    };

    rsx! {
        Shell { CenteredCard { width: Some("440px".to_string()),
            h1 { class: "text-[22px] tracking-tight mb-1.5", "register" }
            p { class: "text-[12px] text-text-muted leading-relaxed mb-7",
                "access is review-gated. admins approve manually."
            }
            form { onsubmit: submit,
                FormField {
                    label: "username".to_string(),
                    value: username,
                    hint: Some("3–40 chars, [a-z0-9._-]".to_string()),
                    autocomplete: Some("username".to_string()),
                    required: true,
                }
                FormField {
                    label: "email (optional)".to_string(),
                    value: email,
                    kind: Some("email".to_string()),
                    autocomplete: Some("email".to_string()),
                }
                FormField {
                    label: "password".to_string(),
                    value: password,
                    kind: Some("password".to_string()),
                    autocomplete: Some("new-password".to_string()),
                    hint: Some("≥ 8 chars".to_string()),
                    required: true,
                }
                FormField {
                    label: "why you'd like access".to_string(),
                    value: reason,
                    rows: Some(4),
                    hint: Some("at least 10 chars".to_string()),
                    required: true,
                }
                if let Some(e) = error.read().as_ref() {
                    div { class: "text-[12px] text-danger mb-3", "{e}" }
                }
                button {
                    r#type: "submit",
                    disabled: *submitting.read(),
                    class: "w-full bg-accent text-bg-deep font-semibold py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30",
                    if *submitting.read() { "registering…" } else { "request access →" }
                }
            }
            div { class: "mt-4 text-[12px] text-text-muted",
                "already have an account? "
                Link { class: "text-accent hover:opacity-80", to: Route::SignIn { next: None }, "sign in →" }
            }
        } }
    }
}
