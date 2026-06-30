# Make trust and auth file persistence atomic and restricted

## Status

`todo`

## Context

`crates/runie-core/src/trust.rs:37-44` writes `~/.runie/trust.json` with plain `std::fs::write`, no lock, no temp+rename, and no permission restrictions. `crates/runie-core/src/auth/storage.rs:146-159` writes the auth fallback with default world-readable permissions on Unix.

## Goal

Use `fs2` advisory locks + atomic temp+rename writes for trust/auth files, and set Unix mode `0o600`.

## Acceptance Criteria

- [ ] Add a small persistence helper used by both trust and auth modules.
- [ ] Write to temp file, `fsync`, rename atomically, under `fs2` lock.
- [ ] Set `0o600` on Unix.
- [ ] Tests cover concurrent writers and permission checks.

## Design Impact

No change to TUI element design or composition. Only file persistence safety changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for atomic writes, locks, and permissions.
- **Layer 2 — Event Handling:** `TrustUpdated` / `AuthProvidersLoaded` facts unchanged.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI persists trust decisions without corruption.
- **Live tmux validation:** Approve a trust decision; inspect file permissions.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new/integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
