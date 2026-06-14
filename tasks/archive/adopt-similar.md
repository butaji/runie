# Adopt `similar` for Diff Generation

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: crate-replacement-audit

## Description

Replace the custom LCS-based diff generator in
`crates/runie-agent/src/diff.rs` with the `similar` crate. `similar` provides
Myers, Patience, and Hunt-McIlroy algorithms, deadlines for large inputs,
and dependency-free operation. Context7 ID: `/mitsuhiko/similar`.

## Acceptance Criteria

- [x] Add `similar = "3"` to `crates/runie-agent/Cargo.toml`.
- [x] Replace `generate_unified_diff` with `similar::TextDiff::from_lines`.
- [x] Preserve the existing `UnifiedDiff` / `DiffHunk` / `DiffLine` types.
- [x] Delete `longest_common_subsequence`, `build_hunks`, and helper functions.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `diff_adds_and_removes_lines` — added/removed lines detected.
- [x] `diff_empty_when_identical` — identical content returns empty hunks.
- [x] `diff_large_file_completes` — deadline prevents hang on large inputs.

## Notes

**ctx7 snippet:**
```rust
use similar::{ChangeTag, TextDiff};
let diff = TextDiff::from_lines(old, new);
for change in diff.iter_all_changes() {
    let sign = match change.tag() {
        ChangeTag::Delete => "-",
        ChangeTag::Insert => "+",
        ChangeTag::Equal => " ",
    };
}
```

**Files touched:**
- `crates/runie-agent/Cargo.toml`
- `crates/runie-agent/src/diff.rs`
- `crates/runie-tui/src/diff.rs` (if types change)

**Out of scope:**
- Replacing `crates/runie-tui/src/diff.rs` unified-diff parser unless the
  agent layer stops producing raw diff strings.
