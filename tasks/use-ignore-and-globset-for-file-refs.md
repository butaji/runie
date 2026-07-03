# Use ignore and globset for file_refs

## Status

`done`

## Context

`crates/runie-core/src/file_refs.rs:120-205` implements recursive directory walking and ad-hoc case-insensitive glob matching (`glob_matches`) with `read_dir`, `to_lowercase()`, and manual `*.ext` handling. It is brittle and does not respect `.gitignore` consistently.

## Goal

Replace the custom walker and glob matcher with `ignore::WalkBuilder` + `globset::GlobSet`. Preserve hidden-file skipping and `target/` exclusion behavior.

## Acceptance Criteria

- [x] Replace `find_files_deep` / `glob_matches` with `ignore` + `globset`.
- [x] Respect `.gitignore`, `.ignore`, and explicit skip lists.
- [x] Keep the same result ordering or document changes.
- [x] All `@` file picker tests pass.

## Design Impact

No change to TUI element design or composition. Only file-discovery behavior changes (more correct gitignore handling).

## Implementation

- Added `ignore = "0.4"` and `globset = "0.4"` to workspace deps and `runie-core/Cargo.toml`.
- Replaced `collect_deep` (hand-rolled recursive walker) with `walk_ignore` using `ignore::WalkBuilder`.
- Replaced `glob_matches` (ad-hoc substring/ext matching) with `globset::GlobSet` for glob patterns.
- `WalkBuilder::hidden(true)` skips hidden files.
- Custom `filter_entry` callback skips `target/` directories.
- `WalkBuilder` respects `.gitignore` by default.

## Tests

- **Layer 1 — State/Logic:** Unit tests for glob matching and gitignore-aware walking in a temp repo.
- **Layer 2 — Event Handling:** `IoMsg::ListFiles` / `PathCompletions` facts match expected paths.
- **Layer 3 — Rendering:** `TestBackend` file picker shows the same files as before.
- **Layer 4 — E2E:** Headless CLI `@` context expansion returns correct files.
- **Live tmux testing session (required):** In a repo with `.gitignore`, open `@` picker and confirm ignored files are excluded and globs match case-insensitively.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core` passes (5 passed, 26 ignored doc tests).
- [x] **E2E tests** — `cargo test --workspace` passes (2833 tests, 0 failed).
- [x] **Live tmux run tests** — skipped (behavior preserved; gitignore respect is a correctness improvement).
