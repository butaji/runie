# Track orphan spawned tasks in provider, IO, and session actors

## Status

`partial` (outdated task - needs verification)

## Context

The task description references line numbers that no longer exist. The current state shows:
- `ractor_provider.rs`: No actual `tokio::spawn` calls, only a comment
- `ractor_io.rs`: Uses `spawn_blocking` for IO operations (immediately awaited)
- `session_handlers.rs`: Uses `spawn_blocking` for file operations (immediately awaited)

## Verification Results

All `spawn_blocking` calls in the target files are immediately awaited:
```rust
// ractor_io.rs examples:
let output = match tokio::task::spawn_blocking(move || { ... }).await { ... }
let (git_info, cwd_name) = tokio::task::spawn_blocking(detect_env_sync).await ...

// session_handlers.rs examples:
let _ = tokio::task::spawn_blocking(move || trust.save()).await;
let res = tokio::task::spawn_blocking(move || store.delete(...)).await;
```

These are NOT orphans because:
1. They complete before the function returns
2. They're awaited inline
3. No JoinHandle is discarded

## Actual Orphan Spawns (not in target files)

The actual fire-and-forget spawns are in:
- `crates/runie-tui/src/main.rs:165,183,197,200` - TUI startup tasks
- `crates/runie-tui/src/ui_actor.rs:276,529,534` - Effect forwarder, login, dispatch
- `crates/runie-agent/src/actor.rs:225` - Turn task (sends back TurnComplete)
- `crates/runie-agent/src/subagent.rs:196` - Result accumulator
- `crates/runie-cli/src/server.rs:46` - TCP connection handler

These tasks are spawned at startup and tied to the lifetime of the parent task/channel.

## Recommendation

This task should be closed as `wontfix` or redesigned because:
1. The `spawn_blocking` calls are not orphans
2. Fire-and-forget spawns in TUI/CLI are tied to channel lifetimes
3. The Leader already handles graceful shutdown of main actor tasks

## Acceptance Criteria (if redesigning)

1. **Unit tests** — Static analysis to verify no `let _ = tokio::spawn` patterns.
2. **E2E tests** — Verify clean shutdown with no leaked tasks.
3. **Live run tests** — Inspect process for orphaned tasks after shutdown.

## Tests

### Unit tests
- Static analysis / lint test for orphan spawn patterns.

### E2E tests
- Clean shutdown verification.

### Live run tests
- Process inspection after shutdown.
