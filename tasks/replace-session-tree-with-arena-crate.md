# Replace hand-rolled session tree with an arena tree crate

**Status**: todo
**Milestone**: R6
**Category**: Sessions / Architecture
**Priority": P2

**Depends on**: unify-session-store-and-index
**Blocks**: none

## Description

`crates/runie-core/src/session/tree.rs` maintains a manual `node_index: HashMap<Vec<usize>, usize>`, version counters, and a `Mutex`-cached filter, plus brittle path-to-index math. Replace it with an arena-backed tree crate (`indextree` or `ego-tree`) for stable node IDs and deterministic traversal.

## Acceptance Criteria

- [ ] Evaluate `indextree` and `ego-tree`; pick one.
- [ ] Replace `SessionTree` internals with arena nodes.
- [ ] Preserve navigation, fork, search, and filtered walk behavior.
- [ ] Delete the versioned `HashMap` index and `Mutex` cache.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `tree_fork_and_navigate` — forking and navigating works.
- [ ] `tree_filtered_walk` — filtered traversal returns expected order.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/session/tree.rs`
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/session/mod.rs`

## Notes

- This is a larger refactor; only attempt after the session store unification is stable.
- If an arena crate adds too much API surface, a smaller Pareto fix is parent-pointer IDs + `RefCell` caching.
