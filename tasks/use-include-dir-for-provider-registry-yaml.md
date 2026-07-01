# Use include_dir for provider registry YAML

## Status

`todo`

## Context

`crates/runie-core/src/provider/registry_data.rs:58-98` lists providers as a hard-coded `vec!` of `include_str!` calls. Adding a provider requires editing Rust code.

## Goal

Use `include_dir!` over `resources/models/`, iterate files, and deserialize each YAML directly into registry structs.

## Acceptance Criteria

- [ ] Embed `resources/models/` with `include_dir!`.
- [ ] Iterate and deserialize provider/model YAMLs at startup.
- [ ] Delete the hard-coded list.
- [ ] All provider registry tests pass.

## Design Impact

No change to TUI element design or composition. Only provider registry loading changes.

## Tests

- **Layer 1 — State/Logic:** Unit test that all YAML files load.
- **Layer 2 — Event Handling:** `ConfigLoaded` includes providers.
- **Layer 3 — Rendering:** `/settings` provider list unchanged.
- **Layer 4 — E2E:** Headless CLI lists providers.
- **Live tmux validation:** `/model` and `/provider` commands show providers.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
