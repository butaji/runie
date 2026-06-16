# Adopt Domain DSLs for Declarative Code

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: (none)
**Blocks**: (none)

## Description

Introduce declarative DSLs (builder patterns + macro-based) for complex domain areas to reduce boilerplate and improve readability:

### 1. Event DSL — Declarative event definitions

```rust
// Instead of manual enum + serialization:
define_event!(TurnStarted {
    turn_id: u64,
    agent_id: String,
});

// Generates: enum, impl Event, serialization
```

### 2. Tool DSL — Declarative tool definitions

```rust
// Instead of manual trait implementation:
define_tool!(ReadFile {
    name: "read",
    description: "Read file contents",
    params: {
        path: PathBuf,
    },
    async fn execute(ctx: ToolCtx, path: PathBuf) -> Result<String, ToolError> {
        tokio::fs::read_to_string(&path).await.map_err(Into::into)
    }
});
```

### 3. Command DSL — Declarative command registration

```rust
// Instead of manual match arms:
define_command!(list_files, "list files", KeyBindings { ctrl: 'l' });
define_command!(toggle_sidebar, "toggle sidebar", KeyBindings { ctrl: 'b' });
```

### 4. Hook DSL — Declarative hook registration

```rust
// Instead of manual hook registration:
define_hook!(PreToolUse, "before_tool", |ctx| {
    tracing::info!("Tool: {}", ctx.tool_name);
    HookResult::Allow
});
```

### 5. Policy DSL — Declarative permission policies

```rust
// Instead of manual policy implementation:
define_policy!(GitTrackedWrite, {
    matches: |ctx| ctx.tool == "write" && is_git_tracked(ctx.path),
    action: PermissionResult::Allow,
});
```

Reference: `~/Code/agents/omegacode/` for workflow DSL patterns

## Acceptance Criteria

- [ ] `define_event!` macro for event definitions.
- [ ] `define_tool!` macro for tool definitions.
- [ ] `define_command!` macro for command registration.
- [ ] `define_hook!` macro for hook registration.
- [ ] `define_policy!` macro for permission policies.
- [ ] All existing functionality preserved (macros are syntactic sugar).
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `define_event_generates_correct_enum` — generated code compiles and works.
- [ ] `define_tool_generates_correct_impl` — generated trait impl correct.
- [ ] `define_command_generates_handler` — command handler generated.
- [ ] `define_hook_generates_closure` — hook closure generated.
- [ ] `define_policy_generates_policy` — policy matches correctly.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
N/A.

## Files touched

- `crates/runie-macros/` (new crate)
  - `src/event.rs`
  - `src/tool.rs`
  - `src/command.rs`
  - `src/hook.rs`
  - `src/policy.rs`
  - `src/lib.rs`

## Notes

DSLs reduce boilerplate and enforce consistency. Macros generate type-safe code at compile time.
