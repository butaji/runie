# Unify build_system_prompt helpers

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/prompts.rs` defines `build_system_prompt` and `crates/runie-core/src/turn_setup.rs` also defines `build_system_prompt` (plus `build_system_prompt_with_tools`). The turn-setup helper looks up the active prompt template, then calls the prompts-module helper with the same arguments. The name collision is confusing.

## Acceptance Criteria

- [ ] There is one public entry point for system-prompt construction.
- [ ] The turn-setup helper is renamed (e.g., `build_agent_system_prompt`) or its lookup step is merged into the prompts module.
- [ ] All callers are updated.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `build_system_prompt_includes_tools` — system prompt still contains tool descriptions.
- [ ] `build_system_prompt_selects_template` — active template lookup still works.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `headless_turn_system_prompt_unchanged` — provider request contains the same system prompt as before.

## Files touched

- `crates/runie-core/src/prompts.rs`
- `crates/runie-core/src/turn_setup.rs`
- Callers in `runie-agent` and tests.

## Notes

The prompts module is the natural home for the template lookup; turn_setup should only wire configuration.
