# Use `pulldown-cmark` event stream for tool-marker stripping

**Status**: todo
**Milestone**: R2
**Category**: Tools
**Priority**: P1

**Depends on**: replace-legacy-tool-parsers-with-thin-shim
**Blocks**: none

## Description

`runie-core/src/tool_markers/strip.rs` (and the legacy `markup`/`legacy` modules) use regex and string scanning to remove tool-call markers from assistant output before displaying it. `pulldown-cmark` already parses the same markdown stream and can emit events; walking that stream lets us strip or rewrite tool-call fences in one pass, with correct handling of nested code blocks, escaping, and line boundaries. `goose` and `jcode` rely on `pulldown-cmark` for markdown processing; Runie should too.

## Acceptance Criteria

- [ ] `tool_markers/strip.rs` is rewritten to operate on `pulldown-cmark::Event` instead of regex.
- [ ] The stripper removes tool-call fences (e.g. `<tool_name>`, XML tags, `<parameter>` blocks) and emits clean markdown.
- [ ] `strip_empty_code_fences` guardrail is enforced by the event walker (skip empty code blocks).
- [ ] Legacy `tool/markup/` and `tool/legacy/` modules are removed after call sites migrate.
- [ ] Provider-specific MiniMax XML parsing stays separate but consumes the same stripped markdown instead of re-stripping.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `strip_removes_tool_fence` — tool-only marker block disappears.
- [ ] `strip_preserves_normal_code_block` — triple-backtick code is untouched.
- [ ] `strip_empty_fences_removed` — empty code fences do not leak.
- [ ] `strip_handles_nested_markers` — nested tool tags are removed.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] `rendered_output_has_no_tool_tags` — TUI buffer assertions after strip.

### Layer 4 — Smoke / Crash
- [ ] N/A.

## Files touched

- `crates/runie-core/src/tool_markers/mod.rs`
- `crates/runie-core/src/tool_markers/strip.rs`
- `crates/runie-core/src/tool/shim.rs`
- `crates/runie-core/src/tool/legacy/` (delete)
- `crates/runie-core/src/tool/markup/` (delete)
- `crates/runie-core/Cargo.toml`
- provider parsers that call the stripper

## Notes

- If the current markers are not valid CommonMark, `pulldown-cmark` will emit them as raw text. A tiny pre-normalizer can convert non-standard tags into HTML comments or code blocks that the event walker can then drop.
- Keep the public `strip_tool_markers(&str) -> String` API unchanged so provider parsers do not need to change.
