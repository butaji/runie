# Verify harness skill hooks receive `tool_input`

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-agent/src/turn.rs` previously passed an empty `tool_input` to the before/after tool-call hooks. This has been fixed: both `fire_tool_after_hook` (`:369`) and `check_tool_call_before_hook` (`:385`) now pass `tool_input: tool_call.args.clone()`.

A regression test already exists in `crates/runie-agent/src/tests/turn.rs`. This task is verification-only.

## Acceptance Criteria

- [ ] `cargo test -p runie-agent turn::tool_call_hook_receives_input` passes.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- Existing `tool_call_hook_receives_input` covers this.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-agent/src/turn.rs` — verify only.
- `crates/runie-agent/src/tests/turn.rs` — verify only.

## Implementation

No code changes needed. Run verification:

```bash
cargo test -p runie-agent turn::tool_call_hook_receives_input
cargo test --workspace
```

If the test does not exist or fails, restore the fix:

```rust
// In fire_tool_after_hook and check_tool_call_before_hook:
tool_input: tool_call.args.clone(),
```

## Notes

- `HashlineEditSkill`, `LoopDetectorSkill`, and `VerificationLoopSkill` now receive actual arguments.
