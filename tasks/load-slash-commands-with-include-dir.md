# Load slash commands with include_dir

## Status

`todo`

## Context

`crates/runie-core/src/commands/dsl/embedded_commands.rs` keeps ~40 manual `include_str!` constants and a hand-maintained `ALL` table.

## Goal

Use `include_dir!` over `resources/commands/` and build the command list at compile time.

## Acceptance Criteria
- [ ] Embed command YAML directory with `include_dir!`.
- [ ] Iterate files to populate command map.
- [ ] Delete manual constants and `ALL` table.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all YAML files load and produce the same command map.
- **Layer 2 — Event Handling:** Command-loaded fact unchanged.
- **Layer 3 — Rendering:** `/help` popup snapshot unchanged.
- **Layer 4 — E2E:** Headless CLI lists all built-in commands.
- **Live tmux validation:** `/help` and `/quit` still work.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
