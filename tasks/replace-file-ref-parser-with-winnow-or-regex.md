# Replace file ref parser with winnow or regex

## Status

`todo`

## Context

`crates/runie-core/src/file_refs.rs:24-90` manually parses `@path:start-end` with `rfind(':')`, digit checks, split-on-hyphen, and inverted-range fallback.

## Goal

Replace with a small `winnow`/`nom`/`regex` grammar parser.

## Acceptance Criteria
- [ ] Implement grammar parser.
- [ ] Preserve current quirks (trailing colon, inverted ranges).
- [ ] Add exhaustive unit tests.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for all reference forms.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Tool tests using file refs pass.
- **Live tmux validation:** `@file:10-20` references work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
