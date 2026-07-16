# Context-aware tool output toggle

## Objective

Make `Ctrl+O` context-aware: toggle the selected tool output block when a tool call is focused, otherwise toggle global tool-output density.

## Agent landscape finding

kimi-code uses `Ctrl+O` to toggle tool output. Runie already uses `Ctrl+O` for expand/collapse all posts.

## runie current state

Runie uses `Ctrl+O` as a global expand/collapse toggle for feed posts.

## Required runie changes

- When a tool output block is selected/focused, `Ctrl+O` toggles that block only.
- When no tool block is focused, `Ctrl+O` toggles global tool-output density (collapse/expand all tool calls).
- Keep existing per-post expansion behavior for non-tool posts.

## Test scenarios

1. **Global toggle collapses tool output**
   - Keys: after a tool call is rendered, press `C-o`
   - Assert: all tool output blocks collapse.

2. **Focused toggle expands one block**
   - Keys: navigate to a collapsed tool block, press `C-o`
   - Assert: only that block expands.

3. **Second global toggle expands all**
   - Keys: press `C-o` again.
   - Assert: all tool output blocks expand.

## Edge / negative cases

- Toggle works in vim navigation mode.
- Toggle does not affect assistant prose blocks.

## Dependencies

- `tool_output_rendering`
- `chat_scrollback`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` or replay fixtures.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
