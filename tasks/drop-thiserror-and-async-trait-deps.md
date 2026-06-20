# Drop thiserror and async-trait deps

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Two small deps are removable with trivial or zero code change, extending the `drop-small-stdlib-replaceable-deps` rationale:

| Dep | Sites | Replacement |
|-----|-------|-------------|
| `thiserror` | 1 file: `crates/runie-agent/src/subagent.rs:21` | `anyhow` (already used in 56 files) or a manual `enum ... {}` impl with `impl std::error::Error`. One-file dep for one error enum is YAGNI. |
| `async-trait` | 9+ files in `runie-engine` (all tool impls) + `runie-agent/src/emit_approval_sink.rs` | Rust 1.75+ (stable since Dec 2023) supports native `async fn` in traits. Edition 2021 enables it; no edition bump required. Drop `#[async_trait]` macros and the dep. |

`async-trait` is the bigger win: it's a proc-macro dep that adds compile time and obscures the actual trait signatures. Every `#[async_trait]` in `runie-engine/src/tool/*.rs` and the agent sink can become a plain `async fn` in the `Tool` trait. The trait is already the single shared tool abstraction; removing the macro is mechanical.

`thiserror` is the smaller win: removing a 1-site proc-macro dep. Either convert `subagent.rs`'s error enum to a manual `impl Error` (5 lines) or use `anyhow::Error` (the rest of the agent crate already does).

## Acceptance Criteria

- [ ] `thiserror` removed: `rg "thiserror" crates/` returns zero hits; `crates/runie-agent/Cargo.toml` no longer declares it; `Cargo.lock` no longer pulls it.
- [ ] `async-trait` removed: `rg "async_trait" crates/` returns zero hits; no `Cargo.toml` declares it; `Cargo.lock` no longer pulls it.
- [ ] `crates/runie-engine/src/tool/*.rs` `Tool` trait and impls use native `async fn` (no `#[async_trait]` attribute).
- [ ] `crates/runie-agent/src/emit_approval_sink.rs` uses native `async fn` in trait.
- [ ] `crates/runie-agent/src/subagent.rs` error type uses `anyhow` or a manual `impl Error`.
- [ ] MSRV pinned to ≥ 1.75 in `Cargo.toml` (`rust-version = "1.75"` or higher) if not already.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `subagent_error_displays_after_thiserror_drop` — the error enum (manual impl or anyhow) still formats correctly and supports `?` propagation.
- [ ] `tool_trait_async_fn_compiles` — a fake `Tool` impl with `async fn run(...)` compiles and is callable via `.await`.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_tool_runtime_calls_native_async_trait` — a `MockToolRuntime` turn calls a native-async-trait tool and returns its output.
- [ ] `cargo tree --workspace` shows no `async-trait` or `thiserror` edges.

## Files touched

- `crates/runie-agent/src/subagent.rs` (drop `thiserror`, use `anyhow` or manual `impl Error`)
- `crates/runie-agent/src/emit_approval_sink.rs` (drop `#[async_trait]`)
- `crates/runie-engine/src/tool/*.rs` (`bash`, `edit_file`, `fetch_docs`, `find`, `find_definitions`, `grep`, `read_file`, `runtime_adapter`, `write_file` — drop `#[async_trait]`)
- `crates/runie-engine/src/tool/mod.rs` (the `Tool` trait — `async fn` signatures)
- `crates/runie-agent/Cargo.toml`, `crates/runie-engine/Cargo.toml` (remove dep declarations)
- `Cargo.toml` (`rust-version` field if missing)
- `Cargo.lock` (regenerated)

## Notes

Bundled with `drop-small-stdlib-replaceable-deps` (which covers `parking_lot`, `chrono`, `nucleo-matcher`, `glob`) as the same YAGNI / stdlib-replacement posture, but kept separate because `async-trait` removal touches trait signatures (a small API change) rather than call-site swaps. If the `Tool` trait's `async fn` returns a `Box<dyn Future>` currently (via `async_trait`'s desugaring), native `async fn` in traits returns an opaque `impl Future` — callers that stored the future as a `Box<dyn>` need updating; check `runtime_adapter.rs` and `tool_runner.rs`. Edition 2024 (stable Nov 2025) further improves async-trait ergonomics (`async fn` in traits is fully object-safe with `trait_upcasting`); consider bumping `edition = "2024"` in the same change if the rest of the workspace is ready.
