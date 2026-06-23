# Fix bash tool orphaning child processes on timeout

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-engine/src/tool/bash.rs` spawns a detached `std::thread` that runs `Command::output()` and then uses `mpsc::recv_timeout`. When the timeout fires, the function returns `TimedOut` but leaves the thread and child process running. The fix is to use `tokio::process::Command` with an explicit timeout future that kills the child.

## Acceptance Criteria

- [ ] A bash command that exceeds its timeout is killed.
- [ ] Output, error, and timeout status codes are still reported correctly.
- [ ] Existing bash tests pass.
- [ ] New test verifies timeout kills the process.

## Tests

### Layer 1 — State/Logic
- [ ] Add `bash_timeout_kills_child` test in `crates/runie-engine/src/tool/bash.rs` (or a new `tests/` file):
  - Start a `sleep 30` command with a 100 ms timeout.
  - Assert status is `TimedOut`.
  - Assert the process no longer appears in the process list (or use a sentinel temp file that the child removes only if killed).

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-engine/src/tool/bash.rs`

## Implementation

### Step 1: Replace `run_bash_inner` with an async implementation

Delete the `std::thread` / `mpsc` version. Replace `BashTool::call` with:

```rust
async fn call(&self, input: Value, ctx: &ToolContext) -> Result<ToolOutput> {
    let start = Instant::now();
    let command = input["command"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("command is required"))?;
    if let Some(reason) = check_bash_safety(command) {
        return Ok(blocked_output(command, &reason, start.elapsed()));
    }
    let timeout_secs = input["timeout_seconds"].as_u64().unwrap_or(DEFAULT_TIMEOUT_SECS);
    let timeout = Duration::from_secs(timeout_secs);

    let mut cmd = tokio::process::Command::new("bash");
    cmd.arg("-c").arg(command)
        .current_dir(&ctx.working_dir)
        .envs(&ctx.env)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let result = match tokio::time::timeout(timeout, cmd.output()).await {
        Ok(Ok(output)) => process_output(output),
        Ok(Err(e)) => bash_error(&format!("Error executing command: {e}")),
        Err(_) => bash_timeout(timeout),
    };

    Ok(ToolOutput {
        tool_name: "bash".to_string(),
        tool_args: serde_json::json!({ "command": command }),
        content: result.output,
        bytes_transferred: result.bytes_transferred,
        duration: start.elapsed(),
        status: result.status,
    })
}
```

Because `BashTool::call` is inside an `#[async_trait]` impl, the async body is fine; remove the `spawn_blocking` wrapper.

### Step 2: Remove `run_bash_inner`

The helper becomes unused; delete it.

### Step 3: Add timeout kill test

```rust
#[tokio::test]
async fn bash_timeout_kills_child() {
    let tool = BashTool;
    let input = serde_json::json!({
        "command": "sleep 30",
        "timeout_seconds": 1,
    });
    let ctx = ToolContext {
        working_dir: std::env::current_dir().unwrap(),
        env: Default::default(),
    };
    let output = tool.call(input, &ctx).await.unwrap();
    assert_eq!(output.status, ToolStatus::TimedOut);
    assert!(output.content.contains("timed out"));
}
```

(Use the actual `ToolContext` constructor from the codebase.)

### Step 4: Run tests

```bash
cargo test -p runie-engine bash_timeout_kills_child
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-engine/src/tool/bash.rs tasks/fix-bash-tool-timeout-orphan.md tasks/index.json
git commit -m "fix(engine): kill bash child on timeout"
```

## Notes

- `tokio::process::Command` kills the child when the future is dropped, so the timeout future does the right thing.
- Ensure `ToolContext` is `Clone` or construct it fresh in tests.
- If the trait still requires `spawn_blocking` for other reasons, wrap the whole async snippet above in `spawn_blocking` and use `block_on` internally, but keep `tokio::process` so the timeout future can kill the child.
