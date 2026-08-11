//! Renderer-neutral facts shared by bounded tool-output projections.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputFacts {
    pub bytes: usize,
    pub lines: usize,
    pub truncated: bool,
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
}
