# Crate Replacement Audit

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

Before implementing the R3 architecture and UI work, audit Runie’s custom code
for areas where a maintained crate can do the same job with less code and
better correctness. Use `context7.com` (and crates.io / docs.rs as fallback)
to evaluate candidates.

**Note on ctx7:** The existing `Context7Client` in
`crates/runie-agent/src/context7.rs` currently fails with
`{"error":"validation_error","message":"Library name is required"}` against
`https://context7.com/api/v2/libs/search`. This task includes fixing or
replacing that client so future lookups work.

## Acceptance Criteria

- [ ] Fix or replace `Context7Client` so `fetch("tokio")` returns documentation.
- [ ] Produce `docs/CRATE_DECISIONS.md` (or section in `docs/SPEC.md`) with a
  decision matrix:
  | Area | Custom Code | Candidate Crate | Decision | Rationale |
  |---|---|---|---|---|
  | Markdown rendering | `runie-tui/src/markdown.rs` | `ratatui-markdown` + `pulldown-cmark` | TBD | ... |
  | Syntax highlighting | `runie-tui/src/syntax/` | `syntect` | TBD | ... |
  | Text input | custom input handling | `tui-textarea` | TBD | ... |
  | Spinner | custom Braille spinner | `throbber-widgets-tui` | TBD | ... |
  | Diff | `runie-agent/src/diff.rs`, `runie-tui/src/diff.rs` | `similar` | TBD | ... |
  | Fuzzy matching | `runie-core/src/fuzzy.rs` | `nucleo` | TBD | ... |
  | Channels | `tokio::sync::mpsc` | `flume` | TBD | ... |
  | OpenAI provider | custom `runie-provider/src/openai.rs` | `async-openai` | TBD | ... |
- [ ] For each "adopt" decision, create or link a follow-up task.
- [ ] For each "keep custom" decision, document the reason (usually
  Runie-specific behavior that the crate cannot provide).
- [ ] Update `Cargo.toml` files for any crates approved for immediate adoption.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `context7_client_fetches_tokio_docs` — integration test against context7
  (mocked or live with short timeout).

### Layer 2+3
N/A (planning / documentation task).

## Notes

**Preliminary recommendations (to be validated by audit):**

| Area | Likely Decision | Reason |
|---|---|---|
| Markdown rendering | **Keep custom** or **hybrid** | Tool-call interleaving is Runie-specific; may use `pulldown-cmark` for parsing but keep custom rendering. |
| Syntax highlighting | **Adopt `syntect`** | Sublime grammars are far more complete than custom keyword files. |
| Text input | **Evaluate `tui-textarea`** | Widely used, supports history/undo/multi-line; may replace custom input handling. |
| Spinner | **Keep custom** | Existing Braille spinner is 10 lines; crate adds dependency without clear win. |
| Diff | **Adopt `similar`** | Proven diff library, gives unified/line diffs and good testability. |
| Fuzzy matching | **Adopt `nucleo`** | Helix’s matcher, high performance, supports fuzzy scoring. |
| Channels | **Keep `tokio::sync`** | Already using tokio; flume is great but adds no needed feature. |
| OpenAI provider | **Keep custom** | Need one `LLMEvent` adapter for 35+ providers; `async-openai` only covers OpenAI shape. |

**Files touched:**
- `crates/runie-agent/src/context7.rs` (fix client)
- `Cargo.toml` workspace / crate files (if adopting crates)
- `docs/CRATE_DECISIONS.md` (new)

**Out of scope:**
- Actually replacing custom code unless the audit explicitly approves it
  (separate tasks will be created).
