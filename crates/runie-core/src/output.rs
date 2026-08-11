//! Renderer-neutral facts shared by bounded tool-output projections.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputFacts {
    pub bytes: usize,
    pub lines: usize,
    pub truncated: bool,
}

/// Return a bounded Unicode-safe preview for renderer-neutral output rows.
pub fn bounded_preview(text: &str, max_chars: usize) -> Option<String> {
    (!text.is_empty()).then(|| text.chars().take(max_chars).collect())
}

pub fn output_facts(text: &str, truncated: bool) -> OutputFacts {
    OutputFacts {
        bytes: text.len(),
        lines: text.lines().count(),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_facts_are_one_data_projection() {
        assert_eq!(
            output_facts("one\ntwo\n", true),
            OutputFacts {
                bytes: 8,
                lines: 2,
                truncated: true
            }
        );
        assert_eq!(output_facts("", false).lines, 0);
    }

    #[test]
    fn bounded_preview_is_unicode_safe_and_data_shaped() {
        assert_eq!(bounded_preview("αβγ", 2).as_deref(), Some("αβ"));
        assert_eq!(bounded_preview("", 2), None);
    }
}
