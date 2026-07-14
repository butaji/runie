# Strengthen status-bar assertions

## Objective

Make status-bar tests assert on concrete labels and values instead of weak
substring checks.

## Why this matters

Current assertions like `pane.contains("s") || pane.contains("ms")` or
`pane.contains('/')` pass on almost any pane text and do not actually verify the
status bar.

## Required changes

1. Capture the bottom one or two lines of the tmux pane where the status bar
   lives.
2. Assert on specific fields:
   - Model name (e.g. `mock/echo`, `openai/gpt-4`).
   - Token/context usage (e.g. `12/4k`).
   - State label (e.g. `idle`, `waiting`, `completed`, `error`).
   - Timing format (e.g. `123ms`).
3. Update `tasks/status_bar.md` acceptance criteria to match concrete fields.

## Files to update

- `tests/status_bar.rs`
- `tasks/status_bar.md`

## Dependencies

- `turn_lifecycle`

## Acceptance checklist

- [ ] Each assertion checks a specific status-bar field.
- [ ] Tests fail if the status bar shows wrong model, state, or timing.
- [ ] Tests pass when the status bar is correct.
