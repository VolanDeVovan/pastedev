//! `/keys` — create / list / revoke; CLI install pane.

use dioxus::prelude::*;
use pastedev_core::{CreateKeyRequest, KeyMintedView, KeyView, Scope};

use crate::api;
use crate::components::{Modal, Shell};
use crate::config::config;
use crate::js::{copy_text, user_agent};
use crate::lib_util::time::ago;
use crate::state::use_toast;

#[component]
pub fn ApiKeysPage() -> Element {
    let toast = use_toast();
    let mut keys = use_signal::<Vec<KeyView>>(Vec::new);
    let mut minted = use_signal::<Option<KeyMintedView>>(|| None);
    let mut error = use_signal::<Option<String>>(|| None);
    let mut name = use_signal(String::new);
    let mut scope_publish = use_signal(|| true);
    let mut scope_read = use_signal(|| false);
    let mut scope_delete = use_signal(|| false);
    let mut submitting = use_signal(|| false);

    let refresh = move || {
        spawn(async move {
            match api::keys::list().await {
                Ok(r) => keys.set(r.items),
                Err(e) => error.set(Some(e.message().into())),
            }
        });
    };

    use_effect(move || refresh());

    let do_create = move |e: FormEvent| {
        e.prevent_default();
        let n = name.read().trim().to_string();
        if n.is_empty() {
            error.set(Some("name is required".to_string()));
            return;
        }
        let mut scopes = Vec::new();
        if *scope_publish.read() { scopes.push(Scope::Publish); }
        if *scope_read.read()    { scopes.push(Scope::Read); }
        if *scope_delete.read()  { scopes.push(Scope::Delete); }
        spawn(async move {
            submitting.set(true);
            error.set(None);
            match api::keys::create(&CreateKeyRequest { name: n, scopes }).await {
                Ok(m) => {
                    minted.set(Some(m));
                    name.set(String::new());
                    refresh();
                }
                Err(e) => error.set(Some(e.message().into())),
            }
            submitting.set(false);
        });
    };

    let mut revoke_target = use_signal::<Option<KeyView>>(|| None);
    let revoke_open = use_signal(move || revoke_target.read().is_some());
    let revoke_name = revoke_target.read().as_ref().map(|k| k.name.clone()).unwrap_or_default();

    let do_revoke = move |_| {
        let Some(k) = revoke_target.read().clone() else { return; };
        spawn(async move {
            match api::keys::revoke(k.id).await {
                Ok(_) => { toast.success(format!("revoked {}", k.name)); refresh(); }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };

    let is_windows = user_agent().to_lowercase().contains("windows");
    let mut os_tab = use_signal(|| if is_windows { "windows" } else { "unix" });

    let public_url = if config().public_base_url.is_empty() {
        "https://your-instance".to_string()
    } else {
        config().public_base_url.clone()
    };

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-6 pb-10 grid grid-cols-1 md:grid-cols-2 gap-6",
                // Left: create + list
                div {
                    h1 { class: "text-[22px] tracking-tight mb-1.5", "api keys" }
                    p { class: "text-[12px] text-text-muted mb-6",
                        "personal access tokens for the CLI / MCP. shown once on create."
                    }
                    form { onsubmit: do_create, class: "border border-border-strong rounded-sm p-3 bg-bg-deep mb-5",
                        div { class: "mb-3",
                            label { class: "text-[11px] text-text-muted block mb-1.5", "key name" }
                            input {
                                class: "w-full bg-bg-deep border border-border-strong rounded-sm px-3 py-2 text-[13px] text-text",
                                value: "{name}",
                                placeholder: "laptop",
                                oninput: move |e| name.set(e.value()),
                            }
                        }
                        div { class: "mb-3",
                            div { class: "text-[11px] text-text-muted mb-1.5", "scopes" }
                            div { class: "flex flex-wrap gap-3 text-[12px]",
                                ScopeBox { value: scope_publish, label: "publish" }
                                ScopeBox { value: scope_read, label: "read" }
                                ScopeBox { value: scope_delete, label: "delete" }
                            }
                        }
                        button {
                            r#type: "submit",
                            disabled: *submitting.read(),
                            class: "bg-accent text-bg-deep font-semibold px-3 py-1.5 text-[12px] rounded-sm hover:opacity-90 disabled:opacity-30",
                            if *submitting.read() { "creating…" } else { "create →" }
                        }
                    }

                    if let Some(m) = minted.read().clone() {
                        div { class: "border border-accent/40 bg-accent/10 rounded-sm p-3 mb-5",
                            div { class: "text-[11px] text-accent mb-1", "key created — copy now, it won't be shown again" }
                            div { class: "flex items-center gap-2",
                                code { class: "flex-1 font-mono text-[12px] text-text break-all", "{m.token}" }
                                CopyButton { text: m.token.clone() }
                                button {
                                    class: "text-[11px] text-text-muted hover:text-text",
                                    onclick: move |_| minted.set(None),
                                    "dismiss"
                                }
                            }
                        }
                    }
                    if let Some(e) = error.read().as_ref() {
                        div { class: "text-[12px] text-danger mb-3", "{e}" }
                    }

                    div { class: "border border-border rounded-sm divide-y divide-border bg-bg-deep",
                        if keys.read().is_empty() {
                            div { class: "text-[12px] text-text-muted py-6 text-center", "no keys yet." }
                        } else {
                            for k in keys.read().iter() {
                                KeyRow {
                                    item: k.clone(),
                                    on_revoke: {
                                        let k = k.clone();
                                        EventHandler::new(move |_| revoke_target.set(Some(k.clone())))
                                    },
                                }
                            }
                        }
                    }
                }

                // Right: install pane
                div { class: "text-[12px]",
                    div { class: "flex items-baseline justify-between mb-2",
                        h2 { class: "text-[16px]", "install" }
                        div { class: "flex gap-1.5",
                            OsTab { active: *os_tab.read() == "unix", label: "macOS · Linux", on_click: move |_| os_tab.set("unix") }
                            OsTab { active: *os_tab.read() == "windows", label: "Windows", on_click: move |_| os_tab.set("windows") }
                        }
                    }
                    InstallBlock { os: os_tab, instance_url: public_url.clone() }
                }
            }
        }
        Modal {
            open: revoke_open,
            title: Some(format!("revoke {revoke_name}")),
            danger: true,
            confirm_label: Some("revoke".to_string()),
            on_confirm: Some(EventHandler::new(do_revoke)),
            "any CLI / MCP using this key will be cut off immediately."
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ScopeBoxProps {
    value: Signal<bool>,
    label: String,
}

#[component]
fn ScopeBox(props: ScopeBoxProps) -> Element {
    let mut v = props.value;
    rsx! {
        label { class: "flex items-center gap-1.5 cursor-pointer",
            input {
                r#type: "checkbox",
                checked: *v.read(),
                onchange: move |e| v.set(e.checked()),
            }
            span { class: "text-text-dim", "{props.label}" }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct CopyButtonProps {
    text: String,
}

#[component]
fn CopyButton(props: CopyButtonProps) -> Element {
    let toast = use_toast();
    let text = props.text.clone();
    rsx! {
        button {
            class: "text-[11px] text-accent hover:opacity-80",
            onclick: move |_| {
                let text = text.clone();
                spawn(async move {
                    match copy_text(&text).await {
                        Ok(_) => toast.success("copied"),
                        Err(e) => toast.error(e),
                    }
                });
            },
            "copy"
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct KeyRowProps {
    item: KeyView,
    on_revoke: EventHandler<()>,
}

#[component]
fn KeyRow(props: KeyRowProps) -> Element {
    let k = props.item.clone();
    let revoked = k.revoked_at.is_some();
    let scopes = k
        .scopes
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let last = k.last_used_at.map(ago).unwrap_or_else(|| "never used".to_string());
    rsx! {
        div { class: "px-3 py-2 flex flex-wrap items-baseline gap-3 text-[12px]",
            code { class: "font-mono text-text", "pds_live_{k.prefix}··········" }
            span { class: "text-text-muted text-[11px]", "{k.name}" }
            span { class: "text-text-faint text-[11px]", "{scopes}" }
            span { class: "ml-auto text-text-faint text-[11px]",
                "created {ago(k.created_at)} · {last}"
            }
            if revoked {
                span { class: "text-danger text-[11px]", "revoked" }
            } else {
                button {
                    class: "text-danger hover:opacity-80 text-[11px]",
                    onclick: move |_| props.on_revoke.call(()),
                    "revoke"
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct OsTabProps {
    active: bool,
    label: String,
    on_click: EventHandler<()>,
}

#[component]
fn OsTab(props: OsTabProps) -> Element {
    let cls = if props.active {
        "bg-accent text-bg-deep font-semibold px-2.5 py-1 text-[11px] rounded-sm"
    } else {
        "text-text-muted hover:text-text px-2.5 py-1 text-[11px] rounded-sm border border-border-strong"
    };
    rsx! {
        button { class: "{cls}", onclick: move |_| props.on_click.call(()), "{props.label}" }
    }
}

#[derive(Props, PartialEq, Clone)]
struct InstallBlockProps {
    os: Signal<&'static str>,
    instance_url: String,
}

#[component]
fn InstallBlock(props: InstallBlockProps) -> Element {
    let url = props.instance_url.clone();
    let url2 = url.clone();
    let install = if *props.os.read() == "windows" {
        "irm https://pastedev.io/install.ps1 | iex".to_string()
    } else {
        "curl -fsSL https://pastedev.io/install.sh | sh".to_string()
    };
    let auth_cmd = format!("pastedev auth login --instance {url} --token pds_live_***");
    let publish_cmd = format!("cat README.md | pastedev publish -t md");
    let curl_cmd = format!(
        "curl -X POST {url2}/paste -H 'authorization: Bearer pds_live_***' \\\n  --data-binary @file.txt"
    );

    rsx! {
        Block { label: "install".to_string(), code: install }
        Block { label: "authenticate".to_string(), code: auth_cmd }
        Block { label: "publish".to_string(), code: publish_cmd }
        Block { label: "curl".to_string(), code: curl_cmd }
        Block {
            label: "mcp (claude code)".to_string(),
            code: "claude mcp add pastedev pastedev mcp".to_string(),
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct BlockProps {
    label: String,
    code: String,
}

#[component]
fn Block(props: BlockProps) -> Element {
    rsx! {
        div { class: "mb-3",
            div { class: "flex items-baseline justify-between mb-1.5",
                span { class: "text-[11px] text-text-muted", "{props.label}" }
                CopyButton { text: props.code.clone() }
            }
            pre { class: "bg-bg-deep border border-border rounded-sm px-3 py-2 text-[11px] font-mono text-text overflow-x-auto whitespace-pre-wrap break-all",
                "{props.code}"
            }
        }
    }
}
