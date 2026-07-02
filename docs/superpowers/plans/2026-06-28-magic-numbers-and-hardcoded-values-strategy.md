# Magic Numbers & Hardcoded Values Strategy

**Goal:** eliminate unexplained literals, centralize constants, and make tunable values configurable. Every change must be low-effort, high-impact (Pareto).

## Decision framework

When you see a literal, ask:

1. **Is it a UX/formatting glyph?** → `runie-tui::theme::glyphs` (e.g., spinner frames, checkboxes, separators).
2. **Is it a color/opacity value?** → `runie-tui::theme::colors`.
3. **Is it a timeout, capacity, retry count, or size limit?** → crate-level `constants` module if fixed, or config schema if tunable.
4. **Is it user-facing copy?** → `runie-core::ui_strings`.
5. **Is it a file path fragment or version?** → named constant near the consumer.
6. **Is it `0`, `1`, `-1`, `true`, `false`, or a sentinel for a well-known type?** → leave if obvious; otherwise document.

## Centralization rules

| Category | Location | Examples |
|----------|----------|----------|
| Cross-crate HTTP/timeouts/retry | `runie-provider::constants` or `runie-core::provider::constants` | request timeout, connect timeout, retry max attempts |
| Actor channel capacities / timeouts | Per-actor `constants.rs` | leader command capacity, event-bus capacity, shutdown timeout |
| TUI glyphs / frames | `runie-tui::theme::glyphs` | spinner frames, checkboxes, arrows, ellipsis |
| TUI colors / opacity | `runie-tui::theme::colors` | diff opacity, selection opacity |
| TUI layout sizes | `runie-tui::theme::layout` or `ui::layout` | popup min sizes, margins, max suggestions |
| Tool result / search limits | `runie-agent::tool::constants` | default match limits, max depth, snippet sizes |
| Byte / duration formatting | `runie-core::tool::format::constants` | KB/MB/GB thresholds, sub-minute threshold |
| User-facing strings | `runie-core::ui_strings` | placeholders, usage messages, panel titles |
| Config defaults | `Config::default()` / schema | truncation max lines, theme name, model name |

## Tunable vs fixed

- **Tunable:** anything a user or operator might reasonably change (timeouts, capacities, retry policy, limits). Add to `Config` or a dedicated config section.
- **Fixed:** anything that is a UI convention or derived from a protocol/format (glyphs, status-code classification ranges, SSE grammar). Keep as named constants.

## Replacements to use

- `strum` for status-code ranges? No — use named constants or a small `match` helper.
- `duration_str`/`humantime` for config durations if we expose them in TOML.
- `tui-textarea` / `tui-input` for input-field constants (cursor, scrolling) once adopted.
- `textwrap` options for wrap widths.

## Anti-patterns to stop

- Duplicating the same literal in two files (e.g., `120` timeout in provider, model client, factory).
- Using raw numbers for semantic thresholds (e.g., `100` as max grep matches without a name).
- Hard-coding UI strings deep in handlers.
- Computing constants inline instead of naming them (e.g., `50 * 1024` for bytes).

## Testing expectation

- **Unit:** every constant module has a test that values are non-zero/positive where required and that duplicate uses reference the same constant.
- **E2E:** smoke tests still pass with centralized values.
- **Live tmux:** a manual session confirms no visual/layout regressions after moving TUI constants.

## Enforcement

- Add a project convention note to `AGENTS.md`.
- Consider a `clippy::numeric_literal` lint or a custom grep check in CI for new raw literals in non-test code (not blocking initially).
- Review new PRs for unexplained literals.
- Update the build-script comment to match the actual magic-number threshold it enforces.

## Phasing

1. **Phase 1 — quick wins:** HTTP timeouts, retry constants, channel capacities, tool default limits, byte/duration thresholds.
2. **Phase 2 — TUI cleanup:** glyphs, layout constants, colors/opacities.
3. **Phase 3 — strings:** centralize UI copy.
4. **Phase 4 — config:** move tunable values from constants to config schema.
5. **Phase 5 — enforcement:** add lint/convention check.
