# Delete the Abandoned `loop_engine` Module in `runie-agent`

**Status**: todo
**Milestone**: R1
**Category**: Core Architecture
**Priority**: P1

## Description

`crates/runie-agent/src/loop_engine/` (6 files, ~1,623 lines) plus
two sibling files `events.rs` (205 lines) and `state.rs` (24 lines)
form a complete alternative agent loop architecture that was
**never wired into the build**. The `lib.rs` of `runie-agent`
declares only the legacy modules (`accumulator`, `context7`, `diff`,
`mutation_queue`, `parser`, `path_utils`, `safety`, `subagent`,
`tools`, `truncate`, `turn`) and not `loop_engine`, `events`, or
`state`. The `loop_engine` code is therefore dead: it compiles only
in isolation, not as part of the workspace.

The same applies to `crates/runie-tui/src/tui.rs` (12k+ lines) and
`crates/runie-tui/src/tui/` directory, which references
`runie_agent::loop_engine::PermissionState` (line 21). That import
would be a compile error if `tui.rs` were wired up.

## What's In the Dead Code

| File | Lines | Purpose |
|---|---|---|
| `crates/runie-agent/src/loop_engine/mod.rs` | 199 | `AgentEventStream`, `AgentLoopConfig`, `AgentLoopError`, `agent_loop()` factory, `classify_error()` |
| `crates/runie-agent/src/loop_engine/run.rs` | 384 | `run_agent_loop()` — main loop entry point |
| `crates/runie-agent/src/loop_engine/streaming.rs` | 308 | SSE streaming, retry config, `start_chat_with_retry()` |
| `crates/runie-agent/src/loop_engine/tools.rs` | 386 | Tool execution with panic recovery, hook integration |
| `crates/runie-agent/src/loop_engine/permissions.rs` | 164 | Permission request/deny/block/add-result workflow |
| `crates/runie-agent/src/loop_engine/permission_state.rs` | 182 | `PermissionState` — shared state for permission flow |
| `crates/runie-agent/src/events.rs` | 205 | `AgentEvent`, `AgentMessage`, `PermissionDecision`, `Hook`, `HookDecision`, `ToolResult`, `AgentTool`, `ContentPart` |
| `crates/runie-agent/src/state.rs` | 24 | `AgentState` — wraps `Session` + `WorkingMemory` |
| **Total** | **~1,852** | |

The dead code also references types that don't exist in the live
codebase:

