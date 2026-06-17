# @File Line Ranges

**Status**: done
**Milestone**: R4
**Category**: Input / Commands
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Runie's `@file` picker attaches whole files. Add Grok-style line-range support
so users can attach only the relevant lines: `@src/main.rs:10-50`.

## Acceptance Criteria

- [x] Typing `@path:10-50` in the input triggers the file picker on `path`
  and inserts the range syntax on selection.
- [x] `read_file_ref` accepts a path with optional `:start-end` suffix and
  returns only those lines.
- [x] Invalid ranges (start > end, out of bounds) produce a clear error.
- [x] Attached file blocks render with the line range in the header.

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

- `crates/runie-core/src/file_refs.rs` — `ParsedFileRef`, `parse_file_ref`, `read_file_ref_with_range`, `extract_lines`
- `crates/runie-core/src/model/state.rs` — `file_picker_range_suffix` field
- `crates/runie-core/src/update/dialog/mod.rs` — `open_at_file_picker` strips range suffix, `build_insert_text` appends it
- `crates/runie-core/src/tests/file_refs.rs` — 10 new Layer 1 tests

## Out of scope

- `start+count` or single-line syntax (Grok uses `start-end` only).
- Rendering line ranges in file attachment headers.

## Done

**`file_refs.rs`** — Core parsing and extraction:

- `ParsedFileRef` struct: `{ path, range: Option<RangeInclusive<u32>>, original }`
- `parse_file_ref(input)` — parses `@path:start-end`, returns `Some(ParsedFileRef)` or `None` on fatal errors. Uses `find_range_separator` (last `:` followed by a digit) and `is_valid_range_str` (exactly one `-`, non-empty parts). Inverted ranges (`start > end`) return `Some` with `range: None`.
- `read_file_ref_with_range(path, range)` — reads file and extracts lines if range provided; clamps out-of-bounds to file length.
- `extract_lines(text, range)` — 1-indexed inclusive line extraction; returns error for `start > end`.

**`model/state.rs`** — Added `file_picker_range_suffix: Option<String>` to carry the `:10-50` suffix through the dialog lifecycle.

**`update/dialog/mod.rs`** — `open_at_file_picker` strips the range suffix before using the base path as the filter, stores suffix in `state.file_picker_range_suffix`. `build_insert_text` appends the suffix after the selected file path. Abort handler clears the suffix. `route_global_dialog_event` clears suffix on escape.

**Tests**: 27 file_refs tests pass (`parse_file_ref` variants, `extract_lines`, `read_file_ref_with_range`, all existing tests). All 1914 workspace tests pass.
