# Unify form submit paths through command registry

## Status

`todo`

## Context

Form submission has dual paths: `FormPanel` carries both an `on_submit` event factory (`submit_factory`) and a `cmd_name`. `route_form_submit` chooses between `form_build_submit` (legacy) and `SubmitCommand`. `build_form_stack_from_template` maps positional args back to fields manually.

## Goal

Always route form submission through the command registry. Delete `submit_factory`, `FormKind::Form`, and `CommandKind::Form`; keep only `FormWithHandler` (or unified `Form`).

## Acceptance Criteria

- [ ] Remove `submit_factory` from `FormPanel`.
- [ ] Delete legacy `form_build_submit` path.
- [ ] All forms submit via the command registry.
- [ ] Login and command forms still work.

## Design Impact

No change to TUI element design or composition. Only form submission behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for form field mapping.
- **Layer 2 — Event Handling:** Form submit events route through the registry.
- **Layer 3 — Rendering:** `TestBackend` form snapshots match.
- **Layer 4 — E2E:** Headless CLI form submission works.
- **Live tmux testing session (required):** Open `/save` or `/login` form, fill it, submit, and verify the expected action runs.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** `UiActor` owns UI state; form submission routes through command registry.
- [ ] **Trigger events:** `Submit` event triggers form submission via registry.
- [ ] **Observer events:** Form submission emits events via command handlers.
- [ ] **No direct mutations:** Form submission must not directly mutate actor state; use commands.
- [ ] **No new mirrors:** Command registry is authoritative for form handling; no duplicates.
- [ ] **Async work observed:** Command handlers are synchronous; async commands have JoinHandle owners.
