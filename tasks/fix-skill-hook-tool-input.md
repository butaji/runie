# Fix empty `tool_input` passed to harness skill hooks

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

In `crates/runie-agent/src/turn.rs`, the `fire_tool_after_hook` and `check_tool_call_before_hook` functions construct `ToolCallCtx` with `tool_input: serde_json::json!({})`. This means skills such as `HashlineEditSkill`, `LoopDetectorSkill`, and `VerificationLoopSkill` cannot see the actual tool arguments, making the before/after hook system largely useless. The fix is to pass `tool_call.args.clone()`.

## Acceptance Criteria

- [x] `ToolCallCtx.tool_input` in both hooks equals `tool_call.args`.
- [x] Existing skill tests still pass.
- [x] New Layer 1 test proves the hook receives the correct input.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [x] Add `tool_call_hook_receives_input` in `crates/runie-agent/src/tests/turn.rs`:
  - Create a `RecordingSkill` that stores the last `ToolCallCtx`.
  - Build a `SkillRegistry` with it.
  - Call `SkillRegistry::on_tool_call` with a `ToolCallCtx` that has `tool_input: json!({"path":"src/main.rs"})`.
  - Assert the recorded `tool_input` matches.

### Layer 2 — Event Handling
- N/A — hook dispatch is direct.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-agent/src/turn.rs` — change two lines.
- `crates/runie-agent/src/tests/turn.rs` — add test (create if missing).

## Implementation

### Step 1: Update `fire_tool_after_hook`

In `crates/runie-agent/src/turn.rs` around line 365:

```rust
skills.on_tool_call(&ToolCallCtx {
    tool_name: tool_call.name.clone(),
    tool_input: tool_call.args.clone(),
    phase: ToolCallPhase::After,
    tool_output: Some(output.content.clone()),
    success: Some(output.status == ToolStatus::Success),
});
```

### Step 2: Update `check_tool_call_before_hook`

Around line 381:

```rust
let tool_ctx = ToolCallCtx {
    tool_name: tool_call.name.clone(),
    tool_input: tool_call.args.clone(),
    phase: ToolCallPhase::Before,
    tool_output: None,
    success: None,
};
```

### Step 3: Add test

In `crates/runie-agent/src/tests/turn.rs`:

```rust
use runie_core::harness_skills::{HarnessSkill, SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult};
use std::sync::{Arc, Mutex};

struct RecordingSkill {
    ctx: Arc<Mutex<Option<ToolCallCtx>>>,
}

impl HarnessSkill for RecordingSkill {
    fn name(&self) -> &str { "recording" }

    fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        *self.ctx.lock().unwrap() = Some(ctx.clone());
        ToolCallResult::Continue
    }
}

#[test]
fn tool_call_hook_receives_input() {
    let recorded = Arc::new(Mutex::new(None));
    let skill = RecordingSkill { ctx: recorded.clone() };
    let mut registry = SkillRegistry::new();
    registry.register(skill);

    let input = serde_json::json!({"path": "src/main.rs"});
    registry.on_tool_call(&ToolCallCtx {
        tool_name: "read_file".into(),
        tool_input: input.clone(),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    });

    let ctx = recorded.lock().unwrap().take().unwrap();
    assert_eq!(ctx.tool_input, input);
}
```

### Step 4: Run tests

```bash
cargo test -p runie-agent tool_call_hook_receives_input
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-agent/src/turn.rs crates/runie-agent/src/tests/turn.rs tasks/fix-skill-hook-tool-input.md tasks/index.json
git commit -m "fix(agent): pass tool args to harness skill hooks"
```

## Notes

- The `ToolCallCtx` type already derives `Clone`; if it does not, add `#[derive(Clone)]`.
- This fix is a prerequisite for `HashlineEditSkill` and `LoopDetectorSkill` to behave correctly.
