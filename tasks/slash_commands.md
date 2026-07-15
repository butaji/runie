# Slash commands

## Objective

Verify `/` opens the command palette filtered to commands with autocomplete descriptions.

All coverage is black-box: tests drive the compiled `runie-tui` / `runie-cli` binaries inside isolated tmux sessions with a temporary `$HOME`. See `AGENTS.md` for the full isolation contract.


## Grok behavior observed

- Typing `/model` shows `Switch the active model` description.
- Executing a slash command leaves no stray leading `/` in the input box.

## runie current state

runie already supports `/` as a command-palette filter.

## Required runie changes

- No change; add regression coverage for stray-slash behavior.

## Test scenarios

1. **Slash opens palette**
   - Keys: `type `/``
   - Assert: `Commands`

2. **Autocomplete description**
   - Keys: `type `/mod``
   - Assert: `Switch model|/model`

3. **Execute /new**
   - Keys: `type `/new` press Enter`
   - Assert: `New session`

4. **No stray slash**
   - Keys: `type `hello``
   - Assert: use `expect_no_text("/hello")` on captured pane

## Edge / negative cases

- Unknown slash command shows 'No matching commands'.
- Esc cancels slash input and clears palette.

## Dependencies

- `command_palette_navigation`
- `input_composition`

## Acceptance checklist

- [x] All P0 scenarios pass with `AppTest::mock()` (or noted context).
- [x] Edge cases are covered.
- [x] No `sleep()` in resulting Rust tests.
- [x] Tests use `keys::` constants, not raw strings.
- [x] `expect_text`/`expect_no_text` use robust regex alternations.
