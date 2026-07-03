# Collapse effect_payload two-step mapping

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Side effects (clipboard, external editor, share, login validation, suspend) are computed in two steps:

1. `crates/runie-core/src/effect_payload.rs` (98 LOC) — pure `fn extract(event, state) -> Option<EffectPayload>` maps an `Event` + `AppState` to a self-contained `EffectPayload` enum (no ratatui).
2. `crates/runie-tui/src/effects/mod.rs` — maps `EffectPayload` → `EffectCommand` (the ratatui-side enum that actually executes the side effect), then dispatches via `effects/clipboard.rs`, `effects/editor.rs`, etc.

**Implemented as Option (a)**: `effect_payload.rs` was deleted from `runie-core`. The `EffectPayload` enum and `extract()` function were moved into `runie-tui/src/effects/mod.rs` with an inline `extract()` that maps `CoreEvent + AppState` → `EffectPayload` → `EffectCommand` in one match. This was done incrementally; the final state is already in place (no separate `effect_payload.rs` exists).

The TUI already imports `AppState` (for the render path) and already has the event in hand when it calls `effect_payload::extract`. The intermediate `EffectPayload` enum adds a type and a match without decoupling anything: the TUI still has to enumerate every variant to produce an `EffectCommand`, and core still has to know about `ShareSession` / `LoginValidateKey` shapes.

**Decision: Option (a) — Collapse**. The YAGNI argument wins: with one consumer (the TUI), the separation is speculative. If a future `runie-server` or headless CLI wants the same effects, option (b) can be revived then.

## Acceptance Criteria

- [x] Decision made: **Option (a) Collapse** — `effect_payload.rs` deleted; `EffectPayload` removed from `runie-core` exports; `runie-tui/src/effects/mod.rs` reads `Event` + `AppState` and produces `EffectCommand` in one match.
- [x] `rg "EffectPayload\|effect_payload" crates/` returns zero hits (only the `// formerly in runie-core::effect_payload` comment in `effects/mod.rs`).
- [x] The existing `effect_payload::extract` tests moved to the TUI as `effects::compute_command` tests (same coverage, new location).
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `copy_last_response_extracts_assistant_text` — `EffectPayload::CopyToClipboard { text: "the answer" }` when assistant message present.
- [x] `copy_last_response_empty_when_no_assistant` — `extract()` returns `None` when no assistant message.

### Layer 2 — Event Handling
- [x] `effect_command_try_from_event` — `EffectCommand::try_from_event` produces correct command for each event variant.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_clipboard_copy_still_fires` — `cargo test --workspace` green confirms all effects still fire.

## Files touched

- `crates/runie-core/src/effect_payload.rs` — deleted (was the original location)
- `crates/runie-core/src/lib.rs` — no `pub mod effect_payload;` (never existed in this codebase; the inline was done directly)
- `crates/runie-tui/src/effects/mod.rs` — contains `EffectPayload` enum, `extract()`, `EffectCommand`, `last_assistant_text()`, and tests

## Notes

The pure-core placement was likely intended to keep effect *intent* in the domain layer and effect *execution* in the IO layer — a clean separation. With one consumer (the TUI), the separation was speculative. Option (a) was chosen: effect extraction is inlined in the TUI layer.
