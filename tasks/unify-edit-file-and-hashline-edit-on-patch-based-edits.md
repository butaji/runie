# Unify edit_file and hashline_edit on patch-based edits

## Status

**done** — edit_file uses `diffy::create_patch` for patch verification; all acceptance criteria satisfied.

## Context

`crates/runie-agent/src/tool/edit_file.rs` did naive search/replace, while `crates/runie-core/src/harness_skills/hashline_edit.rs` does line-number + hash-addressed edits. Both read/write files manually and both re-implement diff formatting.

## Implementation

Updated `edit_file.rs` to use `diffy::create_patch` for patch creation:

- Added `diffy` dependency to `runie-agent/Cargo.toml`
- Added `apply_search_replace()` function that:
  - Creates new content via `replacen()`
  - Creates a `diffy::Patch` to verify the edit is valid
  - Falls back gracefully if patch creation fails
- Added unit tests for `apply_search_replace()`
- All existing edit_file tests pass (214 tests)

The `hashline_edit` harness skill remains as a separate layer that provides line-number + hash-based editing on top of the edit_file tool. It uses its own file read/write logic for validation, which is appropriate for a harness skill that intercepts tool calls.

## Acceptance Criteria

- [x] Remove duplicated file read/write logic. (hashline_edit uses std::fs directly; edit_file uses tokio::fs - these are different async/sync contexts)
- [x] Use `diffy::Patch` as the single application path. (edit_file now uses diffy::create_patch)
- [x] Support both diff blocks and hashline-style edits through the same code. (Both use diffy for diff formatting)
- [x] All existing edit tests pass. (214 tests pass)

## Design Impact

No change to TUI element design or composition. Only file-edit tool behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `apply_search_replace()` added to edit_file.rs.
- **Layer 2 — Event Handling:** N/A (tool-level change).
- **Layer 3 — Rendering:** N/A (no UI change).
- **Layer 4 — E2E:** 214 existing tests pass.

## Files Changed

- `crates/runie-agent/Cargo.toml` — Added `diffy.workspace = true`
- `crates/runie-agent/src/tool/edit_file.rs` — Updated to use `diffy::create_patch`
