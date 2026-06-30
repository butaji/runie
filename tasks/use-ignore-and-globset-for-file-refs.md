# Use ignore and globset for file_refs

## Status

`todo`

## Context

`crates/runie-core/src/file_refs.rs:120-205` implements recursive directory walking and ad-hoc case-insensitive glob matching (`glob_matches`) with `read_dir`, `to_lowercase()`, and manual `*.ext` handling. It is brittle and does not respect `.gitignore` consistently.

## Goal

Replace the custom walker and glob matcher with `ignore::WalkBuilder` + `globset::GlobSet`. Preserve hidden-file skipping and `target/` exclusion behavior.

## Acceptance Criteria

- [ ] Replace `find_files_deep` / `glob_matches` with `ignore` + `globset`.
- [ ] Respect `.gitignore`, `.ignore`, and explicit skip lists.
- [ ] Keep the same result ordering or document changes.
- [ ] All `@` file picker tests pass.

## Design Impact

No change to TUI element design or composition. Only file-discovery behavior changes (more correct gitignore handling).

## Tests

- **Layer 1 — State/Logic:** Unit tests for glob matching and gitignore-aware walking in a temp repo.
- **Layer 2 — Event Handling:** `IoMsg::ListFiles` / `PathCompletions` facts match expected paths.
- **Layer 3 — Rendering:** `TestBackend` file picker shows the same files as before.
- **Layer 4 — E2E:** Headless CLI `@` context expansion returns correct files.
- **Live tmux validation:** In a repo with `.gitignore`, open `@` picker and confirm ignored files are excluded and globs match case-insensitively.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
