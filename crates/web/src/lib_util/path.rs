//! window.location.pathname + ?search — used for the ?next= round-trip on signin.

pub fn current_path() -> String {
    web_sys::window()
        .and_then(|w| {
            let p = w.location().pathname().ok()?;
            let s = w.location().search().ok()?;
            Some(format!("{p}{s}"))
        })
        .unwrap_or_else(|| "/".to_string())
}
