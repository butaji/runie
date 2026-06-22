# Reduce ToolContext Default Environment Capture

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Security
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`ToolContext::default()` captures `std::env::vars()` into every tool invocation. This includes `API_KEY`, `SECRET`, `TOKEN`, and other credentials. Only `multi_agent::redact_secrets` redacts them for subagents; the main tool path and headless modes do not.

## Acceptance Criteria

- [ ] `ToolContext::default()` uses a minimal environment instead of full process env.
- [ ] Only explicitly allowed variables are passed to tool contexts.
- [ ] Subagent redaction remains intact.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `tool_context_default_does_not_contain_secrets` — default context excludes `API_KEY`, `SECRET`, `TOKEN`.
- [ ] `tool_context_allows_explicit_env` — allowed vars are present.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-core/src/tool/context.rs`
- `crates/runie-core/src/multi_agent.rs`

## Notes

This is a privacy/security fix. Consider adding a config allowlist for environment variables that tools genuinely need.
