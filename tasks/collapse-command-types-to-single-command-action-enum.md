# Collapse command types to a single Command/Action enum

## Status

`todo`

## Context

Runie has five overlapping command representations: `CommandSpec`, `CommandDef`, `DeclarativeCommandDef`, `NamedHandler`, and `CommandKind` (`commands/dsl/spec.rs`, `commands/dsl/handlers/registry.rs`, `declarative/types.rs`). Two builder functions convert declarative definitions into `CommandDef`.

## Goal

Introduce a single `Command` struct with an `Action` enum (`Handler`, `Form { fields, handler }`, `Msg`, `Panel`). Drop `CommandSpec`, `DeclarativeCommandDef`, and the duplicate builder functions.

## Acceptance Criteria

- [ ] Define one `Command` + `Action` type in `runie-core`.
- [ ] Migrate all command registry, declarative loader, and form code.
- [ ] Preserve YAML config format (aliases allowed).
- [ ] All command tests pass.

## Design Impact

No change to TUI element design or composition. Only internal command model changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for command construction and dispatch.
- **Layer 2 — Event Handling:** Slash/form commands emit the same events.
- **Layer 3 — Rendering:** `TestBackend` command palette, forms, and panels unchanged.
- **Layer 4 — E2E:** Headless CLI commands work.
- **Live tmux testing session (required):** All common slash commands and forms behave as before.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