- `runie_ai::Provider` (workspace doesn't include `runie-ai`)
- `runie_tools::ToolRegistry` (workspace doesn't include
  `runie-tools`)
- `runie_core::{ToolCall as CoreToolCall, ToolSchema, Context,
  AgentMessage, HookDecision}` (these types don't exist in
  `runie-core`)
- `crate::config::AgentConfig` (doesn't exist in `runie-agent`)
- `crate::tools::AgentTool` (doesn't exist in `runie-agent`)

## Verification That It's Truly Dead

```bash
# 1. The modules are NOT declared in lib.rs
grep -n "pub mod" crates/runie-agent/src/lib.rs
# Shows: accumulator, context7, diff, mutation_queue, parser,
#        path_utils, safety, subagent, tools, truncate, turn
# NOT shown: loop_engine, events, state

# 2. No production code references the dead code
git grep -nE 'use runie_agent::(loop_engine|events)' -- 'crates/' ':!crates/_archive/*'
# Only hit: crates/runie-tui/src/tui.rs:21 (which is itself dead)

# 3. The dead code references itself
git grep -nE 'crate::events|runie_ai::Provider' crates/runie-agent/src/loop_engine/
# 6 hits — all self-referential
```

## Why It Was Never Wired Up

`loop_engine` is a more complete agent loop than the live
`turn.rs` (which is 165 lines and uses the legacy `Stream` callback
API). The `loop_engine` design includes:
- Retry with exponential backoff (`RetryConfig` in
  `streaming.rs:12-27`)
- Tool execution with panic recovery
  (`loop_engine/tools.rs:23-50`)
- Permission gating with state machine
  (`loop_engine/permission_state.rs:20-90`)
- Hook system for pre/post hooks
- Richer event types (`AgentEvent` with 11 variants vs the live
  `Event` with 100+ variants in `runie-core/src/event.rs`)

The architecture is sound but the migration is incomplete. The
intended path was likely:
1. Land the new `events.rs` and `state.rs` types
2. Add `pub mod loop_engine;` to `lib.rs`
3. Migrate `runie-term` to use `loop_engine::agent_loop()` instead
   of `run_agent_turn()`
4. Delete the legacy `turn.rs`

None of those steps happened.

## Acceptance Criteria

- [ ] `crates/runie-agent/src/loop_engine/` directory is removed
  (1,623 lines)
- [ ] `crates/runie-agent/src/events.rs` is removed (205 lines)
- [ ] `crates/runie-agent/src/state.rs` is removed (24 lines)
- [ ] `git grep -nE 'loop_engine|use crate::events|use crate::state'`
  in `crates/` returns zero hits (except in `crates/_archive/` if
  those references are intentional)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds (1,569 tests, same as
  before)
- [ ] The deletion is in a single commit with a clear message
  documenting the abandoned refactor

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds with the same test count
  as before the deletion (1,569)
- [ ] `cargo clippy --workspace` has no new warnings

### Layer 4 — Smoke
- [ ] `cargo run -p runie-term --bin runie` starts the TUI without
  panicking (the live `turn.rs` is still wired up)

## Notes

**Why delete rather than wire up:** the `loop_engine` is a
substantial refactor (1,852 lines) that would require:
1. Creating the `runie-ai` and `runie-tools` workspace members
   (each has 1,000+ lines of code that needs `Cargo.toml` files
   written and dependencies reconciled)
2. Writing the missing types in `runie-core` (`ToolCall`,
   `ToolSchema`, `Context`, `Hook`, `HookDecision`, `AgentMessage`,
   `AgentConfig`)
3. Migrating `runie-term` to the new `agent_loop()` factory
4. Re-writing all 30+ test cases in `tests/login_logout.rs` and
   `tests/slash.rs` that depend on the live `Event` types

This is days-to-weeks of work for a feature the user has been
deliberately working on but never finished. The pragmatic call is
to delete the dead code and let the work be picked up later if
anyone wants it (the `git log --diff-filter=D` will preserve the
history).

**The `events.rs` design IS valuable.** It has nicer event types
than the legacy `Event` enum in `runie-core/src/event.rs`:
streaming-friendly, tagged unions, explicit lifecycle. If the
`loop_engine` is ever revived, the new event types can be ported
to the live tree.

**The `state.rs` `AgentState` struct** is also a reasonable
abstraction (wrapping `Session` + `WorkingMemory` + turn counter).
But the references to `runie_core::Message` (which doesn't exist
as a re-export — the live code has `ChatMessage` and a different
provider `Message`) are wrong; this would need reworking to wire
up.

**The `runie-tui::tui` 12k+ line file** also references
`runie_agent::loop_engine::PermissionState` (line 21). But
`runie-tui::lib.rs` doesn't declare `mod tui;` either, so the
`tui.rs` file is itself dead code. Deletion of `loop_engine` does
not affect `tui.rs` (which is also dead). See
`consolidate-tui-tests` for the tui.rs deletion.

**Out of scope:**
- Restoring `loop_engine` (separate "revival" task; would require
  weeks of work)
- Replacing the legacy `Event` enum in `runie-core/src/event.rs`
  with the new `AgentEvent` enum from `events.rs` (the legacy
  enum is the public API surface; touching it is a major
  breaking change)
- Reconciling the parallel event types (`Event` in
  `runie-core/src/event.rs` vs `AgentEvent` in
  `runie-agent/src/events.rs`)
- The `mutation_queue.rs` (299 lines) which is also dead but is
  in the build path (declared in `lib.rs`); see
  `clean-dead-modules`

**Verification:**
```bash
# After deletion
ls crates/runie-agent/src/loop_engine 2>/dev/null && echo "FAIL" || echo "OK"
test -f crates/runie-agent/src/events.rs && echo "FAIL" || echo "OK"
test -f crates/runie-agent/src/state.rs && echo "FAIL" || echo "OK"

# Build clean
cargo build --workspace
cargo test --workspace

# No references remain
! git grep -nE 'loop_engine::|use crate::events::|use crate::state::' \
  -- 'crates/runie-core/' 'crates/runie-term/' 'crates/runie-tui/' \
  'crates/runie-provider/' 'crates/runie-print/' 'crates/runie-json/' \
  'crates/runie-server/' 'crates/runie-agent/src/'
```
