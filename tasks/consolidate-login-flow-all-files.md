# Consolidate all login_flow files into one directory

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1

**Depends on**: consolidate-login-flow-handlers
**Blocks**: none

## Description

The login-flow feature is spread across 11 files in 5 different module roots:

| File | Root | Role |
|------|------|------|
| `login_flow/mod.rs` | `login_flow/` | State machine module root |
| `login_flow/validation.rs` | `login_flow/` | Key validation |
| `login_flow/panels.rs` | `login_flow/` | Panel builders |
| `login_flow/state.rs` | `login_flow/` | `LoginFlowState` |
| `login_flow/state_tests.rs` | `login_flow/` | State tests |
| `login_flow/tests.rs` | `login_flow/` | General tests |
| `update/login_flow.rs` | `update/` | Event handlers |
| `update/login_flow/tests.rs` | `update/login_flow/` | Handler tests |
| `event/login_flow.rs` | `event/` | `LoginFlowEvent` alias |
| `tests/login_logout/login_flow.rs` | `tests/` | E2E tests |
| `tui/src/tests/login_flow_e2e.rs` | `runie-tui/tests/` | TUI E2E |
| `tui/src/tests/login_flow_form.rs` | `runie-tui/tests/` | TUI form tests |

`consolidate-login-flow-handlers` moves `update/login_flow.rs` → `login_flow/handlers.rs`. This task completes the consolidation: move the `event/login_flow.rs` alias and the loose `tests/login_logout/login_flow.rs` test file into `login_flow/` so the entire feature lives in one directory.

## Acceptance Criteria

- [ ] `event/login_flow.rs` alias folded into `event/aliases.rs` (or `event/mod.rs`) per `simplify-event-module-layout`.
- [ ] `tests/login_logout/login_flow.rs` moved into `login_flow/` (as `login_flow/e2e_tests.rs` or inline with `login_flow/tests.rs`).
- [ ] `login_flow/` contains: `mod.rs`, `state.rs`, `validation.rs`, `panels.rs`, `handlers.rs`, `tests.rs` (or fewer if tests are inlined).
- [ ] No login-flow logic lives outside `login_flow/` except TUI rendering tests (which stay in `runie-tui`).
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `login_flow_state_tests_pass_in_new_location` — existing state tests pass after move.

### Layer 2 — Event Handling
- [ ] `login_flow_handlers_pass` — handler tests (from `consolidate-login-flow-handlers`) pass.

### Layer 3 — Rendering
- [ ] `tui_login_flow_tests_pass` — `runie-tui/src/tests/login_flow_*.rs` pass unchanged.

### Layer 4 — Smoke / Crash
- [ ] `login_logout_e2e_suite_passes` — the full login/logout E2E suite passes after consolidation.

## Files touched

- `crates/runie-core/src/event/login_flow.rs` → delete (fold into aliases)
- `crates/runie-core/src/tests/login_logout/login_flow.rs` → move to `crates/runie-core/src/login_flow/`
- `crates/runie-core/src/login_flow/mod.rs` — update module declarations
- `crates/runie-core/src/login_flow/tests.rs` — absorb moved tests (or new `e2e_tests.rs`)
- `crates/runie-core/src/event/mod.rs` — remove `mod login_flow;`
- `crates/runie-core/src/tests/login_logout/` — cleanup after move

## Notes

Depends on `consolidate-login-flow-handlers` (which moves the handler file first). The TUI tests (`runie-tui/src/tests/login_flow_*.rs`) stay in tui because they test rendering, not domain logic. After this task, `rg "login_flow" crates/runie-core/src` should return hits only inside `login_flow/` and `event/aliases.rs`.
