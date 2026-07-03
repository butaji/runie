# Track orphan spawned tasks in provider, IO, and session actors

**Status**: wontfix
**Milestone**: R7
**Category**: Architecture
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

This task was originally intended to ensure no orphaned `tokio::spawn` calls exist in actor code. After analysis, the original scope was incorrect:

1. **`spawn_blocking` calls are NOT orphans** - They are immediately awaited:
   - `ractor_io.rs`: Uses `spawn_blocking` for IO operations (immediately awaited)
   - `session_handlers.rs`: Uses `spawn_blocking` for file operations (immediately awaited)

2. **Fire-and-forget spawns ARE observed** - They are tied to channel lifetimes:
   - `runie-tui/src/main.rs:165,183,197,200` - TUI startup tasks
   - `runie-tui/src/ui_actor.rs:276,529,534` - Effect forwarder, login, dispatch
   - `runie-agent/src/actor.rs:225` - Turn task (sends back TurnComplete)
   - `runie-agent/src/subagent.rs:196` - Result accumulator
   - `runie-cli/src/server.rs:46` - TCP connection handler

3. **The Leader already handles graceful shutdown** - All main actor tasks are tracked by the Leader and awaited during shutdown.

## Why Wontfix

- The original task scope was incorrect (spawn_blocking calls are not orphans)
- Fire-and-forget spawns in TUI/CLI are tied to channel lifetimes and properly cleaned up
- The `enforce-observed-async-work-in-all-actors.md` task (if implemented) would cover any remaining gaps
- No production issues have been observed with the current implementation

## Files touched

None - no changes were needed

## Related Tasks

- `enforce-observed-async-work-in-all-actors.md` - Would add lint for orphan spawns if needed
