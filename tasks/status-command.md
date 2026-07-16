# Status command

## Objective

Add a `/status` slash command that opens a dialog panel showing the current model, provider, thinking level, permission mode, trust state, queued messages, and approximate context usage.

## Agent landscape finding

codex, goose, and gptme have `/status` commands. Runie shows fragments in the status bar but lacks an expanded view.

## runie current state

Runie displays current model and context usage in the status bar. Provider, thinking level, read-only/trust state, and queued messages are partially visible or hidden.

## Required runie changes

- Add `/status` slash command that opens a non-editable dialog panel.
- Include: current provider/model, thinking level, read-only/trust state, permission mode, queued message count, context usage percent.
- Close with `Esc` or `Enter`.

## Test scenarios

1. **Status panel opens**
   - Keys: `type `/status` press Enter`
   - Assert: panel shows `mock/echo` (or current model) and `Provider`.

2. **Status updates after model switch**
   - Keys: switch model, open `/status`.
   - Assert: panel reflects new model.

3. **Status shows permission mode**
   - Keys: enable auto-approve, open `/status`.
   - Assert: panel shows current permission mode.

## Edge / negative cases

- Status panel is read-only; typing does not edit it.
- Status panel closes cleanly and returns focus to chat input.

## Dependencies

- `model_switching`
- `auto-approve-mode`

## Status

**Blocked** — requires runie TUI feature implementation in `runie/crates/runie-tui/`.

The `/status` slash command and its dialog panel are not yet wired in the runie
binary. `tests/status_command.rs` has 5 `#[ignore]` tests waiting for this feature.
This is a **runie source task**, not a tests task.

## Acceptance checklist

- [ ] `/status` slash command registered and opens dialog panel
- [ ] Dialog shows: provider, model, thinking level, permission mode, trust state, queued count, context usage
- [ ] Dialog closes with Esc or Enter
- [ ] All 5 `#[ignore]` tests in `tests/status_command.rs` pass (remove `#[ignore]`)
- [ ] No `sleep()` in resulting Rust tests.
