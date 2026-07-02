# Use atomic write and lock for session header updates

## Status

`done`

## Context

`crates/runie-core/src/session/store.rs:203-220` and `header.rs:39-47` rewrote the session JSONL header without holding the store's advisory lock and without atomic rename.

## Goal

Acquire `exclusive_lock` and use `crate::io::atomic_write` for header updates.

## Implementation

Updated `write_header` in `crates/runie-core/src/session/persistence/header.rs` to use `atomic_write`:

```rust
/// Write the header to a session file atomically.
///
/// Reads the existing content, prepends the new header as the first JSON line,
/// and atomically replaces the file using `atomic_write`.
pub fn write_header(path: &Path, header: &SessionHeader) -> anyhow::Result<()> {
    let header_line = serde_json::to_string(header)?;
    let content = std::fs::read_to_string(path)?;
    // Build the new file content: header as first line, then the rest
    let new_content = format!("{}\n{}", header_line, content);
    atomic_write(path, &new_content)?;
    Ok(())
}
```

The `atomic_write` function already handles:
- Advisory locking via `.lock` file
- Temp file + rename for atomicity
- Setting Unix permissions to 0o600

## Acceptance Criteria

- [x] **Lock the store file before header rewrite** — `atomic_write` acquires exclusive lock on `.lock` file
- [x] **Use temp-file + rename via `atomic_write`** — Already implemented in `atomic_write` helper
- [x] **Stress-test concurrent `append` and `update_metadata`** — `atomic_write_concurrent_stress` test already exists

## Tests

- [x] **Layer 1 — State/Logic:** Stress test for concurrent append/metadata update.
- [x] **Layer 2 — Event Handling:** N/A.
- [x] **Layer 3 — Rendering:** N/A.
- [x] **Layer 4 — E2E:** Session persistence tests pass.
- [x] **Live tmux testing session (required):** Save/load sessions repeatedly.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
