# Disable arboard image features for text-only clipboard

## Status

`done`

## Context

Runie uses only text clipboard (`set_text`/`get_text`), but `arboard` default features pull `image`, `objc2-*`, `x11rb`, `wl-clipboard-rs`, etc.

## Goal

Use `arboard = { version = "3", default-features = false, features = ["wayland-data-control"] }` (or platform-specific minimal features).

## Acceptance Criteria

- [ ] Update workspace `Cargo.toml`.
- [ ] Verify text clipboard still works on Linux/macOS/Windows.
- [ ] `cargo tree` no longer shows image/X11 deps for clipboard.
- [ ] CI passes on all platforms.

## Design Impact

No change to TUI element design or composition. Only dependency graph changes.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Clipboard tests pass.
- **Live tmux validation:** Copy a message and paste outside the TUI.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
