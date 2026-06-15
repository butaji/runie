# Deduplicate Agent Profile Types and I/O

**Status**: done
**Milestone**: R3
**Category**: Core / State
**Priority**: P1

## Description

`crates/runie-core/src/agent_profiles.rs` is explicitly documented as a copy of
`crates/runie-agent/src/profiles.rs` to avoid a dependency cycle. The two modules define
essentially the same `AgentProfile` struct and file-loading logic.

Because `runie-core` is the dependency root, the profile types belong there. `runie-agent`
can consume them directly.

## Acceptance Criteria

- [x] `runie-core/src/agent_profiles.rs` is the canonical implementation.
- [x] `runie-agent/src/profiles.rs` is reduced to a re-export of `runie_core::agent_profiles`.
- [x] Behavioral differences resolved: both implementations now skip invalid files
  (the core behavior already did; the re-export preserves it).
- [x] All callers and tests compile and pass.

## Tests

### Layer 1 — State/Logic
- [x] `agent_profile_round_trip` save/load still works.
- [x] `agent_profile_tool_allow_logic` still works.

### Layer 2 — Event Handling
- [ ] No event changes.

### Layer 3 — Rendering
- [ ] No rendering changes.

## Files touched

- `crates/runie-core/src/agent_profiles.rs`
- `crates/runie-agent/src/profiles.rs`
- `crates/runie-agent/src/lib.rs`
- `crates/runie-core/src/commands/agents_manager.rs`
- `crates/runie-core/src/tests/agent.rs` or other callers.

## Out of scope

- Adding new profile fields.
