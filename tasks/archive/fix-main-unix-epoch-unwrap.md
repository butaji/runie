# Fix `main.rs` panic on pre-epoch system clock

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/main.rs:45` uses `duration_since(UNIX_EPOCH).unwrap()` to generate a session ID. If the system clock is ever set before the Unix epoch, this panics. The fix is to use a non-panicking fallback.

## Acceptance Criteria

- [ ] Session ID generation never panics, even if the clock is before 1970.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] Add a unit test for the session ID helper:
  - Test that the generated ID starts with `session_` and contains only digits.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-tui/src/main.rs`

## Implementation

### Step 1: Extract a helper and replace unwrap

Replace the inline `duration_since(...).unwrap()` with:

```rust
fn generate_session_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_else(|e| {
            // Clock is before the epoch; use the unsigned magnitude as a fallback.
            e.duration().as_nanos()
        });
    format!("session_{}", nanos)
}
```

Then in `spawn_session_persistence`:

```rust
let session_id = generate_session_id();
```

### Step 2: Add test

At the bottom of `main.rs` (or in a new `tests/main.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::generate_session_id;

    #[test]
    fn generate_session_id_returns_numeric_id() {
        let id = generate_session_id();
        assert!(id.starts_with("session_"));
        let suffix = &id["session_".len()..];
        assert!(suffix.chars().all(|c| c.is_ascii_digit()));
    }
}
```

### Step 3: Run tests

```bash
cargo test -p runie-tui generate_session_id
cargo test --workspace
```

### Step 4: Commit

```bash
git add crates/runie-tui/src/main.rs tasks/fix-main-unix-epoch-unwrap.md tasks/index.json
git commit -m "fix(tui): avoid panic in session ID generation on pre-epoch clock"
```

## Notes

- This is a defensive fix; the fallback value is still unique enough for a session ID because it is based on nanosecond time.
- If `generate_session_id` is in `main.rs` and `main.rs` is a binary, the `#[cfg(test)]` tests compile only with `cargo test`.
