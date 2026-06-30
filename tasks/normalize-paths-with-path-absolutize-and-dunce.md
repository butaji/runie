# Normalize paths with path-absolutize

## Status

`done`

**Completed:** 2026-06-30

## Context

`crates/runie-core/src/path.rs:10-45` implements a custom `normalize_path` that walks `Path::components` and pops on `ParentDir`. It does not handle Windows verbatim prefixes or UNC edge cases cleanly, and tilde expansion is handled separately.

## Goal

Replace the custom normalizer with `std::path::absolute` (Rust 1.79+) or the `path-absolutize` / `dunce` crates, while keeping `shellexpand` for `~` expansion.

## Acceptance Criteria

- [x] Remove custom `normalize_path` implementation.
- [x] Use `path-absolutize` for cross-platform normalization.
- [x] Preserve tilde expansion behavior via `shellexpand`.
- [x] All existing path tests pass.

## Design Impact

No change to TUI element design or composition. Only internal path resolution behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `~`, `..`, `.`, absolute, UNC, and verbatim paths.
- **Layer 2 — Event Handling:** `IoMsg::ResolvePath` produces the same resolved path fact.
- **Layer 3 — Rendering:** Path strings shown in status bar / popups are unchanged.
- **Layer 4 — E2E:** Headless CLI resolves `@` file references correctly.
- **Live tmux validation:** Use `@` file picker with relative, `~`, and parent-dir paths; verify correct files are inserted.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
