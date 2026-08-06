# Runie TUI parity

This directory is the source-of-truth map for reproducing the Grok pager TUI
in Runie. Each component page follows the same contract:

- purpose and anatomy
- source references in `~/Code/agents/grok-build`
- model/state and event inputs
- layout, theme, and animation tokens
- YAML replay and cell-level acceptance criteria

The pages use a component-documentation shape inspired by [shadcn/ui's
component catalogue](https://ui.shadcn.com/docs/components): a small,
composable primitive with explicit states, variants, and implementation notes.

The canonical component index is [`index.json`](index.json).

## Components

- [Layout](layout.md)
- [Header](header.md)
- [Scrollback](scrollback.md)
- [User prompt](user-prompt.md)
- [Thinking](thinking.md)
- [Assistant message](assistant-message.md)
- [Activity group](activity-group.md)
- [Tool card](tool-card.md)
- [Prompt composer](prompt-composer.md)
- [Status/footer](status-footer.md)
- [Theme tokens](theme-tokens.md)
- [Animation](animation.md)
- [Command palette](command-palette.md)
- [Welcome surface](welcome.md)

## Verification

Every page must have at least one YAML scenario under
`crates/runie-tui/tests/e2e/` and, where styling or geometry matters, a
full-screen asciinema/cell oracle. Raw `.cast` files remain authoritative for
terminal escape sequences; `cast_compare --dump` decodes every cell.

Reference frames may use `frame_index` for phase-locked casts, or
`frame_contains` for marker-selected exploratory captures. Strict parity
fixtures should prefer `frame_index` once the reference phase is identified.
