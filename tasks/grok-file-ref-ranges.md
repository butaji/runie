# @File Line Ranges

**Status**: todo
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie's `@file` picker attaches whole files. Add Grok-style line-range support
so users can attach only the relevant lines: `@src/main.rs:10-50`.

## Acceptance Criteria

- [ ] Typing `@path:10-50` in the input triggers the file picker on `path`
  and inserts the range syntax on selection.
- [ ] `read_file_ref` accepts a path with optional `:start-end` suffix and
  returns only those lines.
- [ ] Invalid ranges (start > end, out of bounds) produce a clear error.
- [ ] Attached file blocks render with the line range in the header.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn parse_file_ref_with_range() {
    let (path, range) = parse_file_ref("src/main.rs:10-50").unwrap();
    assert_eq!(path, "src/main.rs");
    assert_eq!(range, Some(10..=50));
}

#[test]
fn parse_file_ref_without_range() {
    let (path, range) = parse_file_ref("src/main.rs").unwrap();
    assert_eq!(path, "src/main.rs");
    assert_eq!(range, None);
}

#[test]
fn read_file_ref_extracts_lines() {
    let text = "line1\nline2\nline3\nline4\nline5";
    let result = extract_lines(text, 2..=4);
    assert_eq!(result, "line2\nline3\nline4");
}
```

### Layer 2 — Event Handling

```rust
#[test]
fn at_ref_picker_preserves_range_suffix() {
    let mut state = AppState::default();
    state.input.input = "@src/main.rs:10-50".into();
    state.update(Event::AtFilePicker);
    // Assert picker opens with base path "src/main.rs".
}
```

## Files touched

- `crates/runie-core/src/file_refs.rs`
- `crates/runie-core/src/update/at_refs.rs`
- `crates/runie-core/src/ui/elements.rs` (file attachment block)

## Out of scope

- `start+count` or single-line syntax (Grok uses `start-end` only).
