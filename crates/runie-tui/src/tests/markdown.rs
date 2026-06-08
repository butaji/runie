//! Tests for markdown rendering in agent messages

#[cfg(test)]
mod tests {
    use ratatui::style::Modifier;
    use crate::markdown::{parse_inline_markdown, extract_code_blocks, CodeBlock};

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

    #[test]
    fn unordered_list_parsed() {
        let text = "Here are items:\n- first item\n- second item\n- third item";
        let blocks = extract_code_blocks(text);
        let list = blocks.iter().find_map(|b| match b {
            CodeBlock::List { ordered: false, items } => Some(items),
            _ => None,
        });
        assert!(list.is_some(), "Should have unordered list");
        let items = list.unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "first item");
        assert_eq!(items[1], "second item");
        assert_eq!(items[2], "third item");
    }

    #[test]
    fn ordered_list_parsed() {
        let text = "Steps:\n1. First step\n2. Second step\n3. Third step";
        let blocks = extract_code_blocks(text);
        let list = blocks.iter().find_map(|b| match b {
            CodeBlock::List { ordered: true, items } => Some(items),
            _ => None,
        });
        assert!(list.is_some(), "Should have ordered list");
        let items = list.unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "First step");
        assert_eq!(items[1], "Second step");
    }

    #[test]
    fn blockquote_parsed() {
        let text = "Text\n> quoted text\nmore text";
        let blocks = extract_code_blocks(text);
        let quote = blocks.iter().find_map(|b| match b {
            CodeBlock::Blockquote(text) => Some(text),
            _ => None,
        });
        assert!(quote.is_some(), "Should have blockquote");
        assert!(quote.unwrap().contains("quoted text"));
    }

    #[test]
    fn mixed_content_with_list_and_codeblock() {
        let text = "Text\n- list item\n```rust\ncode\n```\nmore";
        let blocks = extract_code_blocks(text);
        assert!(blocks.iter().any(|b| matches!(b, CodeBlock::List { .. })));
        assert!(blocks.iter().any(|b| matches!(b, CodeBlock::Code { lang, .. } if lang == "rust")));
    }
}
