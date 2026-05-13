//! Runtime config injected by the server as `<script id="pastedev-config">…</script>`.
//!
//! The contract is identical to web/src/config.ts: same DOM-id, same JSON keys.
//! Reading it on boot means the same WASM bundle works for same-origin and
//! split-origin deployments.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct PasteConfig {
    #[serde(rename = "apiBaseUrl", default)]
    pub api_base_url: String,
    #[serde(rename = "publicBaseUrl", default)]
    pub public_base_url: String,
    // Server emits `appName` in the same block; parsed for parity with the
    // wire contract even though no caller reads it yet.
    #[allow(dead_code)]
    #[serde(rename = "appName", default = "default_app_name")]
    pub app_name: String,
}

fn default_app_name() -> String {
    "pastedev".to_string()
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self {
            api_base_url: String::new(),
            public_base_url: String::new(),
            app_name: default_app_name(),
        }
    }
}

static CONFIG: OnceLock<PasteConfig> = OnceLock::new();

pub fn config() -> &'static PasteConfig {
    CONFIG.get_or_init(|| {
        let txt = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("pastedev-config"))
            .and_then(|el| el.text_content())
            .unwrap_or_default();
        if txt.trim().is_empty() {
            return PasteConfig::default();
        }
        serde_json::from_str(&txt).unwrap_or_default()
    })
}
