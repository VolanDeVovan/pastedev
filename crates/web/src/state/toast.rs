//! Toast queue: a single signal of (id, kind, message); TTL via gloo_timers.
//!
//! Each toast has a TTL (4 s info/success, 6 s error). Dismissal goes through
//! `begin_dismiss()` which flips the toast into "leaving" state — the dock
//! adds a `.toast-leave` class for the CSS slide-out, then 160 ms later the
//! actual removal happens. A direct `dismiss()` also exists for non-animated
//! cleanup (currently unused).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ToastKind {
    Info,
    Error,
    Success,
}

impl ToastKind {
    pub fn css(self) -> &'static str {
        match self {
            ToastKind::Info => "border-l-2 border-l-text-muted",
            ToastKind::Error => "border-l-2 border-l-danger",
            ToastKind::Success => "border-l-2 border-l-accent",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

// ms between flipping a toast into "leaving" state and physically removing
// it from the queue. Must match the .toast-leave animation in tailwind.css.
const LEAVE_MS: u32 = 160;

#[derive(Clone, Copy)]
pub struct ToastQueue {
    pub items: Signal<Vec<Toast>>,
    pub leaving: Signal<HashSet<u64>>,
}

impl ToastQueue {
    pub fn info(self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg.into(), 4000);
    }
    pub fn error(self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg.into(), 6000);
    }
    pub fn success(self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg.into(), 4000);
    }

    /// Mark a toast as leaving (adds .toast-leave for the CSS slide-out)
    /// and schedule its removal once the animation completes.
    pub fn dismiss(self, id: u64) {
        let mut leaving = self.leaving;
        if leaving.read().contains(&id) {
            return;
        }
        leaving.write().insert(id);
        let mut items = self.items;
        gloo_timers::callback::Timeout::new(LEAVE_MS, move || {
            items.write().retain(|t| t.id != id);
            leaving.write().remove(&id);
        })
        .forget();
    }

    fn push(mut self, kind: ToastKind, message: String, ttl_ms: u32) {
        let id = next_id();
        self.items.write().push(Toast { id, kind, message });
        let this = self;
        gloo_timers::callback::Timeout::new(ttl_ms, move || {
            this.dismiss(id);
        })
        .forget();
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn provide_toast() -> ToastQueue {
    use_context_provider(|| ToastQueue {
        items: Signal::new(Vec::new()),
        leaving: Signal::new(HashSet::new()),
    })
}

pub fn use_toast() -> ToastQueue {
    use_context()
}
