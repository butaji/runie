# Wire or remove dead config fields and stub tools

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Security
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Three config-shaped features are parsed but never wired to runtime, plus 5 subagent tool names are parser-recognized but unimplemented:
- `Config.fallback_providers` / `provider_chain()` — called only in `config/tests/mod.rs:275`; production `AgentActor::run_turn` builds one provider and emits `Error`+`Done` on failure, no fallback iteration.
- `Config.permissions.mode` (`PermissionsConfig`) — every production site uses `PermissionManager::default()` (Auto); `PermissionMode::Yolo`/`Manual` branches in `PermissionManager::check` are dead in production; the TOML field is parsed then dropped.
- 5 subagent tool names (`list_subagents`, `cancel_subagent`, `steer_subagent`, `get_subagent_status`, `get_subagent_output`) listed in `tool_parser/minimax.rs:178` `is_known_tool()` with no `impl Tool` — `EngineToolRuntime::execute` returns "unknown tool" at runtime.

## Acceptance Criteria

- [ ] `fallback_providers`: either `AgentActor::run_turn` iterates the chain on provider failure, OR the field + `provider_chain()` are removed and tests updated.
- [ ] `permissions.mode`: either production sites read the configured mode, OR the field + `Yolo`/`Manual` branches are removed (keep `Auto` as the only mode).
- [ ] 5 subagent tools: either `impl Tool` added for each in `runie-engine`, OR the names removed from `is_known_tool()`.
- [ ] No parsed-then-discarded config field remains.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] (wire) `fallback_chain_advances_on_failure` or (remove) `no_fallback_providers_field`.
- [ ] (wire) `permission_mode_yolo_allows_all` or (remove) `only_auto_permission_mode`.
- [ ] (remove) `unknown_subagent_tool_rejected` — parser rejects the 5 names if unimplemented.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_no_silent_unknown_tool` — an unimplemented tool name surfaces a clear error, not a stub success.

## Files touched

- `crates/runie-core/src/config.rs`
- `crates/runie-core/src/provider_registry.rs` or `runie-provider/src/lib.rs`
- `crates/runie-core/src/permissions/mod.rs`
- `crates/runie-agent/src/actor.rs`, `turn_setup.rs`
- `crates/runie-agent/src/tool_parser/minimax.rs`
- `crates/runie-engine/src/tool/` (if implementing)

## Notes

Recommended: remove all three unless a concrete near-term need exists. Parsed-but-dead config misleads users into thinking the feature works.
