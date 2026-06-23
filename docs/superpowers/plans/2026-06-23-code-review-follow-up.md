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

| # | Task ID | Priority | Files | What to do |
|---|---------|----------|-------|------------|
| 1 | `fix-think-filter-guardrails` | P0 | `crates/runie-agent/src/think_filter.rs`, `crates/runie-agent/src/tests/think_filter.rs` | Split `feed_text` into helpers, move tests out, get under 500/40/10 limits. |
| 2 | `fix-skill-hook-tool-input` | P0 | `crates/runie-agent/src/turn.rs` | Pass `tool_call.args.clone()` into `ToolCallCtx`. |
| 3 | `fix-double-turn-start-call` | P0 | `crates/runie-agent/src/turn.rs` | Call `on_turn_start` once and match the result. |
| 4 | `fix-config-actor-write-errors` | P0 | `crates/runie-core/src/actors/config/actor.rs` | Match inner write `Result`; emit `Event::Error` on failure. |
| 5 | `fix-blocking-file-io-in-async-paths` | P1 | `file_refs.rs`, `path_complete.rs`, `update/tools.rs`, `update/command.rs`, `commands/dsl/handlers/system.rs`, `harness_skills/hashline_edit.rs`, `actors/io/actor.rs` | Use `tokio::fs`, `spawn_blocking`, or `block_in_place_if_runtime`. |
| 6 | `fix-bash-tool-timeout-orphan` | P1 | `crates/runie-engine/src/tool/bash.rs` | Use `tokio::process::Command` + timeout and kill the child. |
| 7 | `fix-render-task-blocking-io` | P1 | `crates/runie-tui/src/main.rs` | Run terminal IO on a dedicated OS thread. |
| 8 | `fix-session-store-load-alignment` | P1 | `crates/runie-core/src/session_store.rs` | Pair key/event in the loop and surface parse errors. |
| 9 | `remove-sleeps-from-automatic-tests` | P1 | Multiple test files | Replace `sleep()` with deterministic sync. |
| 10 | `remove-appstate-cfg-test-branches` | P2 | `crates/runie-core/src/model/state/app_state.rs` | Remove `#[cfg(test)]` branches; tests use normal cache path. |
| 11 | `fix-headless-runtime-spinwait` | P2 | `crates/runie-core/src/headless_runtime.rs` | Await event with timeout instead of spin loop. |
| 12 | `fix-main-unix-epoch-unwrap` | P2 | `crates/runie-tui/src/main.rs` | Non-panicking session ID generation. |
| 13 | `sync-docs-after-code-review` | P2 | `AGENTS.md`, `docs/Architecture.md` | Doc clarifications. |

## Execution order

1. Start with **Task 1** (`fix-think-filter-guardrails`) because it unblocks `cargo build`.
2. Implement **Tasks 2–4** in any order; they are independent correctness fixes.
3. Implement **Tasks 5–8** in any order; they are independent but touch many files.
4. Implement **Task 9** after the behavior fixes are stable so tests can be converted safely.
5. Implement **Tasks 10–12** as cleanup.
6. Finish with **Task 13** (docs).

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
