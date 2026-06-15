# Fix harness/ Crate or Archive It

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

`harness/` contained Rust source files but had no `Cargo.toml` and was not a workspace member. It referenced `regex` and `serde` without declaring dependencies, so it could not be built or tested. It was effectively dead code.

## Acceptance Criteria

- [x] The harness directory was moved to `crates/_archive/harness/`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
N/A — archived code is no longer tested.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo build --release` succeeds.

## Files touched

- `crates/_archive/harness/` (moved from root)
- `harness/` (removed)

## Notes

The harness testing framework was an experimental pattern that was never completed. If needed in the future, it can be restored from `crates/_archive/harness/`.

- Check whether `harness/` is referenced by docs, scripts, or CI before deleting.
- If kept, add explicit dependencies for `regex` and `serde` and resolve `#[allow(dead_code)]` modules.
