//! "12.4 kB" / "1.2 MB" — shared between editor + dashboard + view pages.

pub fn size(bytes: i64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{bytes} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.1} kB", b / 1024.0)
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

/// HTML-escape a body for safe innerHTML rendering.
pub fn escape_html(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for c in src.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
