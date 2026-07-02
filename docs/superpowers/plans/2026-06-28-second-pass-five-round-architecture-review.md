# Second-Pass Five-Round Architecture & Code Review — 2026-06-28

**Date:** 2026-06-28  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Everything stays event-based with SSOT actors for state.

This is a deeper, implementation-focused review built on top of the first five-round review (`2026-06-28-five-round-architecture-review.md`). It focuses on concrete code smells, custom code that can be replaced with crates/libs/OS features, and cross-cutting simplification opportunities.

## Round documents

| Round | Focus | Document |
|-------|-------|----------|
| 1 | Event protocol & serialization | [`2026-06-28-second-pass-round-1-event-protocol-and-serialization.md`](./2026-06-28-second-pass-round-1-event-protocol-and-serialization.md) |
| 2 | Provider abstraction & network/retry | [`2026-06-28-second-pass-round-2-provider-and-retry.md`](./2026-06-28-second-pass-round-2-provider-and-retry.md) |
| 3 | Tool execution & MCP integration | [`2026-06-28-second-pass-round-3-tools-and-mcp.md`](./2026-06-28-second-pass-round-3-tools-and-mcp.md) |
| 4 | TUI input, rendering & event loop | [`2026-06-28-second-pass-round-4-tui-rendering.md`](./2026-06-28-second-pass-round-4-tui-rendering.md) |
| 5 | Config, sessions, persistence & integration roadmap | [`2026-06-28-second-pass-round-5-config-sessions-persistence.md`](./2026-06-28-second-pass-round-5-config-sessions-persistence.md) |

## Pareto prioritization

All decisions must be low-effort, high-impact. See [`2026-06-28-second-pass-pareto-prioritization.md`](./2026-06-28-second-pass-pareto-prioritization.md) for the ranked quick-win list and execution order.

## Cross-cutting principles

1. **Derive, don't write:** use `strum`, `derive_builder`, `serde` attributes, and `schemars` to eliminate hand-maintained tables and builders.
2. **One vocabulary:** `Event` and `ProviderEvent` should be the single source for durable storage, headless output, and UI updates.
3. **Centralize HTTP:** one `reqwest` client builder, one retry policy, one error classifier.
4. **No shell-outs for file/text work:** use `walkdir`, `ignore`, `regex`, `tree-sitter`, `similar`.
5. **Route UI changes through events:** even inside the UI actor, state mutations should come from events, not direct field writes.
6. **JSONL as canonical persistence:** avoid SQLite, use snapshot + append-only journal, atomic writes, and replay via events.

## Sources

- Codebase exploration of `runie-core`, `runie-provider`, `runie-agent`, `runie-tui`.
- Survey of `~/Code/agents/{goose,jcode,codex-rs,openfang,thClaws,claw-code}`.
- `ctx7` lookups for `textwrap`, `backon`, `similar`, `walkdir`, `ignore`, `schemars`, `tree-sitter`, `tui-input`, `tui-widgets`, `linkme`.
