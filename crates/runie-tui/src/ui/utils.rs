//! Shared UI utilities.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Truncate `text` so its display width is at most `max_width`, appending an
/// ellipsis only when truncation actually occurs.
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut w = 0usize;
    // Reserve one cell for the ellipsis.
    let limit = max_width.saturating_sub(1);
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if w + ch_width > limit {
            out.push('…');
            break;
        }
        out.push(ch);
        w += ch_width;
    }
    out
}
