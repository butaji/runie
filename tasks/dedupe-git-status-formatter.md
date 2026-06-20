# Deduplicate git-status formatter

**Status**: todo
**Milestone**: R4
**Category**: Tools
**Priority**: P2

**Depends on**: replace-git2-with-cli
**Blocks**: none

## Description

Two copies of the same `git2::Status` → label formatter exist:

- `crates/runie-core/src/update/dialog/fff.rs:85` — `format_fff_git_status(status: git2::Status) -> String` with `STATUS_LABELS: &[(git2::Status, &str)]`.
- `crates/runie-core/src/actors/fff_indexer/search.rs:127` — `format_git_status_str(status: git2::Status) -> String` with the same `STATUS_LABELS: &[(git2::Status, &str)]` table and the same loop body.

Both produce the same human-readable status string ("M", "A", "D", etc.) used by the file picker. After `replace-git2-with-cli` lands, the `git2::Status` type itself goes away and both sites will be parsing `git status --porcelain=1` output instead — making the unification natural.

## Acceptance Criteria

- [ ] One `fn format_git_status(porcelain_char: char) -> &'static str` (or equivalent) lives in a single shared module (e.g. `actors/fff_indexer/` or a new `actors/io/git.rs` helper).
- [ ] Both `update/dialog/fff.rs` and `actors/fff_indexer/search.rs` import and call the shared helper.
- [ ] No `STATUS_LABELS` constant is duplicated anywhere in the workspace.
- [ ] `rg "STATUS_LABELS" crates/` returns exactly one hit (the shared definition).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `format_git_status_covers_all_porcelain_codes` — the shared helper maps every `git status --porcelain=1` first-column code (`M`, `T`, `A`, `D`, `R`, `C`, `U`, `?`, `!`) to the expected label.
- [ ] `format_git_status_unknown_code_falls_back` — an unexpected code returns a stable fallback (empty or `?`), not a panic.

### Layer 2 — Event Handling
- N/A — pure formatter.

### Layer 3 — Rendering
- [ ] `fff_picker_displays_status_labels` — a render test feeds file items with mixed git statuses and asserts the labels appear in the panel output.

### Layer 4 — Smoke / Crash
- [ ] `smoke_fff_search_returns_status_labels` — `FffIndexerActor` search in a temp git repo with staged + unstaged + untracked files returns items whose labels match `git status --porcelain=1`.

## Files touched

- `crates/runie-core/src/actors/fff_indexer/search.rs` (host the shared helper or import it)
- `crates/runie-core/src/update/dialog/fff.rs` (delete local copy, import shared)
- `crates/runie-core/src/actors/io/git.rs` (candidate host if `replace-git2-with-cli` adds a `GitService` helper module)

## Notes

Run after `replace-git2-with-cli` so the unification happens against the new porcelain-parsing code rather than the soon-deleted `git2::Status` enum. If that task stalls, this can land first by extracting a `fn format_git2_status(status: git2::Status) -> &'static str` and having both sites call it — the porcelain rewrite then updates one place instead of two.
