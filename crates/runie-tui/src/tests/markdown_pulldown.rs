//! Tests for pulldown-cmark based markdown parsing.

#[cfg(test)]
mod tests {
    use crate::markdown::{extract_code_blocks, parse_inline_markdown, CodeBlock};
    use ratatui::style::Modifier;

    #[test]
    fn inline_code_parsed() {
        let spans = parse_inline_markdown("use `cargo test` to run");
        let code = spans.iter().find(|s| s.content == "cargo test");
        assert!(code.is_some(), "Should have code span 'cargo test'");
        assert!(code.unwrap().style.bg.is_some(), "Code should have bg");
    }

    #[test]
    fn bold_text_parsed() {
        let spans = parse_inline_markdown("hello **world** test");
        let bold = spans.iter().find(|s| s.content == "world");
        assert!(bold.is_some());
        assert_eq!(bold.unwrap().style.add_modifier, Modifier::BOLD);
    }

    #[test]
    fn italic_text_parsed() {
        let spans = parse_inline_markdown("hello *world* test");
        let italic = spans.iter().find(|s| s.content == "world");
        assert!(italic.is_some());
        assert_eq!(italic.unwrap().style.add_modifier, Modifier::ITALIC);
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
        assert!(spans.iter().any(|s| s.content.contains('*')));
    }

    #[test]
    fn detects_code_block() {
        let text = "Some text\n```rust\nfn main() {}\n```\nMore text";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], CodeBlock::Text(_)));
        assert!(matches!(&blocks[1], CodeBlock::Code { lang, .. } if lang == "rust"));
        assert!(matches!(&blocks[2], CodeBlock::Text(_)));
    }

    #[test]
    fn code_block_extracts_language() {
        let text = "```python\nprint('hi')\n```";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        if let CodeBlock::Code { lang, .. } = &blocks[0] {
            assert_eq!(lang, "python");
        } else {
            panic!("Expected code block");
        }
    }

    #[test]
    fn no_code_blocks_returns_single_text() {
        let text = "Just plain text";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], CodeBlock::Text(t) if t == "Just plain text"));
    }

    #[test]
    fn unclosed_code_block_is_text() {
        let text = "```rust\nfn main() {}";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], CodeBlock::Text(t) if t.contains("fn main")));
    }

    #[test]
    fn multiple_code_blocks() {
        let text = "```a\ncode1\n```\n```b\ncode2\n```";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], CodeBlock::Code { lang, .. } if lang == "a"));
        assert!(matches!(&blocks[1], CodeBlock::Code { lang, .. } if lang == "b"));
    }

    #[test]
    fn unordered_list_parsed() {
        let text = "Here are items:\n- first item\n- second item\n- third item";
        let blocks = extract_code_blocks(text);
        let list = blocks.iter().find_map(|b| match b {
            CodeBlock::List {
                ordered: false,
                items,
            } => Some(items),
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
            CodeBlock::List {
                ordered: true,
                items,
            } => Some(items),
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
        assert!(blocks
            .iter()
            .any(|b| matches!(b, CodeBlock::Code { lang, .. } if lang == "rust")));
    }

    #[test]
    fn parse_table() {
        let text = "| a | b |\n|---|---|\n| 1 | 2 |";
        let blocks = extract_code_blocks(text);
        assert!(!blocks.is_empty(), "Table should parse without panic");
    }

    fn setup_code_block_state() -> runie_core::AppState {
        use runie_core::{AppState, Event};
        let mut state = AppState::default();
        state.agent.streaming = true;
        state.update(Event::AgentResponse {
            id: "req.0".into(),
            content: "```rust\nfn main() {}\n```".into(),
        });
        state.update(Event::AgentDone { id: "req.0".into() });
        state
    }

    fn code_line_has_non_default_color(buf: &ratatui::buffer::Buffer) -> bool {
        use ratatui::style::Color;
        for y in 0..buf.area().height {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect();
            if line.contains("fn main") {
                return (0..buf.area().width).any(|x| {
                    let fg = buf[(x, y)].style().fg;
                    fg.is_some() && fg != Some(Color::Reset)
                });
            }
        }
        false
    }

    #[test]
    fn code_block_renders_with_syntect_colors() {
        use crate::tests::draw_state;
        use crate::ui::view;
        use ratatui::{backend::TestBackend, Terminal};

        let mut state = setup_code_block_state();
        let content = draw_state(&mut state);
        assert!(
            content.contains("fn main"),
            "Rendered output should contain code content"
        );

        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| view(f, &mut state)).unwrap();
        assert!(
            code_line_has_non_default_color(terminal.backend().buffer()),
            "Code block line should have syntect foreground colors"
        );
    }
}
