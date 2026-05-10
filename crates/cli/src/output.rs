use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}

pub fn print<T: Serialize, F: FnOnce()>(format: Format, value: &T, human: F) {
    match format {
        Format::Json => {
            let json = serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string());
            println!("{json}");
        }
        Format::Human => human(),
    }
}

/// Best-effort human-readable byte size: 412 b, 1.2 kb, 1.4 MB.
pub fn fmt_size(n: i32) -> String {
    let n = n as f64;
    if n < 1_024.0 {
        format!("{} b", n as i64)
    } else if n < 1_024.0 * 1_024.0 {
        format!("{:.1} kb", n / 1_024.0)
    } else {
        format!("{:.2} mb", n / (1_024.0 * 1_024.0))
    }
}

pub fn fmt_ago(at: time::OffsetDateTime) -> String {
    let now = time::OffsetDateTime::now_utc();
    let delta = now - at;
    let secs = delta.whole_seconds();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3_600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3_600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}
