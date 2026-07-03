# Actually use pulldown-cmark-frontmatter in resource loader

## Status

`wontfix`

**Reason**: `pulldown-cmark-frontmatter` only supports fenced code-block frontmatter (` ```yaml ... ``` `), not standard YAML frontmatter (`---\n...\n---\n`). The crate cannot replace the manual scanning for standard YAML frontmatter. The current implementation correctly uses `pulldown-cmark-frontmatter` for fenced code blocks and falls back to manual scanning for standard YAML, which is the right architecture given the crate's capabilities.

## Context

`crates/runie-core/src/resource_loader.rs:105-151` still hand-rolls frontmatter scanning (`---\n`, `find("\n---")`, `serde_yaml::Value` → `HashMap<String, String>`). `pulldown-cmark-frontmatter` is already a workspace dependency but unused.

## Goal

Use `pulldown-cmark-frontmatter` + `serde_yaml` typed structs to parse resource frontmatter. Preserve subdirectory priority and skill-frontmatter fallback.

## Acceptance Criteria

- [ ] Replace manual frontmatter scanning with `pulldown-cmark-frontmatter`.
- [ ] Deserialize into typed structs.
- [ ] Preserve subdirectory priority behavior.
- [ ] All resource loader tests pass.

## Design Impact

No change to TUI element design or composition. Only resource loading behavior changes.

## Tests

- **Layer 1 — State/Logic:** Unit tests for frontmatter parsing edge cases.
- **Layer 2 — Event Handling:** Resource-loaded commands produce the same events.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Headless CLI loads skill resources.
- **Live tmux testing session (required):** Slash commands loaded from resources work.

> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
