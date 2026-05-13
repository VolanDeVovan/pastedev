//! highlight.js binding for the editor overlay and the read-only view pages.
//!
//! Two entry points:
//! - `request_async(body, kind)` — off-main-thread via a shared Web Worker.
//!   This is what the editor + ViewCode use; the worker keeps the UI smooth
//!   on big pastes where `hljs.highlightAuto` would otherwise stall the
//!   main thread for tens to hundreds of ms.
//! - `highlight(body, kind)` / `highlight_with_lang(body, lang)` — synchronous
//!   main-thread fallback used by markdown's fenced-code blocks. The fences
//!   inside a snippet are small and run once at render time.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use futures::channel::oneshot;
use pastedev_core::SnippetType;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, Worker};

use crate::lib_util::format::escape_html;

#[wasm_bindgen]
extern "C" {
    type Hljs;
    #[wasm_bindgen(thread_local_v2, js_namespace = window, js_name = hljs)]
    static HLJS: JsValue;

    #[wasm_bindgen(method, js_name = highlight, structural)]
    fn highlight(this: &Hljs, body: &str, opts: &JsValue) -> JsValue;

    #[wasm_bindgen(method, js_name = highlightAuto, structural)]
    fn highlight_auto(this: &Hljs, body: &str) -> JsValue;

    #[wasm_bindgen(method, js_name = getLanguage, structural)]
    fn get_language(this: &Hljs, name: &str) -> JsValue;
}

/// Highlight a body with an explicit language hint (used by markdown's
/// fenced code blocks). Falls back to auto-detect when the language is
/// unknown to hljs.
pub fn highlight_with_lang(body: &str, lang_name: &str) -> HighlightResult {
    if body.is_empty() {
        return HighlightResult {
            html: String::new(),
            language: None,
            truncated: false,
        };
    }
    let hljs = HLJS.with(JsValue::clone);
    if hljs.is_undefined() || hljs.is_null() {
        return HighlightResult {
            html: escape_html(body),
            language: None,
            truncated: false,
        };
    }
    let hljs: Hljs = hljs.unchecked_into();
    let lang = hljs.get_language(lang_name);
    if lang.is_undefined() || lang.is_null() {
        // Unknown language — fall back to plain auto-detect via the public path.
        return highlight(body, SnippetType::Code);
    }
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"language".into(), &lang_name.into());
    let _ = js_sys::Reflect::set(&opts, &"ignoreIllegals".into(), &true.into());
    let result = hljs.highlight(body, &opts);
    extract(result, lang_name)
}

/// Bytes after which we stop auto-detecting and either bail or sample.
const DETECT_SAMPLE_BYTES: usize = 16 * 1024;
/// Hard cap; over this size we ship escaped-only HTML.
const HARD_LIMIT: usize = 4 * 1024 * 1024;

pub struct HighlightResult {
    pub html: String,
    pub language: Option<String>,
    pub truncated: bool,
}

/// Highlight `body`. `kind` provides a per-snippet-type hint:
/// - Html → always paint as language=html
/// - Code → auto-detect (sample-then-render for large bodies)
///
/// Falls back to plain escaped HTML on any error or when hljs hasn't loaded.
pub fn highlight(body: &str, kind: SnippetType) -> HighlightResult {
    if body.is_empty() {
        return HighlightResult {
            html: String::new(),
            language: None,
            truncated: false,
        };
    }
    if body.len() > HARD_LIMIT {
        return HighlightResult {
            html: escape_html(body),
            language: None,
            truncated: true,
        };
    }

    let hljs = HLJS.with(JsValue::clone);
    if hljs.is_undefined() || hljs.is_null() {
        // Script hasn't loaded yet — paint escaped plain text. The editor
        // re-runs on the next keystroke so this self-heals.
        return HighlightResult {
            html: escape_html(body),
            language: None,
            truncated: false,
        };
    }
    let hljs: Hljs = hljs.unchecked_into();

    let hint = match kind {
        SnippetType::Html => Some("xml"),
        SnippetType::Code => None,
        SnippetType::Markdown => Some("markdown"),
    };

    // Path 1: explicit hint, validate via getLanguage so non-built-in names
    // don't blow up the call.
    if let Some(name) = hint {
        let lang = hljs.get_language(name);
        if !lang.is_undefined() && !lang.is_null() {
            let opts = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&opts, &"language".into(), &name.into());
            let _ = js_sys::Reflect::set(&opts, &"ignoreIllegals".into(), &true.into());
            let result = hljs.highlight(body, &opts);
            return extract(result, name);
        }
    }

    // Path 2: small body — auto-detect over the whole thing.
    if body.len() <= DETECT_SAMPLE_BYTES {
        let result = hljs.highlight_auto(body);
        return extract(result, "");
    }

    // Path 3: large body — sample-detect, then explicit-language render.
    let sample = &body[..DETECT_SAMPLE_BYTES];
    let detected = hljs.highlight_auto(sample);
    let detected_lang = js_sys::Reflect::get(&detected, &"language".into())
        .ok()
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty());
    let Some(lang) = detected_lang else {
        return HighlightResult {
            html: escape_html(body),
            language: None,
            truncated: false,
        };
    };
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"language".into(), &lang.clone().into());
    let _ = js_sys::Reflect::set(&opts, &"ignoreIllegals".into(), &true.into());
    let result = hljs.highlight(body, &opts);
    extract(result, &lang)
}

