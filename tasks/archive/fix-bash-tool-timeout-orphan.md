# Verify bash tool kills child on timeout

**Status**: done
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-engine/src/tool/bash.rs` previously spawned a detached `std::thread` and used `mpsc::recv_timeout`, which orphaned the child process on timeout. It now uses `tokio::process::Command` with `tokio::time::timeout` and kills the same `Child` handle on timeout.

## Acceptance Criteria

- [ ] `cargo test -p runie-engine bash_timeout_kills_child` passes.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- Existing `bash_timeout_kills_child` covers this.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-engine/src/tool/bash.rs` — verify only.

## Implementation

No code changes needed. Verify the current implementation at lines 82–114:

```rust
async fn run_bash_inner(...) -> BashResult {
    let mut cmd = tokio::process::Command::new("bash");
    // ...
    let mut child = cmd.spawn().expect(...);
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => collect_output(status, child.stdout.take(), child.stderr.take()).await,
        Ok(Err(e)) => bash_error(...),
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            bash_timeout(timeout)
        }
    }
}
```

Run verification:

```bash
cargo test -p runie-engine bash_timeout_kills_child
cargo test --workspace
```

## Notes

- If the timeout path is refactored again, ensure the same `Child` handle is killed, not a new one.
