//! Typesafe routes — equivalent to web/src/router.ts but compile-checked.
//!
//! Guard logic lives in components::guarded (RootLayout, AuthedShell,
//! RequireApproved, RequireAdmin). Each layout reads `use_auth()` and either
//! renders Outlet or replaces the current navigation target.

use dioxus::prelude::*;
use pastedev_core::SnippetType;

use crate::components::{AuthedShell, NotFoundPage, RequireAdmin, RequireApproved, RootLayout};
use crate::editor::EditorPage;
use crate::pages::{
    AdminPage, ApiKeysPage, DashboardPage, PendingPage, RegisterPage, RejectedPage, SetupPage,
    SignInPage, StatusPage,
};
use crate::views::{ViewCode, ViewHtml, ViewMarkdown};

#[rustfmt::skip]
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[layout(RootLayout)]
        #[route("/status")]
        Status {},

        #[route("/setup")]
        Setup {},

        #[route("/c/:slug")]
        ViewCode { slug: String },

        #[route("/m/:slug")]
        ViewMarkdown { slug: String },

        #[route("/h/:slug")]
        ViewHtml { slug: String },

        #[layout(AuthedShell)]
            #[route("/signin?:next")]
            SignIn { next: Option<String> },

            #[route("/register")]
            Register {},

            #[route("/pending")]
            Pending {},

            #[route("/rejected")]
            Rejected {},

            #[layout(RequireAdmin)]
                #[route("/admin")]
                Admin {},
            #[end_layout]

            #[layout(RequireApproved)]
                #[route("/dashboard")]
                Dashboard {},

                #[route("/keys")]
                ApiKeys {},

                #[route("/?:edit")]
                Editor { edit: Option<String> },
            #[end_layout]
        #[end_layout]

        // Catch-all stays inside RootLayout so it picks up the setup-gate.
        #[route("/:..rest")]
        NotFound { rest: Vec<String> },
}

impl Route {
    /// Convenience: produce the right Route variant for a snippet returned by the API.
    pub fn for_snippet_kind(kind: SnippetType, slug: &str) -> Route {
        match kind {
            SnippetType::Code => Route::ViewCode { slug: slug.into() },
            SnippetType::Markdown => Route::ViewMarkdown { slug: slug.into() },
            SnippetType::Html => Route::ViewHtml { slug: slug.into() },
        }
    }
}

// Component shims for routes that map directly into page components.
#[component]
pub fn Status() -> Element { rsx! { StatusPage {} } }
#[component]
pub fn Setup() -> Element { rsx! { SetupPage {} } }
#[component]
pub fn SignIn(next: Option<String>) -> Element { rsx! { SignInPage { next } } }
#[component]
pub fn Register() -> Element { rsx! { RegisterPage {} } }
#[component]
pub fn Pending() -> Element { rsx! { PendingPage {} } }
#[component]
pub fn Rejected() -> Element { rsx! { RejectedPage {} } }
#[component]
pub fn Admin() -> Element { rsx! { AdminPage {} } }
#[component]
pub fn Dashboard() -> Element { rsx! { DashboardPage {} } }
#[component]
pub fn ApiKeys() -> Element { rsx! { ApiKeysPage {} } }
#[component]
pub fn Editor(edit: Option<String>) -> Element { rsx! { EditorPage { edit } } }
#[component]
pub fn NotFound(rest: Vec<String>) -> Element {
    let _ = rest;
    rsx! { NotFoundPage {} }
}
