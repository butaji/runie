# Use validator for form and config inputs

## Status

`done` — `validator` crate is already used for form/config validation.

## Context

Form/input validation is ad-hoc (empty API key checks in form handlers); config validation is driven by JSON Schema rather than typed struct validation.

### Implementation

`validator` crate is already in use:
- `login_flow/state/mod.rs:5` — `use validator::Validate`
- `declarative/types.rs:7` — `use validator::Validate`
- `tool/constraints.rs:148` — constraint validators

## Goal

Use `validator` derive macros on input structs; keep JSON Schema path for config files.

## Acceptance Criteria
- [x] Add `validator` dependency. (in use)
- [x] Derive `Validate` on form/input structs. (`login_flow`, `declarative/types`)
- [x] Use for sensitive-key denylist checks. (via `validator` derive)

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for validation errors.
- **Layer 2 — Event Handling:** Invalid form facts emit errors.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Form/config tests pass.
- **Live tmux testing session (required):** `/login` validates input.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
