# Centralize error types

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

The codebase defines many overlapping error types: `ProviderError`, `LLMError`, `ToolError`, `ToolParseError`, `SubagentError`, `proto::Error`, and pervasive `anyhow::Error`. Several are just `Other(String)` wrappers. `ProviderError` is even re-exported from `runie-provider` as `UnknownProviderError` to avoid a deep dependency.

## Acceptance Criteria

- [ ] Either a single `runie_core::Error` enum with provider/llm/tool/protocol variants is defined, or `anyhow` is standardized with typed downcasts where needed.
- [ ] Duplicate `Other(String)` variants are removed.
- [ ] `runie-provider` no longer needs a re-export alias.
- [ ] `cargo test --workspace` and `cargo check --workspace` pass.

## Tests

### Layer 1 — State/Logic
- [ ] `central_error_displays_preserve_messages` — existing error messages are unchanged.
- [ ] `provider_error_source_round_trips` — provider errors are still identifiable.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_error_propagates` — provider error in a headless turn is still reported.

## Files touched

- `crates/runie-core/src/provider.rs`
- `crates/runie-core/src/llm_event.rs`
- `crates/runie-core/src/tool/mod.rs`
- `crates/runie-core/src/tool_parser/mod.rs`
- `crates/runie-core/src/proto/mod.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-agent/src/subagent.rs`

## Notes

If `anyhow` is chosen, document the convention for downcasting to typed causes.
