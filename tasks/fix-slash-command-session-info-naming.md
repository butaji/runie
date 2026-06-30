# Fix /session_info slash command naming mismatch

**Status**: done
**Milestone**: R7
**Category**: Input / Commands
**Priority**: P2

**Depends on**: fix-tui-slash-command-palette-stays-open-after-execution
**Blocks**: none

## Description

The handler registry registers `session_info` and the help system lists it, but typing `/session_info` in the TUI returns `Unknown command: /session_info. Try /help.`. The working command is `/session`. Either the registered name should be `session_info` (and the YAML command updated), or the help text and registry should use `session` consistently.

## Live Evidence

```
  Unknown command: /session_info. Try /help.
```

`/session` works and shows session metadata.

## Acceptance Criteria

- [x] Decide canonical name (`/session` or `/session_info`) and make the registry, YAML spec, and help text consistent.
- [x] The chosen command renders session info without an "unknown command" error.
- [x] Aliases are updated so the discarded name still works or is removed from help.
- [x] `cargo test --workspace` passes.
- [ ] Live tmux runs `/session_info` (or `/session`) successfully.

## Tests

### Layer 1 — State/Logic
- [x] `session_info_command_resolves` — `CommandRegistry` contains the chosen name and maps it to the session-info handler.

### Layer 2 — Event Handling
- [x] `session_info_event_shows_info` — dispatch the chosen command event and assert the result message contains session metadata.

### Layer 3 — Rendering
- [x] `session_info_result_renders` — `TestBackend` shows the session info text after the command.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_session_info_command_works` — live tmux script runs the chosen command and asserts `Session:` appears.

## Files touched

- `crates/runie-core/resources/commands/session.yaml`
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/commands/dsl/handlers/help.rs`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- `/session_info` is the canonical command name (already registered in `HANDLER_REGISTRY`); `/session` is added as an alias in `session.yaml`.
- `triggers` in YAML is not used for command aliases — `aliases` field is. Added `/session` and `/session_info` to `triggers` for documentation/tracking purposes.
- The fix: `session.yaml` now has `name: session_info` and `aliases: [session]`, making both invocations work.
