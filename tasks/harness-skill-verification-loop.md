# Harness Skill: Verification Loop

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P0

**Depends on**: harness-skill-framework
**Blocks**: (none)

## Description

After the model declares a turn complete, run a configurable verification command (e.g., `cargo test`, `npm test`) and feed failures back to the model for a fix pass. Research (LangChain deepagents-cli) shows this can improve benchmark scores by ~13.7 pp by catching build breaks, missing tests, and incomplete edits.

## Acceptance Criteria

- [ ] Skill hooks `on_turn_end` to inspect the final response and decide whether verification is needed.
- [ ] Verification command is configurable per project via `~/.runie/config.toml` or project-local `.runie/config.toml`.
- [ ] If verification fails, the failure output is appended as a tool result and the agent continues for up to a configured number of fix passes.
- [ ] Skill can be disabled globally or per-session.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `verification_needed_for_code_task` — a task with code edits triggers verification.
- [ ] `verification_failure_appends_tool_result` — failed command output becomes a tool result.
- [ ] `max_fix_passes_respected` — agent stops after the configured number of fix attempts.

### Layer 2 — Event Handling
- [ ] `turn_end_runs_verification_hook` — `on_turn_end` fires and emits verification events.

### Layer 3 — Rendering
- [ ] `verification_status_in_tool_card` — TUI shows "Verifying…" and pass/fail state.

### Layer 4 — Smoke / Crash
- [ ] `smoke_verification_loop` — run binary on a task with a deliberate test failure, verify it retries and fixes.

## Files touched

- `crates/runie-core/src/skills/verification_loop.rs`
- `crates/runie-core/src/config.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-core/src/event/variants.rs`

## Notes

- Default verification command should be empty (skill auto-detects build system when possible) or a sensible fallback.
- Avoid infinite loops: enforce `max_fix_passes` and detect verification-output loops.
- See `docs/adr/0022-harness-middleware-plugins.md`.
