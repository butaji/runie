# Header

## Anatomy

The header contains the repository glyph, branch, working directory, and the
event-owned token meter. It is a pure projection of model/status snapshots.

## Contract

- no hard-coded usage values
- theme colors come from tokens
- branch/path spacing is width-aware
- usage is projected from the terminal `Done` event

## Reference

Grok source: `src/app/agent_view/render.rs` and git-info providers.
Runie: `src/bin/runie.rs::render_header`.

## Acceptance

The `Hey` cast matrix checks the header symbol-by-symbol, including blank
padding and fg/bg attributes.
