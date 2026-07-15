# Model switching

## Objective

Verify that `/model` opens a picker, selects a model, and updates the status indicator.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Grok `/model` shows a list with current model marked: `MiniMax M2.7 (current)`.
- Selecting another model updates bottom-right indicator and shows confirmation.

## runie current state

runie `/model` opens a `Select Model` dialog with recent models and a `▸` selection marker.

## Required runie changes

- No major change; ensure current model is visually marked and status bar updates.

## Test scenarios

1. **Open model picker**
   - Keys: `type `/model` press Enter`
   - Assert: `Select Model`

2. **Arrow down selects**
   - Keys: `press Down`
   - Assert: `▸`

3. **Tab selects**
   - Keys: `press Tab`
   - Assert: `▸`

4. **Select and confirm**
   - Keys: `press Enter`
   - Assert: `mock/echo|mock/list_dir`

5. **Status indicator updates**
   - Keys: `capture pane`
   - Assert: `mock/`

## Edge / negative cases

- Cancel with Esc leaves model unchanged.
- Filtering by typing narrows model list.

## Dependencies

- `command_palette_navigation`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
