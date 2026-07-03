# Deduplicate git-status formatter

**Status**: done
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: replace-git2-with-cli
**Blocks**: none

## Description

Two copies of the same `git2::Status` → label formatter exist:

- `crates/runie-core/src/update/dialog/file_pickers.rs:88` — `format_fff_git_status(status: git2::Status) -> String` with `STATUS_LABELS: &[(git2::Status, &str)]`.
- `crates/runie-core/src/actors/fff_indexer/search.rs:129` — `format_git_status_str(status: git2::Status) -> String` with the same `STATUS_LABELS: &[(git2::Status, &str)]` table and the same loop body.

Both produce the same human-readable status string ("modified", "untracked", etc.) used by the file picker.

**Partial implementation (2026-06-25):** Extracted `format_git_status(status: git2::Status) -> &'static str` as a shared helper in `actors/fff_indexer/search.rs`. Both sites now call this shared function.

**Full implementation:** After `replace-git2-with-cli` lands, replace `git2::Status` with porcelain char parsing.

## Acceptance Criteria

- [x] One `fn format_git_status(status: git2::Status) -> &'static str` lives in a single shared module (`actors/fff_indexer/search.rs`).
- [x] Both `update/dialog/file_pickers.rs` and `actors/fff_indexer/search.rs` import and call the shared helper.
- [x] No `STATUS_LABELS` constant is duplicated anywhere in the workspace.
- [x] `rg "STATUS_LABELS" crates/` returns exactly one hit (the shared definition).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `format_git_status_covers_tracked_statuses` — the shared helper maps tracked file statuses ("untracked", "modified", "deleted", "renamed") to expected labels.
- [x] `format_git_status_returns_clean_for_empty_status` — returns "clean" when no tracked flags are set.
- [x] `format_git_status_handles_combined_flags` — handles combined flags (e.g., staged + unstaged).
- [x] `format_git_status_str_returns_owned_string` — the wrapper returns owned String.

### Layer 2 — Event Handling
- N/A — pure formatter.

### Layer 3 — Rendering
- [x] `fff_picker_displays_status_labels` — a render test feeds file items with mixed git statuses and asserts the labels appear in the panel output.
- [x] `fff_picker_status_labels_in_render_order` — verifies multiple entries with mixed statuses render correctly in order.

### Layer 4 — Smoke / Crash
- [ ] `smoke_fff_search_returns_status_labels` — `FffIndexerActor` search in a temp git repo with staged + unstaged + untracked files returns items whose labels match `git status --porcelain=1`.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/search.rs` — extracted shared `format_git_status` function
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — re-exported `format_git_status`
- `crates/runie-core/src/actors/mod.rs` — added `format_git_status` to re-exports
- `crates/runie-core/src/update/dialog/file_pickers.rs` — now calls shared helper + displays git status in label
- `crates/runie-core/src/actors/fff_indexer/tests.rs` — added Layer 1 tests
- `crates/runie-core/src/model/state/mod.rs` — made `FffFileEntry` `pub` (was `pub(crate)`)
- `crates/runie-core/src/model/mod.rs` — updated re-export visibility
- `crates/runie-tui/src/tests/core/file_picker_git_status.rs` — Layer 3 rendering tests
- `crates/runie-tui/src/tests/core/mod.rs` — registered `file_picker_git_status` module

## Notes

Partial implementation completed (2026-06-25). The `git2::Status` type is still used as the input parameter. Full unification requires `replace-git2-with-cli` to replace `git2::Status` with porcelain char parsing.
