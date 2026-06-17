# Unify `resolve_path` Function

**Status**: done
**Completed**: 2026-06-16
**Notes**: Extracted `resolve_path` to `runie_core::tool::resolve_path`. All 7 tool implementations in `runie-engine` now use the shared helper. Added 2 Layer 1 tests. cargo test --workspace passes.
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Extract duplicated `resolve_path` function from 7 tool files into a shared utility.

**Duplicated code (7 copies):**
```rust
fn resolve_path(path: &str, working_dir: &std::path::Path) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        working_dir.join(p)
    }
}
```

**Files with duplication:**
- `crates/runie-core/src/tool/read_file.rs:96`
- `crates/runie-core/src/tool/write_file.rs:101`
- `crates/runie-core/src/tool/list_dir.rs:97`
- `crates/runie-core/src/tool/edit_file.rs:136`
- `crates/runie-core/src/tool/grep.rs:140`
- `crates/runie-core/src/tool/find.rs:90`
- `crates/runie-core/src/tool/search.rs:150`

## Acceptance Criteria

- [ ] `resolve_path` extracted to `crates/runie-core/src/tool/mod.rs`
- [ ] All 7 files updated to use shared function
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `resolve_path_absolute` — absolute paths returned as-is.
- [ ] `resolve_path_relative` — relative paths joined with working_dir.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool/read_file.rs`
- `crates/runie-core/src/tool/write_file.rs`
- `crates/runie-core/src/tool/list_dir.rs`
- `crates/runie-core/src/tool/edit_file.rs`
- `crates/runie-core/src/tool/grep.rs`
- `crates/runie-core/src/tool/find.rs`
- `crates/runie-core/src/tool/search.rs`
