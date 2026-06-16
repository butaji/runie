# Harness Skill: Startup Context Injector

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: harness-skill-framework
**Blocks**: (none)

## Description

Before the agent turn starts, run a small set of discovery commands (`pwd`, `ls`, tool detection, `git status`, Python/Node detection) and inject a compact summary into the system prompt. This grounds the model in the actual workspace and reduces reasoning errors and timeouts.

## Acceptance Criteria

- [ ] Skill hooks `on_turn_start` to run discovery commands in a sandboxed way.
- [ ] Discovered context is appended to the system prompt sent to the LLM.
- [ ] Commands and timeout are configurable under `[harness.skills.startup_context]`.
- [ ] Skill can be disabled or have its command list overridden.
- [ ] Discovery output is capped to avoid token bloat.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `startup_context_injects_summary` — the system prompt contains discovered cwd.
- [ ] `startup_context_respects_token_budget` — output is truncated to a configured max length.

### Layer 2 — Event Handling
- [ ] `turn_start_runs_discovery_hook` — `on_turn_start` emits discovery events.

### Layer 3 — Rendering
N/A — no direct UI change.

### Layer 4 — Smoke / Crash
- [ ] `smoke_startup_context` — run binary in a git repo, verify prompt mentions branch/status.

## Files touched

- `crates/runie-core/src/skills/startup_context.rs`
- `crates/runie-core/src/config.rs`
- `crates/runie-agent/src/turn.rs`

## Notes

- Discovery commands should be read-only and safe (no network, no file writes).
- Caching per session is acceptable; re-run only when cwd changes.
- See `docs/adr/0022-harness-middleware-plugins.md`.
