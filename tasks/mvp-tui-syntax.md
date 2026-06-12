# Syntax highlighting

**Status**: done

**Milestone**: MVP

**Category**: TUI Rendering

## Description

Syntax highlighting for code blocks.

## Acceptance Criteria

- [x] Language detection
- [x] Keyword highlighting
- [x] Multiple language support

## Implementation

### Files

- `crates/runie-tui/src/syntax.rs` — Syntax highlighting module with tokenizer
- `crates/runie-tui/src/ui.rs` — Updated to use syntax highlighting for code blocks

### Architecture

1. **Language detection** (`Language::from_fence`):
   - Parses fence labels (rust, rs, py, python, js, javascript, etc.)
   - Supports 16 languages: Rust, Python, JavaScript, TypeScript, Go, Java, C, C++, Markdown, JSON, YAML, Bash, SQL, HTML, CSS, XML, TOML

2. **Tokenization** (`tokenize_line`):
   - Tokenizes code into keywords, types, functions, strings, numbers, comments, and plain text
   - Per-language keyword sets for Rust, Python, JavaScript, Go, Java, C, SQL, Bash
   - Type and builtin function detection

3. **Styling**:
   - Keywords: Light magenta + bold
   - Strings: Green (Indexed 114)
   - Numbers: Light green (Indexed 175)
   - Comments: Gray + italic (Indexed 245)
   - Types: Cyan (Indexed 75)
   - Functions: Light cyan (Indexed 111)

## Tests

### Layer 1 — State/Logic (syntax.rs)
- [x] `rust_keyword_highlighting` — Rust fn/main/brace
- [x] `python_keyword_highlighting` — Python def/hello
- [x] `js_string_highlighting` — JS string literal with green color
- [x] `number_highlighting` — Numbers detected
- [x] `comment_highlighting` — Comments detected
- [x] `language_detection` — All 16 language variants
- [x] `multi_language_highlight` — Rust and Python
- [x] `highlight_code_multiline` — Multi-line code
- [x] `sql_keyword_highlighting` — SQL SELECT/FROM
- [x] `go_keyword_highlighting` — Go package/main
- [x] `type_highlighting` — Rust String type
- [x] `empty_line` — Empty line handling

All 12 syntax tests pass.

### Layer 3 — Rendering (ui.rs integration)
- [x] Code blocks render with syntax highlighting in agent messages

## Lint Note

`syntax.rs` is currently **582 lines** (limit: 500). This is the only file in the
codebase exceeding the lint limit. The excess comes from keyword/type tables for
8 languages. Options:
1. Move keyword tables to a separate `syntax/keywords.rs` module
2. Generate tables at compile time via `build.rs`
3. Accept exemption for data-heavy modules

## Notes

- For performance, tokenization happens per-line
- No external dependencies (pure Rust implementation)
- Follows design system rule: all colors from theme (uses Indexed colors for portability)
