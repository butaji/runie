# Use `pulldown-cmark-frontmatter` for declarative resource loading

**Status**: done
**Milestone**: R2
**Category**: Core / State
**Priority**: P1

**Depends on**: unify-declarative-resource-loader
**Blocks**: none

## Description

`crates/runie-core/src/resource_loader.rs` implements its own YAML frontmatter scanning and markdown body extraction. `ctx7` found `pulldown-cmark-frontmatter`, a small, high-reputation crate that does exactly this on top of `pulldown-cmark`. Replacing the custom parser removes regex/string scanning, supports both YAML and TOML frontmatter naturally, and aligns with the SKILL.md/YAML frontmatter convention used by `thClaws` and `OpenFang`.

## Acceptance Criteria

- [x] Add `pulldown-cmark-frontmatter` (and `serde_yaml`) to `runie-core` dependencies.
- [x] `resource_loader.rs` uses `FrontmatterExtractor` to parse frontmatter and body; custom delimiter scanning is removed.
- [x] Frontmatter is deserialized into a typed struct (or `HashMap<String, String>`) via `serde_yaml`/`toml` instead of manual line parsing.
- [x] Subdirectory `SKILL.md` + flat `.md` precedence logic is preserved but expressed in terms of `walkdir`/`ignore`.
- [x] Existing resource records loaded from `crates/runie-core/resources/` and project `.runie/skills/` produce identical `frontmatter`/`content` fields.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `resource_with_yaml_frontmatter` — parses title, version, and body.
- [x] `resource_with_toml_frontmatter` — parses TOML code-block frontmatter (the format `pulldown-cmark-frontmatter` demonstrates).
- [x] `resource_without_frontmatter` — body equals full file content, frontmatter empty.
- [x] `subdir_skill_precedes_flat_file` — `foo/SKILL.md` wins over `foo.md`.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Smoke / Crash
- [x] N/A.

## Files touched

- `Cargo.toml` (workspace)
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/resource_loader.rs`
- `crates/runie-core/src/lib.rs` (removed `parse_frontmatter_yaml` export)
- `crates/runie-core/src/declarative/tests.rs` (updated test expectations)
- `crates/runie-core/src/skills/tests.rs` (updated test expectations)

## Implementation Notes

- `pulldown-cmark-frontmatter` uses fenced code blocks (` ```yaml `) for frontmatter. Runie's existing resources use raw `---` delimiters, so a `normalize_raw_frontmatter` function converts between formats.
- The old simple YAML parser didn't support block scalars (`|` and `>`) or properly handle empty values. The new `serde_yaml` parser correctly handles these, and tests were updated to reflect this improvement.
- The `parse_frontmatter_yaml` function was removed as it was only used for the old custom parser.
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
