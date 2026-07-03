# Rename model/snapshot.rs to compaction.rs

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/model/snapshot.rs` (76 LOC) is not a snapshot — it implements `AppState::total_tokens()` and session-compaction helpers. The real UI `Snapshot` lives in `crates/runie-core/src/snapshot.rs` (387 LOC). The name `model/snapshot.rs` misleads: readers expect it to build `Snapshot`, but it does token accounting and compaction. Rename to `model/compaction.rs` (or fold into `model/cache.rs` if the functions are small and cache-related).

## Acceptance Criteria

- [ ] `crates/runie-core/src/model/snapshot.rs` renamed to `crates/runie-core/src/model/compaction.rs` (or folded into `model/cache.rs`).
- [ ] `model/mod.rs` declares `mod compaction;` (or `cache;`) instead of `mod snapshot;`.
- [ ] No file other than `crates/runie-core/src/snapshot.rs` defines the UI `Snapshot` type.
- [ ] `rg "model::snapshot" crates/` returns zero hits (all migrated).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `total_tokens_unchanged_after_rename` — `AppState::total_tokens()` returns the same value.
- [ ] `compaction_functions_resolve_from_new_path` — `use runie_core::model::compaction::*` compiles.

### Layer 2 — Event Handling
- [ ] N/A — pure rename.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green.

## Files touched

- `crates/runie-core/src/model/snapshot.rs` → rename to `model/compaction.rs` (or fold into `model/cache.rs`)
- `crates/runie-core/src/model/mod.rs` — update module declaration + re-exports
- All files importing `crate::model::snapshot::` or `runie_core::model::snapshot::` (grep-driven)

## Notes

Use `git mv` to preserve history. If folding into `cache.rs`, verify `cache.rs` stays under the 500-line file limit. The naming collision between `snapshot.rs` (UI) and `model/snapshot.rs` (compaction) was introduced when the UI `Snapshot` was promoted to a top-level type; this rename completes the separation.
