# Replace hand-rolled session tree with an arena tree crate

**Status**: done
**Milestone**: R6
**Category**: Sessions / Architecture
**Priority**: P2

**Depends on**: unify-session-store-and-index
**Blocks**: none

## Description

`crates/runie-core/src/session/tree.rs` maintains a manual `node_index: HashMap<Vec<usize>, usize>`, version counters, and a `Mutex`-cached filter, plus brittle path-to-index math. Replaced with arena-backed tree using `indextree` for stable node IDs and deterministic traversal.

## Acceptance Criteria

- [x] Evaluate `indextree` and `ego-tree`; pick one. → Chose `indextree`
- [x] Replace `SessionTree` internals with arena nodes.
- [x] Preserve navigation, fork, search, and filtered walk behavior.
- [x] Delete the versioned `HashMap` index and `Mutex` cache.
- [x] `cargo test --workspace` succeeds after the change. → 1706 tests pass
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `tree_fork_and_navigate` — forking and navigating works (verified via existing tests).
- [x] `tree_filtered_walk` — filtered traversal returns expected order (verified via existing tests).

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/session/tree.rs` — Replaced with `indextree` arena implementation
- `crates/runie-core/Cargo.toml` — Added `indextree` workspace dependency
- `crates/runie-core/src/session/mod.rs` — Updated `Session` serialization and `PartialEq`
- `crates/runie-core/src/model/state/session.rs` — Updated `SessionState` with manual `PartialEq`
- `crates/runie-core/src/model/cache/mod.rs` — Updated to use new `indextree` API
- `crates/runie-core/src/update/dialog/open.rs` — Updated to use new `indextree` API

## Notes

- `indextree` was chosen over `ego-tree` for its simpler API and `NodeId` stability.
- `TreeNode` renamed to `TreeNodeData` to clarify it holds data, not tree structure.
- `SessionTree` no longer implements `PartialEq` (arena doesn't), but `Session` and `SessionState` have manual implementations.
- The `id_index: HashMap<String, NodeId>` provides O(1) message lookup instead of the old path-based index.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
