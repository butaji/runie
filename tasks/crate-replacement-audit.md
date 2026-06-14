# Crate Replacement Audit

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

Before implementing the R3 architecture and UI work, audit Runie’s custom code
for areas where a maintained crate can do the same job with less code and
better correctness. Use `context7.com` (and crates.io / docs.rs as fallback)
to evaluate candidates.

**Note on ctx7:** `ctx7` is the Context7 CLI (`npm i -g ctx7`). It resolves
library names to Context7 IDs and fetches condensed documentation. The
in-repo `Context7Client` in `crates/runie-agent/src/context7.rs` is a separate
piece of code that directly calls the Context7 REST API and is currently
broken (`Library name is required`); that client is not required for this
audit.

## Acceptance Criteria

- [x] Use `ctx7` CLI research (and crates.io / docs.rs fallback) to resolve crate IDs.
- [x] Fetch API examples for each candidate.
- [x] Produce `docs/CRATE_DECISIONS.md` with a decision matrix including Context7
  library IDs.
- [x] For each "adopt" decision, create or link a follow-up task.
- [x] For each "keep custom" decision, document the reason.
- [x] Update `Cargo.toml` files for crates approved for adoption.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `ctx7_library_resolves_syntect` — `ctx7 library syntect` returns
  `/trishume/syntect`.
- [ ] `ctx7_docs_returns_non_empty` — `ctx7 docs /trishume/syntect example`
  returns content.

### Layer 2+3
N/A (planning / documentation task).

## Notes

**ctx7 findings (to be verified and expanded during the task):**

| Area | Crate | Context7 ID | Finding | Preliminary Decision |
|---|---|---|---|---|
| Markdown parser | `pulldown-cmark` | `/pulldown-cmark/pulldown-cmark` | Fast CommonMark/GFM parser, event-based API. | **Adopt** for parsing; keep custom rendering for tool-call interleaving. |
| Syntax highlighting | `syntect` | `/trishume/syntect` | Uses Sublime Text syntax definitions, `HighlightLines` API, default themes. | **Adopt** — far better than custom keyword tokenizers. |
| Diff | `similar` | `/mitsuhiko/similar` | `TextDiff::from_lines`, `ChangeTag`, unified diff, deadline support, dependency-free. | **Adopt** — replaces custom diff logic. |
| Channels | `flume` | `/zesterer/flume` | Multi-producer/multi-consumer, bounded/unbounded, async/sync. | **Keep tokio** — `flume` is great but tokio channels already meet our needs; no clear win. |
| OpenAI provider | `async-openai` | `/64bit/async-openai` | Full OpenAI API client; does not normalize Anthropic/Gemini/etc. shapes. | **Keep custom** — we need one `LLMEvent` adapter for 35+ providers. |
| Text input | `tui-input` | `/sayanarijit/tui-input` | Ratatui/crossterm/termion backends, editing modes, history. Closest match to `tui-textarea`. | **Evaluate** — may replace custom input if undo/history/multi-line fit. |
| Spinner | `throbber-widgets-tui` | (not indexed directly; use `tui-widgets` or cargo search) | Ratatui spinner widget. | **Keep custom** — existing spinner is trivial; crate adds dependency. |
| Fuzzy matching | `nucleo` | (verify via ctx7; cargo shows `nucleo = "0.5.0"`) | Helix matcher, high performance. | **Evaluate** — likely adopt for command palette/model selector. |

**Key ctx7 doc snippets:**

- `syntect` (`/trishume/syntect`):
  ```rust
  let ps = SyntaxSet::load_defaults_newlines();
  let ts = ThemeSet::load_defaults();
  let syntax = ps.find_syntax_by_extension("rs").unwrap();
  let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
  let ranges = h.highlight_line(line, &ps).unwrap();
  ```

- `similar` (`/mitsuhiko/similar`):
  ```rust
  let diff = TextDiff::from_lines(old, new);
  for change in diff.iter_all_changes() {
      let sign = match change.tag() { ChangeTag::Delete => "-", ... };
  }
  ```

- `pulldown-cmark` (`/pulldown-cmark/pulldown-cmark`):
  ```rust
  let parser = Parser::new_ext(markdown, options);
  for event in parser { match event { Event::Text(t) => ... } }
  ```

**Follow-up tasks created:**
- `tasks/adopt-syntect.md`
- `tasks/adopt-similar.md`
- `tasks/adopt-pulldown-cmark.md`
- `tasks/adopt-tiktoken-rs.md`
- `tasks/spike-nucleo-fuzzy.md`

**Files touched:**
- `Cargo.toml` workspace / crate files (if adopting crates)
- `docs/CRATE_DECISIONS.md` (new)
- `crates/runie-agent/src/context7.rs` (optional: fix or deprecate the broken REST client)

**Out of scope:**
- Actually replacing custom code unless the audit explicitly approves it
  (separate tasks will be created).
