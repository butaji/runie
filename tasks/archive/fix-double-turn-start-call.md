# Verify `on_turn_start` is called exactly once

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`check_turn_start` in `crates/runie-agent/src/turn.rs` previously called `skills.on_turn_start(&ctx)` twice. It now calls it once and matches all three result variants (`SkipWithMessage`, `Abort`, `Continue`).

## Acceptance Criteria

- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- N/A — existing behavior is covered by agent turn tests.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-agent/src/turn.rs` — verify only.

## Implementation

No code changes needed. Verify the current implementation at lines 72–98:

```rust
fn check_turn_start(
    skills: &SkillRegistry,
    command: &AgentCommand,
    emit: &EmitFn,
) -> Option<Result<()>> {
    let ctx = TurnStartCtx {
        message: command.content.clone(),
        system_prompt: command.system_prompt.clone(),
        skills_context: command.skills_context.clone(),
    };

    match skills.on_turn_start(&ctx) {
        TurnStartResult::SkipWithMessage(msg) => { ... }
        TurnStartResult::Abort(reason) => { ... }
        TurnStartResult::Continue => None,
    }
}
```

Run verification:

```bash
cargo test --workspace
```

## Notes

- If the double-call is reintroduced, restore the single `match`.
