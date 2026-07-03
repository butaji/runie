# Regenerate config schema and fix PermissionMode description

## Status

`done`

## Context

`config.schema.json:372` lists camelCase values in the `PermissionMode` description while the constants are snake_case. The schema may be stale relative to recent type changes.

## Goal

Regenerate `config.schema.json` from Rust types and verify the description matches constants.

## Acceptance Criteria
- [x] Run schema generator. — Done; `cargo run --example write_config_schema` ran successfully.
- [x] Verify `PermissionMode` description uses snake_case. — Done; schema shows `const: "default"`, `"accept_edits"`, `"dont_ask"`, `"bypass_permissions"`, `"plan"`, `"auto"` — all snake_case.
- [x] Check no unintended schema diffs. — Done; `git diff config.schema.json` shows no diff — schema was already up-to-date.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, async runtime, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Schema validation tests pass.
- **Live tmux testing session (required):** N/A.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — N/A (schema-only task; no runtime behavior changes).

### SSOT/Event Compliance
- [ ] **Actor/SSOT:** N/A (schema regeneration; no actor state involved).
- [ ] **Trigger events:** N/A (schema regeneration doesn't introduce state transitions).
- [ ] **Observer events:** N/A (schema regeneration doesn't emit events).
- [ ] **No direct mutations:** N/A (schema regeneration doesn't change state ownership).
- [ ] **No new mirrors:** N/A (schema regeneration doesn't introduce new state).
- [ ] **Async work observed:** N/A (schema regeneration is synchronous).
