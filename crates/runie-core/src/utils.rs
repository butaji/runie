//! Small shared string/list helpers.

/// Truncate a string to `n` Unicode characters, appending an ellipsis when shortened.
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n).collect();
        out.push('…');
        out
    }
}

/// Join an optional list of strings with a custom separator.
pub fn join_optional(list: &Option<Vec<String>>, sep: &str) -> String {
    list.as_ref().map(|v| v.join(sep)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let s = "a".repeat(100);
        let t = truncate(&s, 10);
        assert_eq!(t.chars().count(), 11);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn join_optional_default_sep() {
        let list = Some(vec!["a".into(), "b".into()]);
        assert_eq!(join_optional(&list, ", "), "a, b");
    }

    #[test]
    fn join_optional_custom_sep() {
        let list = Some(vec!["a".into(), "b".into()]);
        assert_eq!(join_optional(&list, ","), "a,b");
    }

    #[test]
    fn join_optional_empty() {
        let list: Option<Vec<String>> = None;
        assert_eq!(join_optional(&list, ", "), "");
    }
}
