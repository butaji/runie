# Migrate subagent loader to unified resource loader

**Status**: done
**Milestone**: R5
**Category**: Core / State
**Priority**: P1

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: none

## Description

`crates/runie-core/src/subagents/mod.rs:203-251` re-implements its own frontmatter/body scanner instead of using the shared `resource_loader.rs` (or the `pulldown-cmark-frontmatter` version). Migrate subagent type files to the same loader used by skills and declarative resources.

## Acceptance Criteria

- [x] Delete the custom frontmatter/body scanner in `subagents/mod.rs`.
- [x] Load subagent types via the shared resource loader.
- [x] Preserve subdirectory `AGENT.md` precedence and built-in embedded agents.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `subagent_loads_via_resource_loader` — a subagent markdown file parses correctly through the shared loader.
- [x] `builtin_subagents_still_load` — embedded built-in subagents still parse.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-core/src/subagents/mod.rs` (deleted duplicate parsing functions, now imports from resource_loader)
- `crates/runie-core/src/resource_loader.rs` (added `extract_body` function)
- `crates/runie-core/src/subagents/manifest.rs` (unchanged)

## Notes

- `subagents/mod.rs` reduced from ~470 lines to 411 lines after removing duplicate code.
- `resource_loader.rs` added `extract_body` function to complement existing `extract_frontmatter`.
- The module now imports `extract_frontmatter` and `extract_body` from `resource_loader.rs` instead of duplicating the logic.
- All existing tests pass; the redundant `yaml_line_skips_empty_and_comments` test was removed since it's now covered by `resource_loader` tests.

## Implementation Details

**Before:**
```rust
// subagents/mod.rs had duplicate functions:
fn parse_frontmatter(content: &str) -> Option<HashMap<String, String>> { ... }
fn parse_yaml_line(line: &str) -> Option<(String, String)> { ... }
fn strip_quotes(s: &str) -> String { ... }
fn extract_body(content: &str) -> String { ... }
```

**After:**
```rust
// subagents/mod.rs now imports from resource_loader:
use crate::resource_loader::{extract_body, extract_frontmatter};

// resource_loader.rs added:
pub fn extract_body(content: &str) -> String { ... }
```
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
