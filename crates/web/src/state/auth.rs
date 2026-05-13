//! Auth context: signals for user / setup state + async actions.

use dioxus::prelude::*;
use pastedev_core::{
    LoginRequest, RegisterRequest, Role, SetupAdminRequest, SetupStatus, UserPublic, UserStatus,
};

use crate::api;
use crate::api::HttpError;

#[derive(Clone, Copy)]
pub struct AuthState {
    pub user: Signal<Option<UserPublic>>,
    pub setup: Signal<Option<SetupStatus>>,
    pub loading: Signal<bool>,
    pub last_fetch_at: Signal<Option<f64>>,
}

impl AuthState {
    pub fn needs_setup(&self) -> bool {
        self.setup
            .read()
            .as_ref()
            .is_some_and(|s| s.needs_setup)
    }

    pub fn is_admin(&self) -> bool {
        self.user
            .read()
            .as_ref()
            .is_some_and(|u| u.role == Role::Admin)
    }

    pub fn is_approved(&self) -> bool {
        self.user
            .read()
            .as_ref()
            .is_some_and(|u| u.status == UserStatus::Approved)
    }

    pub async fn boot(mut self) {
        self.loading.set(true);
        let (s, u) = futures::join!(api::auth::setup_status(), api::auth::me());
        if let Ok(s) = s {
            self.setup.set(Some(s));
        }
        self.user.set(u.ok());
        self.last_fetch_at.set(Some(now_ms()));
        self.loading.set(false);
    }

    pub async fn refresh_me(mut self) {
        if let Ok(u) = api::auth::me().await {
            self.user.set(Some(u));
        } else {
            self.user.set(None);
        }
    }

    pub async fn login(mut self, input: LoginRequest) -> Result<(), HttpError> {
        let u = api::auth::login(&input).await?;
        self.user.set(Some(u));
        Ok(())
    }

    pub async fn logout(mut self) -> Result<(), HttpError> {
        api::auth::logout().await?;
        self.user.set(None);
        Ok(())
    }

    pub async fn register(mut self, input: RegisterRequest) -> Result<(), HttpError> {
        let u = api::auth::register(&input).await?;
        self.user.set(Some(u));
        Ok(())
    }

    pub async fn setup_admin(mut self, input: SetupAdminRequest) -> Result<(), HttpError> {
        let r = api::auth::create_first_admin(&input).await?;
        self.user.set(Some(r.user));
        // Once the first admin exists the gate flips permanently.
        if let Some(s) = self.setup.write().as_mut() {
            s.needs_setup = false;
        }
        Ok(())
    }
}

pub fn provide_auth() -> AuthState {
    use_context_provider(|| AuthState {
        user: Signal::new(None),
        setup: Signal::new(None),
        loading: Signal::new(false),
        last_fetch_at: Signal::new(None),
    })
}

pub fn use_auth() -> AuthState {
    use_context()
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}
