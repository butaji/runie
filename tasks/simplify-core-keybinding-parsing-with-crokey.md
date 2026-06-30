# Simplify core keybinding parsing with crokey

## Status

`done`

## Context

`crates/runie-core/src/keybindings/mod.rs:22-113` contains `parse_key_combo`, `validate_key_combo`, and a `VALID_KEYS` table that duplicates parsing already available in the `crokey` crate (already a TUI dependency).

## Goal

Replace the custom keybinding combo parser with `crokey::KeyCombination` parsing/serialization, sharing the same representation between core defaults and TUI dispatch.

## Acceptance Criteria

- [x] Remove `parse_key_combo` / `validate_key_combo` / `VALID_KEYS`.
- [x] Use `crokey` to parse default binding strings into `KeyCombination`.
- [x] Keep serialization format identical in config files.
- [x] All keybinding tests pass.

## Design Impact

No change to TUI element design or composition. Only keybinding parsing behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests parsing all default bindings with `crokey`.
- **Layer 2 — Event Handling:** `ConfigMsg::SetKeybinding` resolves to the same `CoreEvent`.
- **Layer 3 — Rendering:** `/settings` keybinding display is unchanged.
- **Layer 4 — E2E:** Headless CLI loads keybindings from config.
- **Live tmux validation:** Custom keybinding in config still works after parser swap.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
