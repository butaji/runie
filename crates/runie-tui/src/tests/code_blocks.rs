//! Tests for syntax highlighted code blocks

#[cfg(test)]
mod tests {
    use crate::markdown::{extract_code_blocks, CodeBlock};

    #[test]
    fn detects_fenced_code_block() {
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
        // Should treat entire text as plain text since block not closed
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], CodeBlock::Text(_)));
    }

    #[test]
    fn multiple_code_blocks() {
        let text = "```a\ncode1\n```\n```b\ncode2\n```";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], CodeBlock::Code { lang, .. } if lang == "a"));
        assert!(matches!(&blocks[1], CodeBlock::Code { lang, .. } if lang == "b"));
    }
}
