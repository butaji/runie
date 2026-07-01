# Remove dead redb optional dependency

## Status

`done`

## Context

`crates/runie-core/Cargo.toml:83` declares a `redb-migration` feature and `redb` dependency, but there are zero `redb::` usages in source.

## Goal

Remove the feature and dependency.

## Acceptance Criteria

- [ ] Remove `redb` from `Cargo.toml`.
- [ ] Remove the feature.
- [ ] `cargo check --workspace` passes.

## Design Impact

No change to TUI element design or composition. Only dependency graph changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo check --workspace` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
