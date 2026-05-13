//! Page shell: 52-pixel top bar, optional mobile tab bar for approved users.

use dioxus::prelude::*;

use crate::route::Route;
use crate::state::{use_auth, use_toast};

#[component]
pub fn Shell(children: Element) -> Element {
    let auth = use_auth();
    let route = use_route::<Route>();

    rsx! {
        div { class: "min-h-screen flex flex-col",
            header { class: "border-b border-border",
                div { class: "max-w-6xl mx-auto px-4 md:px-7 h-[52px] flex items-center gap-3 md:gap-6 text-sm",
                    Link {
                        to: Route::Editor { edit: None },
                        class: "font-bold tracking-tight text-text",
                        "pastedev"
                    }
                    if auth.is_approved() {
                        nav { class: "hidden md:flex gap-6 ml-1",
                            NavLink {
                                to: Route::Editor { edit: None },
                                active: matches!(route, Route::Editor { .. }),
                                label: "new",
                            }
                            NavLink {
                                to: Route::Dashboard {},
                                active: matches!(route, Route::Dashboard {}),
                                label: "my snippets",
                            }
                            NavLink {
                                to: Route::ApiKeys {},
                                active: matches!(route, Route::ApiKeys {}),
                                label: "api keys",
                            }
                            if auth.is_admin() {
                                NavLink {
                                    to: Route::Admin {},
                                    active: matches!(route, Route::Admin {}),
                                    label: "admin",
                                }
                            }
                        }
                    }
                    div { class: "ml-auto flex items-center gap-2 md:gap-3 text-xs",
                        UserMenu {}
                    }
                }
            }
            main { class: "flex-1 pb-[68px] md:pb-0", {children} }
            if auth.is_approved() {
                MobileTabBar { route: route }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
pub struct NavLinkProps {
    pub to: Route,
    pub active: bool,
    pub label: String,
}

#[component]
pub fn NavLink(props: NavLinkProps) -> Element {
    let cls = if props.active {
        "pb-0.5 border-b border-accent text-text"
    } else {
        "pb-0.5 border-b border-transparent text-text-muted hover:text-text"
    };
    rsx! {
        Link { to: props.to, class: "{cls}", "{props.label}" }
    }
}

#[component]
fn UserMenu() -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let nav = use_navigator();
    let signed_in = auth.user.read().is_some();
    let username = auth
        .user
        .read()
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();

    let do_logout = move |_| {
        spawn(async move {
            match auth.logout().await {
                Ok(_) => {
                    nav.push(Route::SignIn { next: None });
                }
                Err(e) => toast.error(e.message().to_string()),
            }
        });
    };

    rsx! {
        if signed_in {
            span { class: "text-text-dim hidden md:inline", "{username}" }
            button {
                class: "text-text-muted hover:text-text",
                onclick: do_logout,
                "sign out"
            }
        } else {
            Link {
                to: Route::SignIn { next: None },
                class: "text-text-muted hover:text-text",
                "sign in"
            }
            Link {
                to: Route::Register {},
                class: "text-accent hover:opacity-80",
                "register"
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct MobileTabBarProps {
    route: Route,
}

#[component]
fn MobileTabBar(props: MobileTabBarProps) -> Element {
    let auth = use_auth();
    let cols = if auth.is_admin() { 4 } else { 3 };
    let grid = if cols == 4 {
        "grid grid-cols-4"
    } else {
        "grid grid-cols-3"
    };
    rsx! {
        nav { class: "fixed bottom-0 inset-x-0 md:hidden bg-bg-deep border-t border-border-strong z-30",
            style: "padding-bottom: env(safe-area-inset-bottom)",
            div { class: "{grid}",
                TabCell {
                    to: Route::Editor { edit: None },
                    active: matches!(props.route, Route::Editor { .. }),
                    label: "new",
                }
                TabCell {
                    to: Route::Dashboard {},
                    active: matches!(props.route, Route::Dashboard {}),
                    label: "snippets",
                }
                TabCell {
                    to: Route::ApiKeys {},
                    active: matches!(props.route, Route::ApiKeys {}),
                    label: "keys",
                }
                if auth.is_admin() {
                    TabCell {
                        to: Route::Admin {},
                        active: matches!(props.route, Route::Admin {}),
                        label: "admin",
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct TabCellProps {
    to: Route,
    active: bool,
    label: String,
}

#[component]
fn TabCell(props: TabCellProps) -> Element {
    let stripe = if props.active {
        "bg-accent"
    } else {
        "bg-transparent"
    };
    let text = if props.active { "text-text" } else { "text-text-muted" };
    rsx! {
        Link {
            to: props.to,
            class: "relative py-3 text-center text-[11px] {text}",
            span { class: "absolute top-0 left-1/2 -translate-x-1/2 h-0.5 w-6 {stripe}" }
            "{props.label}"
        }
    }
}

#[derive(Props, PartialEq, Clone)]
pub struct CenteredCardProps {
    pub width: Option<String>,
    pub children: Element,
}

#[component]
pub fn CenteredCard(props: CenteredCardProps) -> Element {
    let style = props
        .width
        .map(|w| format!("max-width: {w}"))
        .unwrap_or_default();
    rsx! {
        div { class: "flex justify-center pt-16 md:pt-32 px-4",
            div { class: "w-full", style: "{style}", {props.children} }
        }
    }
}
