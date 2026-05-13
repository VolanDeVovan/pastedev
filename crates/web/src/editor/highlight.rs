//! highlight.js binding for the editor overlay and the read-only view pages.
//!
//! Strategy: load `highlight.min.js` at the top level of `index.html` so
//! `window.hljs` is in scope, then call into it through wasm-bindgen. Both
//! the editor (debounced) and ViewCode (one-shot per snippet load) go through
//! this module. Worker-backed off-main-thread highlight (plans/06-editor.html
//! MVP-3) is a follow-up; for now everything runs on the main thread.

use pastedev_core::SnippetType;
use wasm_bindgen::prelude::*;

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
