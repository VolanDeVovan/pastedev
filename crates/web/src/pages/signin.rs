//! `/signin?:next` — username + password.

use dioxus::prelude::*;
use pastedev_core::LoginRequest;

use crate::components::{CenteredCard, FormField, Shell};
use crate::route::Route;
use crate::state::use_auth;

#[derive(Props, PartialEq, Clone)]
pub struct SignInPageProps {
    pub next: Option<String>,
}

#[component]
pub fn SignInPage(props: SignInPageProps) -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut submitting = use_signal(|| false);
    let next = props.next.clone();

    let submit = move |e: FormEvent| {
        e.prevent_default();
        let next = next.clone();
        spawn(async move {
            submitting.set(true);
            error.set(None);
            let req = LoginRequest {
                username: username.read().trim().to_string(),
                password: password.read().clone(),
            };
            match auth.login(req).await {
                Ok(_) => {
                    // Best-effort navigate to ?next= when provided.
                    if let Some(t) = next.as_ref() {
                        if t == "/" {
                            nav.replace(Route::Editor { edit: None });
                        } else if let Some(rest) = t.strip_prefix("/c/") {
                            nav.replace(Route::ViewCode { slug: rest.into() });
                        } else if let Some(rest) = t.strip_prefix("/m/") {
                            nav.replace(Route::ViewMarkdown { slug: rest.into() });
                        } else if let Some(rest) = t.strip_prefix("/h/") {
                            nav.replace(Route::ViewHtml { slug: rest.into() });
                        } else if t == "/dashboard" {
                            nav.replace(Route::Dashboard {});
                        } else if t == "/keys" {
                            nav.replace(Route::ApiKeys {});
                        } else if t == "/admin" {
                            nav.replace(Route::Admin {});
                        } else {
                            nav.replace(Route::Editor { edit: None });
                        }
                    } else {
                        nav.replace(Route::Editor { edit: None });
                    }
                }
                Err(e) => error.set(Some(e.message().into())),
            }
            submitting.set(false);
        });
    };

    rsx! {
        Shell { CenteredCard { width: Some("380px".to_string()),
            h1 { class: "text-[22px] tracking-tight mb-1.5", "sign in" }
            p { class: "text-[12px] text-text-muted leading-relaxed mb-7", "use your credentials to continue." }
            form { onsubmit: submit,
                FormField { label: "username".to_string(), value: username, autocomplete: Some("username".to_string()), required: true }
                FormField { label: "password".to_string(), value: password, kind: Some("password".to_string()), autocomplete: Some("current-password".to_string()), required: true }
                if let Some(e) = error.read().as_ref() {
                    div { class: "text-[12px] text-danger mb-3", "{e}" }
                }
                button {
                    r#type: "submit",
                    disabled: *submitting.read(),
                    class: "w-full bg-accent text-bg-deep font-semibold py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30",
                    if *submitting.read() { "signing in…" } else { "continue →" }
                }
            }
            div { class: "mt-4 text-[12px] text-text-muted",
                "no account? "
                Link { class: "text-accent hover:opacity-80", to: Route::Register {}, "register →" }
            }
        } }
    }
}
