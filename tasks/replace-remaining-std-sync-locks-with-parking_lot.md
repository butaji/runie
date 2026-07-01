# Replace remaining std sync locks with parking_lot

## Status

`done`

## Context

Production code still uses `std::sync::Mutex`/`RwLock` in `actors/provider/factory.rs`, `session/tree.rs`, `auth/store_trait.rs`, `harness_skills/startup_context.rs`, and `harness_skills/loop_detector.rs` despite a done normalization task.

## Goal

Replace with `parking_lot` equivalents and remove poison-unwrap risks.

## Acceptance Criteria
- [x] Replace `std::sync::Mutex`/`RwLock` with `parking_lot`.
- [x] Remove `.lock().unwrap()` patterns.
- [x] Document any intentional std-lock exceptions.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, internal architecture, or async runtime changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** `cargo test --workspace` passes.
- **Live tmux validation:** N/A.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
