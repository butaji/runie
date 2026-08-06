# Thinking block

## States

- streaming: spinner/working indicator
- collapsed: `◆ Thought for N.Ns`
- expanded: dim italic reasoning body
- completed: immutable summary

## Reference

Grok: `src/scrollback/blocks/thinking.rs` and `scrollback/block.rs`.
Runie: `LineKind::ThinkingStatus` and `LineKind::Reasoning`.

Elapsed values must come from event-owned telemetry or deterministic replay,
never wall-clock sleeps in tests.

## Acceptance

`visual-reasoning.yaml` and `visual-reasoning-expanded.yaml` assert glyph,
style modifiers, and fold state.
