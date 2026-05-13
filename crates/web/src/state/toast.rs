//! Toast queue: a single signal of (id, kind, message); TTL via gloo_timers.

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

#[derive(Clone, Copy)]
pub struct ToastQueue {
    pub items: Signal<Vec<Toast>>,
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

    pub fn dismiss(mut self, id: u64) {
        self.items.write().retain(|t| t.id != id);
    }

    fn push(mut self, kind: ToastKind, message: String, ttl_ms: u32) {
        let id = next_id();
        self.items.write().push(Toast { id, kind, message });
        let mut items = self.items;
        gloo_timers::callback::Timeout::new(ttl_ms, move || {
            items.write().retain(|t| t.id != id);
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
    })
}

pub fn use_toast() -> ToastQueue {
    use_context()
}
