# Fold runie-engine into runie-agent

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`runie-engine` (2234 LOC, 17 files) has exactly one consumer: `runie-agent`. The crate boundary adds a `Cargo.toml`, workspace member, and re-export layer for no independence — tools are not pluggable today, and the `ToolRuntime` trait in `runie-core` already provides the seam for mock injection in tests. Fold the tool implementations into `runie-agent/src/tool/` (or `runie-core/src/tool/impls/` if the domain needs them). Delete the `runie-engine` crate.

## Acceptance Criteria

- [x] Audit complete: confirm `runie-engine` is consumed only by `runie-agent` (grep `runie-engine` in `Cargo.toml` files).
- [x] `crates/runie-engine/` deleted from workspace.
- [x] Tool implementations (`grep.rs`, `bash.rs`, `edit_file.rs`, `list_dir.rs`, `read_file.rs`, `write_file.rs`, `find.rs`, `fetch_docs.rs`, `find_definitions.rs`, `search/`) moved to `crates/runie-agent/src/tool/`.
- [x] `runie-agent` `Cargo.toml` absorbs engine-only deps (`reqwest`, `fff-search`, `git2`, `tempfile`).
- [x] `Cargo.toml` workspace members reduced by 1.
- [x] `builtin_registry()` accessible from `runie-agent` (and `runie-core::tool` re-exports kept for compat).
- [x] `cargo test --workspace` succeeds (2483 tests, 0 failures).
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `builtin_registry_still_assembles` — `builtin_registry()` returns the same tool set after the move.
- [x] `tool_runtime_adapter_compiles` — `ToolRuntime` adapter over the builtin registry compiles from new location.

### Layer 2 — Event Handling
- [x] `tool_execution_unchanged` — existing tool execution tests (Layer 4 mock-tool E2E) pass.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_agent_turn_with_builtin_tools` — all 4 minimax replay tests pass (`m3_list_files_turn_executes_list_dir`, etc.).

## Files touched

- `crates/runie-engine/` → delete (17 files + Cargo.toml)
- `crates/runie-agent/src/tool/` → new (absorb engine `tool/` contents)
- `crates/runie-agent/src/lib.rs` — declare `mod tool;` + re-exports
- `crates/runie-agent/Cargo.toml` — absorb engine-only deps
- `Cargo.toml` — remove workspace member + `runie-engine` workspace dep
- `Cargo.lock` — regenerated
- All files importing `runie_engine::` (grep-driven; likely only `runie-agent`)

## Notes

If tool pluggability becomes a real requirement later (third-party tool crates), the `ToolRuntime` trait in `runie-core` is the seam — it does not require `runie-engine` to be a separate crate. The 1:1 consumer relationship today is the YAGNI signal. Rejected alternative: keep `runie-engine` "for when tools are pluggable" — rejected because speculative crate boundaries add overhead now for a future that may not arrive; re-extracting is cheap if it does.
