# Remove stale DynProvider references

## Status

`todo`

## Context

`crates/runie-core/src/actors/provider/factory.rs:45` and `docs/Architecture.md:97` still describe `DynProvider`, which was deleted.

## Goal

Update the factory doc and Architecture doc to describe `BuiltProvider` directly.

## Acceptance Criteria
- [ ] Update `factory.rs` doc comment.
- [ ] Update `Architecture.md` provider section.
- [ ] Search for any remaining `DynProvider` references.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo doc` builds with no broken links.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
