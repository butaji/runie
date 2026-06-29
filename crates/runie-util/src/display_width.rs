//! Display-cell width helpers for terminal layout.
//!
//! Part of `runie-util` crate.

pub use unicode_width::UnicodeWidthStr;

/// Display width of a string in terminal cells.
pub fn width(s: &str) -> u16 {
    UnicodeWidthStr::width(s) as u16
}

/// Split `s` so that the left part fits into `max_width` display cells without
/// breaking inside a wide character. Returns `(left, right)`.
pub fn split_at_width(s: &str, max_width: u16) -> (&str, &str) {
    let max = max_width as usize;
    let mut accumulated = 0usize;
    let mut split_idx = 0usize;
    for (idx, ch) in s.char_indices() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if accumulated + w > max {
            break;
        }
        accumulated += w;
        split_idx = idx + ch.len_utf8();
    }
    s.split_at(split_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_character_counts_as_two_cells() {
        assert_eq!(width("日"), 2);
        assert_eq!(width("日本語"), 6);
        assert_eq!(width("hello"), 5);
    }

    #[test]
    fn split_at_width_respects_wide_characters() {
        let (left, right) = split_at_width("日本語", 3);
        assert_eq!(width(left), 2);
        assert_eq!(left, "日");
        assert_eq!(right, "本語");
    }
}
