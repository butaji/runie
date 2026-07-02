# Fourth-Pass Architecture & Code Review — Backlog Consolidation & Execution Plan

**Goal:** after three horizontal architecture passes and a magic-numbers review, the backlog has grown to ~430 tasks. This pass is a meta-review: consolidate duplicates, resolve contradictions, identify keystone tasks, map dependencies, and produce an executable roadmap.

## Round documents

| Round | Focus | Document |
|-------|-------|----------|
| 1 | Backlog consolidation and duplicate removal | [`2026-06-28-fourth-pass-round-1-consolidation.md`](./2026-06-28-fourth-pass-round-1-consolidation.md) |
| 2 | Keystone tasks and unlock order | [`2026-06-28-fourth-pass-round-2-keystones.md`](./2026-06-28-fourth-pass-round-2-keystones.md) |
| 3 | Dependency map and sequencing | [`2026-06-28-fourth-pass-round-3-dependency-map.md`](./2026-06-28-fourth-pass-round-3-dependency-map.md) |
| 4 | Risk and assumption review | [`2026-06-28-fourth-pass-round-4-risks.md`](./2026-06-28-fourth-pass-round-4-risks.md) |
| 5 | Integrated execution plan | [`2026-06-28-fourth-pass-round-5-execution-plan.md`](./2026-06-28-fourth-pass-round-5-execution-plan.md) |

## Cross-cutting principles

1. **Single backlog:** `tasks/index.json` is the SSOT; per-pass roadmap tasks are merged into one.
2. **Explicit blocked state:** every blocked task must have a `blocked_reason` and `blocked_by` link.
3. **Deduplication contract:** tasks can declare `supersedes` and `duplicates` links.
4. **Keystone-first:** implement tasks that unlock clusters of work before peripheral cleanups.
5. **Ready-for-implementation gate:** no task moves to `in_progress` without a short implementation plan.

## Immediate decisions required

1. **Persistence format:** JSONL is the canonical store (already decided and marked done). The remaining `migrate-session-persistence-to-rusqlite.md` must be closed as `wontfix` or the decision reopened.
2. **MCP scope:** either wire `rmcp` client or remove MCP config scaffolding.
3. **`ProviderProtocol` abstraction:** the existing task is `wontfix`; update docs to reflect that.

## Sources

- Meta-review of passes 1–3 and magic-numbers review.
- Survey of `~/Code/agents/{claw-code,omegacode,gemini-cli,openclaw,thClaws,goose,openfang}` backlog/roadmap patterns.
