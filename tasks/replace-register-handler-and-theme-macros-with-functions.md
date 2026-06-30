# Replace register_handler and theme macros with functions

## Status

`todo`

## Context

Three macros exist only to avoid typing boilerplate:
- `register_handler!` in `crates/runie-core/src/commands/dsl/handlers/mod.rs:49-76`
- `theme_color!` / `theme_color_try!` in `crates/runie-tui/src/theme/colors.rs:9-28`
- `style_fn!` in `crates/runie-tui/src/theme/styles.rs:150-156`

They add indirection and slow compile-time understanding.

## Goal

Replace the macros with plain functions or small generic helpers.

## Acceptance Criteria

- [ ] `register_handler!` becomes `registry.register_handler(name, kind, f)` or a generic helper.
- [ ] `theme_color!` becomes plain `const`/fn getters.
- [ ] `style_fn!` becomes plain functions.
- [ ] All callers compile and behavior is unchanged.

## Design Impact

No change to TUI element design or composition. Only internal macro/function structure changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for theme color and style helpers.
- **Layer 2 — Event Handling:** Command dispatch works.
- **Layer 3 — Rendering:** `TestBackend` snapshots match.
- **Layer 4 — E2E:** Headless CLI command registration works.
- **Live tmux validation:** Theme colors and command hints look identical.

## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
