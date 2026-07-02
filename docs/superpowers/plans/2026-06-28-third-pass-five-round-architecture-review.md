# Third-Pass Five-Round Architecture & Code Review — 2026-06-28

**Goal:** continue Pareto simplification of the codebase with a focus on cross-cutting concerns: errors, tests, telemetry, and crate boundaries. Everything remains event-based with SSOT actors.

This pass builds on the first two reviews (`2026-06-28-five-round-architecture-review.md` and `2026-06-28-second-pass-five-round-architecture-review.md`). It does not duplicate their findings; instead it covers error handling, testing strategy, logging/telemetry, dependency graph, and high-risk coupling.

## Round documents

| Round | Focus | Document |
|-------|-------|----------|
| 1 | Error handling & result types | [`2026-06-28-third-pass-round-1-error-handling.md`](./2026-06-28-third-pass-round-1-error-handling.md) |
| 2 | Testing strategy & fixtures | [`2026-06-28-third-pass-round-2-testing.md`](./2026-06-28-third-pass-round-2-testing.md) |
| 3 | Logging, telemetry & diagnostics | [`2026-06-28-third-pass-round-3-logging-telemetry.md`](./2026-06-28-third-pass-round-3-logging-telemetry.md) |
| 4 | Dependency graph & crate boundaries | [`2026-06-28-third-pass-round-4-crate-boundaries.md`](./2026-06-28-third-pass-round-4-crate-boundaries.md) |
| 5 | Synthesis & execution roadmap | [`2026-06-28-third-pass-round-5-synthesis-roadmap.md`](./2026-06-28-third-pass-round-5-synthesis-roadmap.md) |

## Cross-cutting principles

1. **Typed errors at boundaries:** actor/provider/tool errors should carry kind/structure, not just strings.
2. **Deterministic tests:** no real sleeps, no env mutation without isolation, no network in unit tests.
3. **Observable async:** every actor turn and provider call should be visible in traces.
4. **Pay-as-you-go dependencies:** heavy/optional subsystems live behind feature flags.
5. **Single test-support crate:** `runie-testing` is the shared harness; no duplicate env locks or fixture helpers.

## Sources

- Codebase exploration of `runie-core`, `runie-provider`, `runie-agent`, `runie-tui`, `runie-testing`.
- Survey of `~/Code/agents/{goose,openfang,thClaws,jcode,codex-rs}`.
- `ctx7` lookups for `thiserror`, `tracing-test`, `test-case`, `metrics`, `opentelemetry`.
