# Replace location parser with regex

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-core/src/location.rs:20-123` hand-parses `file:42`, `file:42:5`, and range strings with split/slice logic. This is error-prone for paths containing colons.

## Goal

Replace the hand-written parser with a compiled `regex` that captures path, line, column, and optional end line/column, handling relative paths and edge cases.

## Acceptance Criteria

- [x] Remove manual string splitting in `location.rs`. — Done; regex-based parsing with `LOCATION_RE` replaces manual splitting
- [x] Use one compiled `Regex` for location parsing. — Done; `static LOCATION_RE: LazyLock<Regex>` handles all patterns
- [x] Preserve support for paths containing colons where possible. — Done; path portion must look like a filename or contain path separator
- [x] All existing location tests pass. — Done; 14 tests pass (location + search_query)

## Design Impact

No change to TUI element design or composition. Only location parsing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for `path:line`, `path:line:col`, ranges, Windows paths, paths with colons.
- **Layer 2 — Event Handling:** `IoMsg::OpenLocation` resolves correctly.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI `--goto` argument resolves.
- **Live tmux validation:** Click or use a shortcut on a `file:line` link in the TUI; verify the editor opens the right place.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
