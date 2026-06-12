# Add: Layer 4 Smoke Tests for CI

**Status**: done
**Milestone**: R2
**Category**: Core Architecture

## Description

Per AGENTS.md guidelines, Layer 4 smoke tests (tmux-based) should run before every push and in CI. These catch bugs that unit tests cannot: async event ordering, race conditions, stale indices, infinite loops, memory leaks.

Currently no Layer 4 tests exist in the codebase. Add a smoke test suite.

## Acceptance Criteria

- [x] Rust-native e2e suite in `crates/runie-term/tests/e2e.rs` using `rexpect` replaces all bash smoke tests
- [x] App starts without panic and renders the welcome prompt
- [x] Command palette opens, filters, and closes without panic
- [x] Help command responds with the expected help content
- [x] Login flow opens and Cancel returns to the main UI
- [x] Dialog flows (`settings`, `model`, `theme`, `scoped-models`, `thinking`) open and cancel
- [x] Session form dialogs (`save`, `load`, `delete`, `export`, `import`, `prompt`, `name`, `fork`, `compact`) open and cancel
- [x] Session commands (`new`, `clone`, `sessions`, `history`, `session`, `tree`, `skills`, `diagnostics`, `reload`, `reset`, `logout`) execute without panic
- [x] Rapid submit stress test submits multiple messages quickly with no panic or stuck timer
- [x] Resize stress test cycles through multiple terminal dimensions with no panic or stuck timer
- [x] All e2e tests pass in CI
- [x] E2E tests fail the build if any panic/stuck timer is detected

## Tests

### Layer 1 — State/Logic
N/A

### Layer 2 — Event Handling
N/A

### Layer 3 — Rendering
N/A

### Layer 4 — E2E / Smoke
- [x] `e2e_app_starts_without_panic`
- [x] `e2e_command_palette_opens_and_searches`
- [x] `e2e_help_command_responds`
- [x] `e2e_login_flow_opens_and_cancel_works`
- [x] `e2e_dialog_flows_settings_model_theme_scoped`
- [x] `e2e_session_form_dialogs_open_and_cancel`
- [x] `e2e_session_commands_no_panic`
- [x] `e2e_rapid_submit_no_panic_or_stuck_timer`
- [x] `e2e_stress_resize_and_rapid_submit`

## Running the E2E Suite

```bash
cargo build --release -p runie-term
cargo test -p runie-term --test e2e -- --ignored
```

The tests spawn the real release binary in a PTY via `rexpect`, drive the TUI with control sequences, and assert on captured output. Each test uses an isolated temporary `HOME` directory so sessions/config are not written to the user's real data directory.

## Notes

All bash/tmux smoke tests have been removed. The Rust-native suite lives in `crates/runie-term/tests/e2e.rs`.

CI should:
1. Build release binary: `cargo build --release -p runie-term`
2. Run the e2e suite: `cargo test -p runie-term --test e2e -- --ignored`
3. Fail if any test fails

**Out of scope**: Adding automated visual diffing (just check for no panics/stuck timers)
