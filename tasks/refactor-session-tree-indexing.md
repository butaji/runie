# Refactor: Add Indexing to Session Tree

**Status**: done
**Milestone**: R3
**Category**: Core Architecture

## Description

`SessionTree` in `crates/runie-core/src/session_tree.rs` uses linear traversal for operations:

- `filtered_walk()` — creates new Vec on every call, O(n) allocation
- `find_node()` — traverses tree linearly, no indexing
- `navigate_to()` — path lookup is O(path_length × branching_factor)

Add a flattened index for O(1) message lookup and cached filter results.

## Acceptance Criteria

- [x] Add `node_index: HashMap<Vec<usize>, usize>` for O(1) node lookup
- [x] Add `index_version: u64` to invalidate cache when tree changes
- [x] Cache `filtered_walk()` results with invalidation on tree modification
- [x] `navigate_to()` uses index lookup instead of traversal
- [x] Maintain backward compatibility with existing tree structure
- [x] All existing session tree tests pass

## Tests

### Layer 1 — State/Logic
- [x] `test_session_tree_index_lookup` — verify O(1) lookup
- [x] `test_session_tree_index_invalidation` — verify index cleared on tree change
- [x] `test_session_tree_navigate_with_index` — verify path navigation

### Layer 2 — Event Handling
- [x] `test_fork_session_uses_index` — verify fork works with indexed tree
- [x] `test_clone_session_uses_index` — verify clone works

### Layer 3 — Rendering
N/A

### Layer 4 — Smoke
- [x] `smoke_session_tree_operations.sh` — create tree, fork, navigate

## Notes

The index should map `current_branch` paths to node indices. When a node is added/removed, bump `index_version` and lazily rebuild index on next access.

For `filtered_walk()`, cache the result keyed by (filter, index_version). Only recompute when filter changes or tree structure changes.

**Out of scope**: Changing the public API of SessionTree
