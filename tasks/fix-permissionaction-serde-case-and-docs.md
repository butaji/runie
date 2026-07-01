# Fix PermissionAction serde case and docs

## Status

`done`

## Context

`PermissionAction` serializes as PascalCase (`Allow`/`Ask`/`Deny`) but all docs/examples show lowercase; generated schema matches PascalCase.

## Goal

Add `#[serde(rename_all = "snake_case")]`, regenerate schema, and align docs.

## Acceptance Criteria
- [ ] Add serde rename attribute.
- [ ] Regenerate `config.schema.json`.
- [ ] Update all doc examples.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or documentation changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for serialization round-trip.
- **Layer 2 — Event Handling:** Config fact carries lowercase value.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Config loading tests pass.
- **Live tmux validation:** Permission config works.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
