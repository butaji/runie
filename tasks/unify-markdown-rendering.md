# Unify markdown rendering across `runie-core` and `runie-tui`

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

Markdown handling is split across two crates with overlapping concerns:

- `crates/runie-core/src/markdown/` — markdown parsing using `pulldown-cmark`.
- `crates/runie-tui/src/markdown.rs` — markdown rendering (turning parsed events into `Span`s for ratatui).

The split mirrors a sensible *parse* vs *render* separation, but `runie-core` is the domain crate and should not own rendering helpers. Per the IO/Domain/UI split in `AGENTS.md`, parsing (data extraction) belongs in domain; rendering (visual projection) belongs in UI. The current placement has the parse-tree plus some presentation logic in the domain crate.

## Acceptance Criteria

- [ ] `runie-core/src/markdown/` exposes only a `ParsedDoc` (events, ranges, plain-text view). No rendering types (`Style`, `Color`, `Span`).
- [ ] `runie-tui/src/markdown.rs` is the only place that converts `ParsedDoc` to ratatui output.
- [ ] The new boundary matches `Architecture.md`'s crate decisions ("pulldown-cmark is the standard, well-tested choice").
- [ ] `cargo test --workspace` passes.
- [ ] Markdown rendering in chat messages still works (live tmux validation: send a message containing `# heading` and `*bold*`, confirm both render).

## Tests

### Layer 1 — State/Logic
- [ ] `parsed_doc_strips_html` — `ParsedDoc::from("# hello\n<script>")` has no `<script>`.
- [ ] `parsed_doc_preserves_line_breaks` — three consecutive newlines produce two paragraph events.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- [ ] `heading_renders_bold` — `Span` for `# hello` has bold style.
- [ ] `code_block_renders_monospace` — `Span` for `` `foo` `` has monospace style.
- [ ] Snapshot test in `runie-tui/src/markdown.rs` covering a representative multi-paragraph message.

### Layer 4 — Smoke / Crash
- N/A.

## Files touched

- `crates/runie-core/src/markdown/` (split: keep parsing, drop rendering helpers)
- `crates/runie-tui/src/markdown.rs` (consume the slimmed-down parse events)

## Notes

- Coordinate with `split-runie-core-into-domain-and-io-crates` (P0) — once the crate split happens, the markdown parser should land in the domain crate and the renderer in the UI crate. This task is the prerequisite cleanup.
