# Layout

## Purpose

Compose the full-mode agent view into outer chrome, header, scrollback,
prompt, and status regions without letting widgets invent their own geometry.

## Grok construction

Source: `src/app/agent_view/render.rs`, `src/scrollback/layout.rs`.

The horizontal stack is:

```text
outer pad │ accent(1) │ block pad left(2) │ flexible content │ block pad right(1) │ outer pad
```

Runie owns the equivalent constants in `crates/runie-tui/src/layout.rs`.
The scrollback renderer must measure logical rows before selecting the
autoscroll window.

## States

- full mode
- compact mode
- narrow terminal
- prompt-expanded
- prompt-inline/overlay

## Acceptance

Replay `visual-resize.yaml` and compare all cells at 62×32, 80×24, 100×30,
and 120×36. Geometry mismatch is a failure even when text matches.
