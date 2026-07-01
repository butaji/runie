# Compare tool execution and permission flows and fix gaps

**Status**: todo
**Milestone**: R7
**Category**: Tools
**Priority**: P0

**Depends on**: build-runie-vs-grok-build-comparison-harness, fix-tui-permission-dialog-key-input-routed-to-input
**Blocks**: none

## Description

Run prompts that invoke bash/file tools in both Grok Build and Runie. Compare permission UX (dialog clarity, `y`/`n`/`a`, always-approve), tool output rendering, and error handling. Fix Runie gaps with unit + E2E tests.

## Scenario Set

1. Bash tool: `"run echo hi"`.
2. File read: `"read src/lib.rs"`.
3. File write: `"write a comment to src/lib.rs"`.
4. Auto-approve mode: Grok `--always-approve` vs Runie trust rules.
5. Denied tool and recovery.

## Acceptance Criteria

- [ ] Each scenario runs in both tools.
- [ ] Permission dialog behavior is documented side-by-side.
- [ ] Runie permission dialog is answerable and respects user decisions.
- [ ] User `/trust` rules affect tool execution.
- [ ] Actionable findings become tasks with unit + E2E + live tmux AC.
- [ ] `cargo test --workspace` passes after fixes.

## Tests

### Layer 2 — Event Handling
- [ ] `permission_dialog_y_allows_once` — `y` grants and dialog closes.
- [ ] `trust_rule_skips_dialog` — a configured allow rule bypasses the dialog.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `harness_bash_tool_parity` — both tools execute or deny `echo hi` cleanly.

## Files touched

- `crates/runie-core/src/actors/permission/ractor_permission.rs`
- `crates/runie-core/src/update/input/mod.rs`
- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-agent/src/actor.rs`

## Fixture / Replay Strategy

Use recorded Grok Build fixtures for tool/permission scenarios. The recorder should capture both headless (`--always-approve` vs default) and TUI permission dialog panes. Runie tests replay these fixtures; do not invoke live Grok Build from `cargo test` or CI.

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- Overlaps with `fix-tui-permission-dialog-key-input-routed-to-input` and `wire-user-permission-rules-into-agent-gate`.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
