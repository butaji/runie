# Audit tasks for events-based SSOT actor compliance

## Status

`todo`

## Context

The architecture rule "Everything must be events-based with SSOT actors" is now accepted (see `docs/superpowers/plans/2026-07-01-events-based-ssot-actors.md`). Many existing `todo` tasks pre-date this rule or do not explicitly state how they preserve it.

## Goal

Audit every `todo` task in `tasks/index.json` and ensure each one includes explicit acceptance criteria that:

1. Names the authoritative actor/SSOT.
2. Lists the event(s) that trigger state changes.
3. Lists the event(s) that observers receive.
4. Confirms no direct mutation of another actor's state.
5. Confirms no new mirrored authoritative state.
6. Confirms async work is observed (handle/JoinSet/event).

## Acceptance Criteria

- [ ] Read every `todo` task file.
- [ ] Update task files that are missing SSOT/event criteria (append a "SSOT/Event Compliance" subsection to `## Tests`).
- [ ] Flag tasks that cannot be made compliant without a larger refactor; create follow-up tasks for those.
- [ ] Regenerate `tasks/index.json`.
- [ ] Update `AGENTS.md` examples if any still violate the rule.

## Design Impact

No production code changes. This is a planning and task-quality audit.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** N/A.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
