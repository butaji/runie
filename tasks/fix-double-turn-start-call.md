# Fix double invocation of `on_turn_start`

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`check_turn_start` in `crates/runie-agent/src/turn.rs` calls `skills.on_turn_start(&ctx)` twice in two consecutive `if let` statements. If a skill is stateful or returns `Abort`/`SkipWithMessage`, the second call can observe different state or drop the abort decision. The hook must be called exactly once per turn.

## Acceptance Criteria

- [ ] `skills.on_turn_start` is invoked exactly once in `check_turn_start`.
- [ ] `SkipWithMessage` and `Abort` results are both handled correctly.
- [ ] A test proves the hook is called exactly once even when it returns `Abort`.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] Add `turn_start_hook_called_once` in `crates/runie-agent/src/tests/turn.rs`:
  - Create a counting skill that increments a counter in `on_turn_start` and returns `Abort("no")`.
  - Build a `SkillRegistry`, call `on_turn_start` through `check_turn_start` (or directly via `SkillRegistry::on_turn_start`).
  - Assert counter == 1.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-agent/src/turn.rs` — refactor `check_turn_start`.
- `crates/runie-agent/src/tests/turn.rs` — add test.

## Implementation

### Step 1: Refactor `check_turn_start`

Replace lines 83–94:

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
        TurnStartResult::SkipWithMessage(msg) => {
            emit_response_and_done(emit, &command.id, msg);
            Some(Ok(()))
        }
        TurnStartResult::Abort(reason) => {
            emit_error_and_done(
                emit,
                &command.id,
                format!("Turn aborted by skill: {}", reason),
            );
            Some(Ok(()))
        }
        TurnStartResult::Continue => None,
    }
}
```

### Step 2: Add test

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAbortSkill {
    count: AtomicUsize,
}

impl HarnessSkill for CountingAbortSkill {
    fn name(&self) -> &str { "counting_abort" }

    fn on_turn_start(&self, _ctx: &TurnStartCtx) -> TurnStartResult {
        self.count.fetch_add(1, Ordering::SeqCst);
        TurnStartResult::Abort("no".into())
    }
}

#[test]
fn turn_start_hook_called_once() {
    let skill = CountingAbortSkill { count: AtomicUsize::new(0) };
    let mut registry = SkillRegistry::new();
    registry.register(skill);

    let ctx = TurnStartCtx {
        message: "hi".into(),
        system_prompt: "".into(),
        skills_context: "".into(),
    };
    let _ = registry.on_turn_start(&ctx);

    assert_eq!(
        registry.skills[0].count.load(Ordering::SeqCst),
        1,
        "on_turn_start must be called exactly once"
    );
}
```

(If `SkillRegistry.skills` is private, test via `on_turn_start` returning `Abort` and check no panic; the counter can be exposed through an `Arc<AtomicUsize>`.)

### Step 3: Run tests

```bash
cargo test -p runie-agent turn_start_hook_called_once
cargo test --workspace
```

### Step 4: Commit

```bash
git add crates/runie-agent/src/turn.rs crates/runie-agent/src/tests/turn.rs tasks/fix-double-turn-start-call.md tasks/index.json
git commit -m "fix(agent): invoke on_turn_start exactly once"
```

## Notes

- The function signature and `EmitFn` usage remain unchanged.
- `emit_response_and_done` and `emit_error_and_done` are existing helpers in `turn.rs`.
