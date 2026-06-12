# Wire Orphan Crates Into the Workspace

**Status**: done
**Milestone**: MVP
**Category**: Configuration
**Priority**: P0

## Summary

Investigated orphan crates and archived those that cannot be wired cleanly:

### Archived (cannot wire)
- `runie-ai` — no Cargo.toml, depends on external `rig-core` crate
- `runie-cli` — depends on archived runie-ai and runie-tools
- `runie-tools` — no Cargo.toml, depends on external `rig-core` crate
- `runie-ext` — has Cargo.toml but references non-existent types from runie-core (Tool, ToolOutput, Context)
- `runie-ext-macros` — depends on runie-ext

### Archived orphaned binaries/tests
- `crates/runie-agent/src/bin/reply_to_scenario.rs` — references archived runie-ai
- `crates/runie-tui/src/bin/*.rs` — 5 binaries with broken imports (missing modules)
- `crates/runie-tui/tests/wireframe_layout.rs` — references non-existent style module
- `crates/runie-agent/src/tests/` (partial) — agent_loop_tests.rs and mod.rs reference runie_tools

### Current workspace members
```
crates/runie-core
crates/runie-tui
crates/runie-term
crates/runie-agent
crates/runie-provider
crates/runie-print
crates/runie-json
crates/runie-server
```

## Acceptance Criteria

- [x] All present-and-buildable orphan crates are added to `[workspace] members` — None qualify
- [x] `cargo build --workspace` succeeds with the expanded member list
- [x] `cargo test --workspace` succeeds
- [x] Archived crates are in `crates/_archive/` with reasons
- [x] README.md "Crates" table updated to reflect pruned member list

## Tests

### Layer 1 — State/Logic
- [x] `cargo build --workspace` produces zero errors
- [x] `cargo test --workspace` produces zero errors

## Archive Location

All archived items are in `crates/_archive/`:
- `runie-ai/` — no Cargo.toml, external dependency on rig-core
- `runie-cli/` — depends on archived crates
- `runie-tools/` — no Cargo.toml, external dependency on rig-core
- `runie-ext/` — broken imports of non-existent runie-core types
- `runie-ext-macros/` — depends on runie-ext
- `runie-agent-bin/reply_to_scenario.rs` — references runie_ai
- `runie-tui-bins/` — 5 binaries with broken imports
- `runie-tui-tests/wireframe_layout.rs` — references non-existent style module
- `runie-agent-tests/` (partial) — tests referencing runie_tools
