# Migrate subagent loader to unified resource loader

**Status**: todo
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: none

## Description

`crates/runie-core/src/subagents/mod.rs:203-251` re-implements its own frontmatter/body scanner instead of using the shared `resource_loader.rs` (or the `pulldown-cmark-frontmatter` version). Migrate subagent type files to the same loader used by skills and declarative resources.

## Acceptance Criteria

- [ ] Delete the custom frontmatter/body scanner in `subagents/mod.rs`.
- [ ] Load subagent types via the shared resource loader.
- [ ] Preserve subdirectory `AGENT.md` precedence and built-in embedded agents.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `subagent_loads_via_resource_loader` — a subagent markdown file parses correctly through the shared loader.
- [ ] `builtin_subagents_still_load` — embedded built-in subagents still parse.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/subagents/mod.rs`
- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/subagents/manifest.rs`

## Notes

- `subagents/mod.rs` is already close to the 500-line file limit; this change may require splitting the module.
- This task depends on the frontmatter loader unification.
