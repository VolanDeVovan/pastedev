//! Guard layouts. Each renders either Outlet or schedules a redirect.
//!
//! RootLayout    : boot once, render splash until ready, gate /setup
//! AuthedShell   : signed-in shaping (bounce /signin, park pending/rejected)
//! RequireAdmin  : admin-only sub-tree
//! RequireApproved: approved-only sub-tree

use dioxus::prelude::*;
use pastedev_core::UserStatus;

use crate::components::SplashFallback;
use crate::lib_util::path::current_path;
use crate::route::Route;
use crate::state::use_auth;

#[component]
pub fn RootLayout() -> Element {
    let auth = use_auth();
    let route = use_route::<Route>();
    let nav = use_navigator();

    // Kick off boot exactly once per session. `last_fetch_at == None` is the
    // post-mount, pre-boot state.
    let mut booted = use_signal(|| false);
    use_effect(move || {
        if !*booted.read() {
            booted.set(true);
            spawn(async move { auth.boot().await; });
        }
    });

    if auth.last_fetch_at.read().is_none() {
        return rsx! { SplashFallback {} };
    }

    let in_setup = matches!(route, Route::Setup {});
    let in_status = matches!(route, Route::Status {});
    if auth.needs_setup() && !in_setup && !in_status {
        nav.replace(Route::Setup {});
        return rsx! { SplashFallback {} };
    }
    if !auth.needs_setup() && in_setup {
        nav.replace(Route::Editor { edit: None });
        return rsx! { SplashFallback {} };
    }

    rsx! { Outlet::<Route> {} }
}

#[component]
pub fn AuthedShell() -> Element {
    let auth = use_auth();
    let route = use_route::<Route>();
    let nav = use_navigator();

    // Signed-in users on /signin or /register go home.
    let signed_in = auth.user.read().is_some();
    if signed_in
        && matches!(route, Route::SignIn { .. } | Route::Register {})
    {
        nav.replace(Route::Editor { edit: None });
        return rsx! { SplashFallback {} };
    }

    // Park pending / rejected users on their landing page.
    let park: Option<Route> = match (auth.user.read().as_ref().map(|u| u.status), &route) {
        (Some(UserStatus::Pending), r) if !matches!(r, Route::Pending {}) => Some(Route::Pending {}),
        (Some(UserStatus::Rejected), r) if !matches!(r, Route::Rejected {}) => {
            Some(Route::Rejected {})
        }
        _ => None,
    };
    if let Some(target) = park {
        nav.replace(target);
        return rsx! { SplashFallback {} };
    }

    rsx! { Outlet::<Route> {} }
}

#[component]
pub fn RequireApproved() -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    let status = auth.user.read().as_ref().map(|u| u.status);
    match status {
        Some(UserStatus::Approved) => rsx! { Outlet::<Route> {} },
        Some(_) => {
            nav.replace(Route::Pending {});
            rsx! { SplashFallback {} }
        }
        None => {
            nav.replace(Route::SignIn {
                next: Some(current_path()),
            });
            rsx! { SplashFallback {} }
        }
    }
}

#[component]
pub fn RequireAdmin() -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    if auth.is_admin() {
        rsx! { Outlet::<Route> {} }
    } else {
        nav.replace(Route::Editor { edit: None });
        rsx! { SplashFallback {} }
    }
}

#[component]
pub fn NotFoundPage() -> Element {
    rsx! {
        crate::components::Shell {
            div {
                class: "max-w-2xl mx-auto px-4 py-16 text-text-dim text-sm",
                "this page doesn't exist."
            }
        }
    }
}
