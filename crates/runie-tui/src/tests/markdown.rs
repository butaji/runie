//! Tests for markdown rendering in agent messages

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;
    use crate::markdown::parse_inline_markdown;

    #[test]
    fn bold_text_parsed() {
        let spans = parse_inline_markdown("hello **world** test");
        let bold_span = spans.iter().find(|s| s.content == "world");
        assert!(bold_span.is_some(), "Should have bold 'world' span");
        assert!(
            bold_span.unwrap().style.add_modifier == Modifier::BOLD,
            "Bold span should have BOLD modifier"
        );
    }

    #[test]
    fn italic_text_parsed() {
        let spans = parse_inline_markdown("hello *world* test");
        let italic_span = spans.iter().find(|s| s.content == "world");
        assert!(italic_span.is_some(), "Should have italic 'world' span");
        assert!(
            italic_span.unwrap().style.add_modifier == Modifier::ITALIC,
            "Italic span should have ITALIC modifier"
        );
    }

    #[test]
    fn plain_text_no_modifiers() {
        let spans = parse_inline_markdown("plain text");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "plain text");
    }

    #[test]
    fn mixed_bold_and_italic() {
        let spans = parse_inline_markdown("**bold** and *italic*");
        let has_bold = spans.iter().any(|s| s.content == "bold");
        let has_italic = spans.iter().any(|s| s.content == "italic");
        assert!(has_bold, "Should have bold");
        assert!(has_italic, "Should have italic");
    }

    #[test]
    fn unmatched_asterisks_ignored() {
        let spans = parse_inline_markdown("hello *world");
        // The asterisk and remaining text should be plain
        assert!(spans.iter().any(|s| s.content.contains("*")));
    }
}
