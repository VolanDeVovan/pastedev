//! `/admin` — pending/all tabs; per-user actions w/ confirm modals.

use dioxus::prelude::*;
use pastedev_core::{AdminUserView, Role, UserStatus};

use crate::api;
use crate::components::{Modal, Shell};
use crate::lib_util::time::ago;
use crate::state::use_toast;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Tab {
    Pending,
    All,
}

#[component]
pub fn AdminPage() -> Element {
    let toast = use_toast();
    let mut tab = use_signal(|| Tab::Pending);
    let mut users = use_signal::<Vec<AdminUserView>>(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal::<Option<String>>(|| None);

    let refresh = move || {
        let want = *tab.read();
        spawn(async move {
            loading.set(true);
            let st = match want {
                Tab::Pending => Some(UserStatus::Pending),
                Tab::All => None,
            };
            match api::admin::list(st).await {
                Ok(r) => users.set(r.items),
                Err(e) => error.set(Some(e.message().into())),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        let _ = *tab.read();
        refresh();
    });

    let mut approve_target = use_signal::<Option<AdminUserView>>(|| None);
    let mut reject_target = use_signal::<Option<AdminUserView>>(|| None);

    let do_approve = move |_| {
        let Some(u) = approve_target.read().clone() else { return; };
        spawn(async move {
            match api::admin::approve(u.id).await {
                Ok(_) => {
                    toast.success(format!("approved {}", u.username));
                    refresh();
                }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };
    let do_reject = move |_| {
        let Some(u) = reject_target.read().clone() else { return; };
        spawn(async move {
            match api::admin::reject(u.id).await {
                Ok(_) => {
                    toast.success(format!("rejected {}", u.username));
                    refresh();
                }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };

    let do_suspend = move |id| {
        spawn(async move {
            match api::admin::suspend(id).await {
                Ok(_) => { toast.info("suspended"); refresh(); }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };
    let do_restore = move |id| {
        spawn(async move {
            match api::admin::restore(id).await {
                Ok(_) => { toast.info("restored"); refresh(); }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };
    let do_promote = move |id| {
        spawn(async move {
            match api::admin::promote(id).await {
                Ok(_) => { toast.info("promoted"); refresh(); }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };
    let do_demote = move |id| {
        spawn(async move {
            match api::admin::demote(id).await {
                Ok(_) => { toast.info("demoted"); refresh(); }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };

    let approve_open = use_memo(move || approve_target.read().is_some());
    let reject_open = use_memo(move || reject_target.read().is_some());

    let approve_modal_open = use_signal(move || approve_open());
    let reject_modal_open = use_signal(move || reject_open());

    let approve_name = approve_target.read().as_ref().map(|u| u.username.clone()).unwrap_or_default();
    let reject_name = reject_target.read().as_ref().map(|u| u.username.clone()).unwrap_or_default();

    rsx! {
        Shell {
            div { class: "max-w-6xl mx-auto px-4 md:px-7 pt-6 pb-10",
                div { class: "flex items-baseline justify-between flex-wrap gap-2 mb-1",
                    h1 { class: "text-[22px] tracking-tight", "admin queue" }
                    button {
                        class: "text-[12px] text-text-muted hover:text-text",
                        onclick: move |_| refresh(),
                        if *loading.read() { "loading…" } else { "refresh" }
                    }
                }
                p { class: "text-[12px] text-text-muted mb-5",
                    "review and manage user accounts."
                }
                div { class: "flex gap-1.5 mb-4",
                    TabBtn { active: *tab.read() == Tab::Pending, label: "pending", on_click: move |_| tab.set(Tab::Pending) }
                    TabBtn { active: *tab.read() == Tab::All,     label: "all",     on_click: move |_| tab.set(Tab::All) }
                }
                if let Some(e) = error.read().as_ref() {
                    div { class: "text-[12px] text-danger mb-3", "{e}" }
                }
                div { class: "border border-border rounded-sm divide-y divide-border bg-bg-deep",
                    if users.read().is_empty() && !*loading.read() {
                        div { class: "text-[12px] text-text-muted py-8 text-center", "nothing to review." }
                    } else {
                        for u in users.read().iter() {
                            UserRow {
                                user: u.clone(),
                                on_approve: {
                                    let u = u.clone();
                                    EventHandler::new(move |_| approve_target.set(Some(u.clone())))
                                },
                                on_reject: {
                                    let u = u.clone();
                                    EventHandler::new(move |_| reject_target.set(Some(u.clone())))
                                },
                                on_suspend: EventHandler::new(move |id| do_suspend(id)),
                                on_restore: EventHandler::new(move |id| do_restore(id)),
                                on_promote: EventHandler::new(move |id| do_promote(id)),
                                on_demote: EventHandler::new(move |id| do_demote(id)),
                            }
                        }
                    }
                }
            }
        }
        Modal {
            open: approve_modal_open,
            title: Some(format!("approve {approve_name}")),
            confirm_label: Some("approve".to_string()),
            on_confirm: Some(EventHandler::new(do_approve)),
            "this lets the user create snippets immediately."
        }
        Modal {
            open: reject_modal_open,
            title: Some(format!("reject {reject_name}")),
            danger: true,
            confirm_label: Some("reject".to_string()),
            on_confirm: Some(EventHandler::new(do_reject)),
            "the user will see a 'request declined' page. their sessions are revoked."
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct TabBtnProps {
    active: bool,
    label: String,
    on_click: EventHandler<()>,
}

#[component]
fn TabBtn(props: TabBtnProps) -> Element {
    let cls = if props.active {
        "bg-accent text-bg-deep font-semibold px-2.5 py-1 text-[11px] rounded-sm"
    } else {
        "text-text-muted hover:text-text px-2.5 py-1 text-[11px] rounded-sm border border-border-strong"
    };
    rsx! {
        button { class: "{cls}", onclick: move |_| props.on_click.call(()), "{props.label}" }
    }
}

const AVATAR_PALETTE: &[&str] = &[
    "bg-accent/20 text-accent",
    "bg-warn/20 text-warn",
    "bg-blue-400/20 text-blue-300",
    "bg-rose-400/20 text-rose-300",
    "bg-purple-400/20 text-purple-300",
    "bg-emerald-400/20 text-emerald-300",
];

fn avatar_class(name: &str) -> &'static str {
    let h: u32 = name
        .bytes()
        .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    AVATAR_PALETTE[(h as usize) % AVATAR_PALETTE.len()]
}

#[derive(Props, PartialEq, Clone)]
struct UserRowProps {
    user: AdminUserView,
    on_approve: EventHandler<()>,
    on_reject: EventHandler<()>,
    on_suspend: EventHandler<uuid::Uuid>,
    on_restore: EventHandler<uuid::Uuid>,
    on_promote: EventHandler<uuid::Uuid>,
    on_demote: EventHandler<uuid::Uuid>,
}

#[component]
fn UserRow(props: UserRowProps) -> Element {
    let u = props.user.clone();
    let first = u.username.chars().next().unwrap_or('?').to_uppercase().to_string();
    let av = avatar_class(&u.username);
    let id = u.id;
    let status_cls = match u.status {
        UserStatus::Pending   => "text-warn",
        UserStatus::Approved  => "text-accent",
        UserStatus::Rejected  => "text-danger",
        UserStatus::Suspended => "text-text-muted",
    };
    rsx! {
        div { class: "px-3 py-3",
            div { class: "flex items-start gap-3",
                span { class: "{av} flex items-center justify-center w-8 h-8 text-[12px] font-bold rounded-sm", "{first}" }
                div { class: "flex-1 min-w-0",
                    div { class: "flex flex-wrap items-baseline gap-2 text-[12px]",
                        span { class: "text-text", "{u.username}" }
                        if let Some(e) = u.email.as_ref() { span { class: "text-text-muted text-[11px]", "{e}" } }
                        span { class: "{status_cls} text-[11px] uppercase tracking-wider", "{u.status.as_str()}" }
                        if u.role == Role::Admin {
                            span { class: "text-accent text-[11px] uppercase tracking-wider", "admin" }
                        }
                        span { class: "text-text-faint text-[11px] ml-auto", "{ago(u.created_at)}" }
                    }
                    if let Some(r) = u.reason.as_ref() {
                        div { class: "text-[12px] text-text-dim mt-1.5", "“{r}”" }
                    }
                    if let Some(ip) = u.registration_ip.as_ref() {
                        div { class: "text-[11px] text-text-faint mt-1 font-mono", "ip {ip}" }
                    }
                    div { class: "flex flex-wrap gap-1.5 mt-2",
                        match u.status {
                            UserStatus::Pending => rsx! {
                                ActionBtn { kind: "ok", label: "approve", on_click: move |_| props.on_approve.call(()) }
                                ActionBtn { kind: "danger", label: "reject", on_click: move |_| props.on_reject.call(()) }
                            },
                            UserStatus::Approved => rsx! {
                                ActionBtn { kind: "neutral", label: "suspend", on_click: move |_| props.on_suspend.call(id) }
                                if u.role == Role::User {
                                    ActionBtn { kind: "neutral", label: "promote", on_click: move |_| props.on_promote.call(id) }
                                } else {
                                    ActionBtn { kind: "neutral", label: "demote", on_click: move |_| props.on_demote.call(id) }
                                }
                            },
                            UserStatus::Suspended | UserStatus::Rejected => rsx! {
                                ActionBtn { kind: "neutral", label: "restore", on_click: move |_| props.on_restore.call(id) }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ActionBtnProps {
    kind: String,   // "ok" | "danger" | "neutral"
    label: String,
    on_click: EventHandler<()>,
}

#[component]
fn ActionBtn(props: ActionBtnProps) -> Element {
    let cls = match props.kind.as_str() {
        "ok"     => "text-[11px] bg-accent/20 text-accent border border-accent/40 hover:bg-accent/30 px-2.5 py-1 rounded-sm",
        "danger" => "text-[11px] bg-danger/20 text-danger border border-danger-border hover:bg-danger/30 px-2.5 py-1 rounded-sm",
        _        => "text-[11px] text-text-muted border border-border-strong hover:text-text px-2.5 py-1 rounded-sm",
    };
    rsx! {
        button { class: "{cls}", onclick: move |_| props.on_click.call(()), "{props.label}" }
    }
}
