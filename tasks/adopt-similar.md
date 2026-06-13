# Adopt `similar` for Diff Generation

**Status**: todo
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

- [ ] Add `similar = "3"` to `crates/runie-agent/Cargo.toml`.
- [ ] Replace `generate_unified_diff` with `similar::TextDiff::from_lines`.
- [ ] Preserve the existing `UnifiedDiff` / `DiffHunk` / `DiffLine` types or
  replace them with `similar` types if rendering code is updated.
- [ ] Delete `longest_common_subsequence`, `build_hunks`, and helper functions.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `diff_adds_and_removes_lines` — added/removed lines detected.
- [ ] `diff_empty_when_identical` — identical content returns empty hunks.
- [ ] `diff_large_file_completes` — deadline prevents hang on large inputs.

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
