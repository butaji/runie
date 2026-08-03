//! Canonical parsing for `<think>...</think>` blocks.

/// Remove reasoning blocks from text while preserving visible content.
/// An incomplete opening tag consumes the remainder of the input.
pub fn strip_thinking_tags(content: &str) -> String {
    split_think_blocks(content).0
}

/// Split visible text from reasoning blocks.
///
/// An unclosed `<think>` block is treated as reasoning through end of input.
pub fn split_think_blocks(content: &str) -> (String, Option<String>) {
    static THINK_REGEX: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(?s)<think>(.*?)</think>").unwrap());

    let complete_reasoning = THINK_REGEX
        .captures_iter(content)
        .filter_map(|capture| capture.get(1).map(|m| m.as_str()))
        .collect::<String>();
    let last_complete_end = THINK_REGEX
        .find_iter(content)
        .last()
        .map(|m| m.end())
        .unwrap_or(0);
    let remaining = &content[last_complete_end..];
    let (visible, unclosed_reasoning) = match remaining.find("<think>") {
        Some(pos) => {
            let visible_end = if last_complete_end == 0 {
                pos
            } else {
                last_complete_end + pos
            };
            (
                &content[..visible_end],
                Some(&remaining[pos + "<think>".len()..]),
            )
        }
        None => (content, None),
    };

    let visible = THINK_REGEX.replace_all(visible, "").to_string();
    let reasoning = match unclosed_reasoning {
        Some(tail) => format!("{complete_reasoning}{tail}"),
        None if complete_reasoning.is_empty() => String::new(),
        None => complete_reasoning,
    };
    if reasoning.is_empty() {
        (visible, None)
    } else {
        (visible, Some(reasoning))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_parser_handles_closed_and_unclosed_blocks() {
        let cases = [
            ("answer", "answer", None),
            ("<think>reason</think>answer", "answer", Some("reason")),
            ("answer<think>partial", "answer", Some("partial")),
            ("<think>a</think>x<think>b</think>y", "xy", Some("ab")),
        ];
        for (input, visible, reasoning) in cases {
            assert_eq!(
                split_think_blocks(input),
                (visible.to_string(), reasoning.map(str::to_string))
            );
            assert_eq!(strip_thinking_tags(input), visible);
        }
    }

    #[test]
    fn incomplete_markup_never_leaks_into_visible_text() {
        let (visible, reasoning) = split_think_blocks("before<think>not visible");
        assert_eq!(visible, "before");
        assert_eq!(reasoning.as_deref(), Some("not visible"));
    }
}