/// Pulls `{ value, language }` out of the JS object hljs.highlight returns.
fn extract(result: JsValue, fallback_lang: &str) -> HighlightResult {
    let html = js_sys::Reflect::get(&result, &"value".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let language = js_sys::Reflect::get(&result, &"language".into())
        .ok()
        .and_then(|v| v.as_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            if fallback_lang.is_empty() {
                None
            } else {
                Some(fallback_lang.to_string())
            }
        });
    HighlightResult {
        html,
        language,
        truncated: false,
    }
}

// ─── Worker-backed async highlight ────────────────────────────────────────
//
// One shared Worker instance, plus a single onmessage listener that
// dispatches replies into a per-request registry keyed by id. Each
// `request_async` call gets a fresh id and a oneshot channel; the worker's
// reply resolves the channel.

const WORKER_PATH: &str = "/assets/highlight.worker.js";

type Sender = oneshot::Sender<HighlightResult>;

thread_local! {
    static PENDING: RefCell<HashMap<u64, Sender>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn worker() -> Option<&'static Worker> {
    // OnceLock<Worker> is !Send/!Sync but we're single-threaded on wasm32.
    // SAFETY: WASM has no threads; OnceLock<Worker> behaves as a deferred
    // initializer here.
    static W: OnceLock<SyncWrap> = OnceLock::new();
    struct SyncWrap(Worker);
    // Worker isn't Send/Sync but on wasm32 single-threaded that's a
    // technicality; OnceLock just needs the value type to compile.
    unsafe impl Send for SyncWrap {}
    unsafe impl Sync for SyncWrap {}

    let wrap = W.get_or_init(|| {
        let w = Worker::new(WORKER_PATH).expect("highlight worker");
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(|e: MessageEvent| {
            let data = e.data();
            let id = js_sys::Reflect::get(&data, &"id".into())
                .ok()
                .and_then(|v| v.as_f64())
                .map(|f| f as u64);
            let html = js_sys::Reflect::get(&data, &"html".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let language = js_sys::Reflect::get(&data, &"language".into())
                .ok()
                .and_then(|v| v.as_string())
                .filter(|s| !s.is_empty());
            let truncated = js_sys::Reflect::get(&data, &"truncated".into())
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let Some(id) = id else { return; };
            let pending = PENDING.with(|p| p.borrow_mut().remove(&id));
            if let Some(tx) = pending {
                let _ = tx.send(HighlightResult {
                    html,
                    language,
                    truncated,
                });
            }
        });
        w.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
        SyncWrap(w)
    });
    Some(&wrap.0)
}

/// Post `body` to the worker and return a Future that resolves with the
/// reply. Always succeeds — on a worker error or unknown language the worker
/// echoes escaped HTML.
///
/// The `kind` SnippetType becomes a hint:
/// - Html → `xml`
/// - Markdown → `markdown`
/// - Code → no hint, the worker auto-detects
pub async fn request_async(body: &str, kind: SnippetType) -> HighlightResult {
    let Some(w) = worker() else {
        // Worker construction failed — fall back to sync.
        return highlight(body, kind);
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = oneshot::channel();
    PENDING.with(|p| p.borrow_mut().insert(id, tx));

    let msg = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&msg, &"id".into(), &JsValue::from_f64(id as f64));
    let _ = js_sys::Reflect::set(&msg, &"body".into(), &body.into());
    let hint = match kind {
        SnippetType::Html => Some("xml"),
        SnippetType::Markdown => Some("markdown"),
        SnippetType::Code => None,
    };
    if let Some(h) = hint {
        let _ = js_sys::Reflect::set(&msg, &"hint".into(), &h.into());
    }

    if w.post_message(&msg).is_err() {
        PENDING.with(|p| p.borrow_mut().remove(&id));
        return highlight(body, kind);
    }

    match rx.await {
        Ok(r) => r,
        Err(_) => HighlightResult {
            html: escape_html(body),
            language: None,
            truncated: false,
        },
    }
}
