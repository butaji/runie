# Code Review Follow-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve all ranked findings from the 2026-06-23 architecture & code review so the workspace builds cleanly and respects the async IO, testing, and guardrail conventions documented in `AGENTS.md` and `docs/Architecture.md`.

**Architecture:** The review found correctness bugs in the agent turn, swallowed config errors, synchronous file IO on the async runtime, resource leaks in the bash tool, and build-guardrail violations. Each finding is tracked as a standalone task in `tasks/`; the tasks can be implemented in parallel except where noted.

**Tech Stack:** Rust, Tokio, Ratatui, redb.

---

## File structure

- `tasks/index.json` — canonical task registry; new entries added for every finding.
- `tasks/fix-*.md` — one detailed task per finding with exact files, code changes, and tests.
- `AGENTS.md` — updated to remove the stale "Current violations: 0" claim.
- `docs/Architecture.md` — updated with concrete async-IO remediation order.

## Task map

| # | Task ID | Priority | Status | Files | What to do |
|---|---------|----------|--------|-------|------------|
| 1 | `fix-think-filter-guardrails` | P0 | done | `crates/runie-agent/src/think_filter.rs` | Already under 500/40/10 limits; verify only. |
| 2 | `fix-skill-hook-tool-input` | P0 | done | `crates/runie-agent/src/turn.rs` | Already passes `tool_call.args.clone()`; verify only. |
| 3 | `fix-double-turn-start-call` | P0 | done | `crates/runie-agent/src/turn.rs` | Already calls `on_turn_start` once; verify only. |
| 4 | `fix-config-actor-write-errors` | P0 | done | `crates/runie-core/src/actors/config/actor.rs` | Already propagates inner errors; verify only. |
| 5 | `fix-blocking-file-io-in-async-paths` | P1 | done | `file_refs.rs`, `path_complete.rs`, `update/tools.rs`, `update/command.rs`, `commands/dsl/handlers/system.rs`, `actors/io/actor.rs` | Already wrapped with `block_in_place_if_runtime` / `spawn_blocking`; verify only. |
| 6 | `fix-bash-tool-timeout-orphan` | P1 | done | `crates/runie-engine/src/tool/bash.rs` | Already uses `tokio::process::Command` + timeout; verify only. |
| 7 | `fix-render-task-blocking-io` | P1 | todo | `crates/runie-tui/src/main.rs` | Run terminal IO on a dedicated OS thread. |
| 8 | `fix-session-store-load-alignment` | P1 | todo | `crates/runie-core/src/session_store.rs` | Pair key/event in the loop and surface parse errors. |
| 9 | `remove-sleeps-from-automatic-tests` | P1 | todo | `fff_indexer/tests.rs`, `runie-provider/src/tests.rs` | Replace remaining `sleep()` calls with deterministic sync. |
| 10 | `remove-appstate-cfg-test-branches` | P2 | done | `crates/runie-core/src/model/state/app_state.rs` | `#[cfg(test)]` branches already removed; verify only. |
| 11 | `fix-headless-runtime-spinwait` | P2 | todo | `crates/runie-core/src/headless_runtime.rs` | Await event with timeout instead of spin loop. |
| 12 | `fix-main-unix-epoch-unwrap` | P2 | todo | `crates/runie-tui/src/main.rs` | Non-panicking session ID generation. |
| 13 | `sync-docs-after-code-review` | P2 | done | `AGENTS.md`, `docs/Architecture.md` | Doc updates already applied; verify only. |

Done tasks are archived under `tasks/archive/`. Remaining open work:

1. Implement **Tasks 7, 8** in any order; they are independent.
2. Implement **Task 9** (remove remaining sleeps).
3. Implement **Tasks 11–12** as cleanup.

## Verification

After every task:

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

The final state must satisfy:

- `cargo build --workspace` passes with zero lint violations.
- `cargo test --workspace` passes.
- No new `sleep()` calls in `#[test]` / `#[tokio::test]` functions.
- No `std::fs` / `std::process` calls on async actor paths without offloading.

## Notes

- Each task file contains the exact file paths, code snippets, and test expectations needed for implementation.
- If a task proves larger than expected, split it into follow-up tasks and update `tasks/index.json`.
