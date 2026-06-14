# Tool Block Inline Status

**Status**: todo
**Milestone**: R4
**Category**: TUI / Feed
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Keep Runie's block glyphs, but add Grok-style inline metadata to tool blocks:
duration, downloaded bytes, and a final status icon.

## Acceptance Criteria

- [ ] Running tool blocks show spinner + label + elapsed time
  (`⠴ Run List . 1.8s`).
- [ ] Completed tool blocks show total duration + byte count + status
  (`5.7s ⇣21.2k [✓]` or `[✗]`).
- [ ] Failed tool blocks show `[✗]` and error count.
- [ ] Byte counts are humanized (`21.2k`, `4.93k`).

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn tool_status_line_includes_duration() {
    let line = tool_status_line("List .", 2.5, None, ToolStatus::Running);
    assert!(line.contains("2.5s"));
}

#[test]
fn tool_status_line_formats_bytes() {
    let line = tool_status_line("Read file", 1.0, Some(4930), ToolStatus::Done);
    assert!(line.contains("4.93k"));
}
```

### Layer 3 — Rendering

```rust
#[test]
fn running_tool_block_renders_spinner_and_duration() {
    // TestBackend assertion: tool block row contains spinner char and "2.5s".
}
```

## Files touched

- `crates/runie-core/src/ui/elements.rs` (ToolRunning/ToolDone fields)
- `crates/runie-core/src/ui/transform.rs`
- `crates/runie-tui/src/ui/messages.rs`
- `crates/runie-tui/src/glyphs.rs` (add status icons if missing)

## Out of scope

- Changing the glyph set (keep Runie's).
