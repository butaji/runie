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
- **Live tmux validation:** Open `/save` or `/login` form, fill it, submit, and verify the expected action runs.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
