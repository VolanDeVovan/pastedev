//! Thin wasm-bindgen wrappers for the spots that need raw JS.

use wasm_bindgen_futures::JsFuture;

/// Best-effort clipboard write. Returns Ok if the promise resolves.
pub async fn copy_text(text: &str) -> Result<(), String> {
    let win = web_sys::window().ok_or("no window")?;
    let clipboard = win.navigator().clipboard();
    let promise = clipboard.write_text(text);
    JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|e| format!("clipboard: {e:?}"))
}

/// Reads `navigator.userAgent`. Used by the install pane to default OS toggle.
pub fn user_agent() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
}
