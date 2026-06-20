# Collapse effect_payload two-step mapping

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Side effects (clipboard, external editor, share, login validation, suspend) are computed in two steps:

1. `crates/runie-core/src/effect_payload.rs` (98 LOC) — pure `fn extract(event, state) -> Option<EffectPayload>` maps an `Event` + `AppState` to a self-contained `EffectPayload` enum (no ratatui).
2. `crates/runie-tui/src/effects/mod.rs` — maps `EffectPayload` → `EffectCommand` (the ratatui-side enum that actually executes the side effect), then dispatches via `effects/clipboard.rs`, `effects/editor.rs`, etc.

The TUI already imports `AppState` (for the render path) and already has the event in hand when it calls `effect_payload::extract`. The intermediate `EffectPayload` enum adds a type and a match without decoupling anything: the TUI still has to enumerate every variant to produce an `EffectCommand`, and core still has to know about `ShareSession` / `LoginValidateKey` shapes.

Either (a) collapse to a single mapping in the TUI (drop `effect_payload.rs`; `effects/mod.rs` reads `Event` + `AppState` directly), or (b) document a concrete reason the pure-core intermediate is required (e.g. a future non-TUI consumer of `EffectPayload`).

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Collapse** — `effect_payload.rs` deleted; `EffectPayload` removed from `runie-core` exports; `runie-tui/src/effects/mod.rs` reads `Event` + `AppState` and produces `EffectCommand` in one match; OR
  - (b) **Keep + document** — a concrete future consumer (e.g. `runie-server` wanting the same effect extraction without ratatui) is written into `effect_payload.rs` module docs.
- [ ] If (a): `rg "EffectPayload\|effect_payload" crates/` returns zero hits.
- [ ] If (a): the existing `effect_payload::extract` tests move to the TUI as `effects::compute_command` tests (same coverage, new location).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `compute_command_copy_last_response` — `compute_command(CopyLastResponse, state_with_assistant_msg)` returns `EffectCommand::CopyToClipboard { text: "the answer" }`.
- [ ] `compute_command_copy_last_response_empty` — with no assistant message, returns `None`.
- [ ] `compute_command_open_editor` — `OpenExternalEditor` with `input = "hi"` returns `EffectCommand::OpenExternalEditor { text: "hi" }`.
- [ ] `compute_command_share_session` — `ShareSession` returns `EffectCommand::ShareSession { messages, display_name }`.

### Layer 2 — Event Handling
- [ ] `ui_actor_dispatches_copy_effect` — `UiActor::handle_event` for `ControlEvent::CopyLastResponse` enqueues an `EffectCommand::CopyToClipboard` (existing test stays green).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_clipboard_copy_still_fires` — pressing the copy key in a render/e2e test still triggers the clipboard effect.

## Files touched

- `crates/runie-core/src/effect_payload.rs` (deleted if option a)
- `crates/runie-core/src/lib.rs` (remove `pub mod effect_payload;` if option a)
- `crates/runie-tui/src/effects/mod.rs` (inline the extraction match if option a)
- `crates/runie-core/src/tests/` (move the two `effect_payload` tests to `runie-tui/src/effects/`)

## Notes

The pure-core placement was likely intended to keep effect *intent* in the domain layer and effect *execution* in the IO layer — a clean separation. The YAGNI argument is that with one consumer (the TUI), the separation is speculative. If a future `runie-server` or headless CLI wants the same effects, option (b) + keep is correct. Weigh against the posture: "IO | Domain (pure) | UI" suggests the pure extraction *should* live in domain — so option (b) may be the more architecturally faithful choice. Decide explicitly, don't drift.
