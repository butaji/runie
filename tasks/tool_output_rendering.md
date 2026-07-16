# Tool output rendering

## Objective

Verify that tool calls and outputs render as Grok-style collapsible blocks.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Tool blocks: `◆ List .`, `◆ Creating hello.txt`, `◆ Run pwd ...`.
- Expanded blocks show output inline; collapsed blocks hide it.
- Denied edits show a clear rejection message.

## runie current state

runie renders tool results but not as Grok-style collapsible titled blocks.

## Required runie changes

- Render each tool call as a collapsible block with a title derived from tool name/target.
- Show output inside the block when expanded.
- Support expand/collapse via `Ctrl+O` and `←/→` in scrollback.

## Test scenarios

1. **List dir block**
   - Keys: `type `list files` press Enter allow`
   - Assert: `◆ List|list_dir|Cargo\.toml`

2. **Expand block**
   - Keys: `press Tab Up Ctrl+o`
   - Assert: `Cargo\.toml|src/`

3. **Collapse block**
   - Keys: `press Ctrl+o`
   - Assert: `◆ List`

4. **Edit block**
   - Keys: `type `create hello.txt` allow`
   - Assert: `◆ Creating|Write`

5. **Denied block**
   - Keys: `type `list files` press 4`
   - Assert: `◆ Permission denied|denied`

## Edge / negative cases

- Empty tool output still renders a block.
- Multiline output preserves line breaks.

## Dependencies

- `tool_permissions`
- `turn_lifecycle`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
