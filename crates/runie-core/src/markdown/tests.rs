use super::*;

#[test]
fn parse_markdown_splits_fenced_code() {
    let text = "hello\n```rust\nlet x = 1;\n```\nworld";
    let blocks = parse_markdown(text);
    assert_eq!(blocks.len(), 3);
    assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "hello"));
    assert!(
        matches!(&blocks[1], CodeBlock::Code { lang, content } if lang == "rust" && content == "let x = 1;\n")
    );
    assert!(matches!(&blocks[2], CodeBlock::Text { content, .. } if content == "world"));
}

#[test]
fn parse_markdown_handles_unclosed_fence() {
    let text = "hello\n```rust\nlet x = 1;";
    let blocks = parse_markdown(text);
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "hello"));
    assert!(
        matches!(&blocks[1], CodeBlock::Text { content, .. } if content.starts_with("```rust"))
    );
}

#[test]
fn parse_markdown_handles_lists() {
    let text = "items:\n- one\n- two\n";
    let blocks = parse_markdown(text);
    assert_eq!(blocks.len(), 2);
    assert!(matches!(&blocks[0], CodeBlock::Text { content, .. } if content == "items:"));
    assert!(
        matches!(&blocks[1], CodeBlock::List { ordered, items } if !ordered && items.len() == 2)
    );
}

#[test]
fn parse_markdown_handles_blockquote() {
    let text = "> quote line 1\n> quote line 2";
    let blocks = parse_markdown(text);
    assert_eq!(blocks.len(), 1);
    assert!(matches!(&blocks[0], CodeBlock::Blockquote(q) if q.lines().count() == 2));
}

#[test]
fn text_block_has_inlines() {
    let blocks = parse_markdown("hello **bold** world");
    assert_eq!(blocks.len(), 1);
    let CodeBlock::Text { content, inlines } = &blocks[0] else {
        panic!()
    };
    assert_eq!(content, "hello **bold** world");
    assert!(inlines
        .iter()
        .any(|s| matches!(s, MdInline::Text(t) if t == "hello ")));
    assert!(inlines
        .iter()
        .any(|s| matches!(s, MdInline::Bold(b) if b == "bold")));
    assert!(inlines
        .iter()
        .any(|s| matches!(s, MdInline::Text(t) if t == " world")));
}

#[test]
fn inlines_to_text_round_trips() {
    let text = "hello **bold** world";
    let inlines = parse_inline_spans(text);
    assert_eq!(inlines_to_text(&inlines), text);
}

#[test]
fn extract_code_blocks_alias_works() {
    // Verify backward compat — extract_code_blocks returns same blocks as parse_markdown
    let text = "hello\n```rust\nlet x;\n```";
    assert_eq!(extract_code_blocks(text), parse_markdown(text));
}

#[test]
fn text_block_content_helper() {
    let blocks = parse_markdown("hello **bold**");
    assert_eq!(text_block_content(&blocks[0]), Some("hello **bold**"));
}
