//! Tiny "5m ago" formatter — used by every view that shows a timestamp.

use time::OffsetDateTime;

pub fn ago(t: OffsetDateTime) -> String {
    let now_ms = js_sys::Date::now() as i128;
    let now = OffsetDateTime::from_unix_timestamp_nanos(now_ms * 1_000_000).unwrap_or(t);
    let diff = now - t;
    let secs = diff.whole_seconds();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else if secs < 86_400 * 30 {
        format!("{}d ago", secs / 86_400)
    } else if secs < 86_400 * 365 {
        format!("{}mo ago", secs / (86_400 * 30))
    } else {
        format!("{}y ago", secs / (86_400 * 365))
    }
}

