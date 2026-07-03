# Remove `_ => {}` empty match arms

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: none
**Blocks**: none

## Description

`_ => {}` empty catch-all match arms appear 7x in `update/dialog/panel.rs`, 3x in `markdown/blocks.rs`, 2x each in `update/system.rs` and `update/login_flow.rs`, and 1x in several other files. These are either intentional no-ops (events that should be ignored) or bugs (events that should be handled but aren't). Replace each with either an explicit `#[default]` arm with a `// intentionally ignored: <reason>` comment, or — where the enum is small — exhaustively list the ignored variants so new variants trigger a compile error instead of silent swallowing.

## Acceptance Criteria

- [ ] Every `_ => {}` arm in production code (`update/`, `markdown/`, `dialog/`) either: (a) has a `// intentionally ignored` comment explaining why, or (b) is replaced with explicit variant arms.
- [ ] No `_ => {}` arm in `update/dialog/panel.rs` without a comment.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo clippy --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `ignored_event_is_truly_noop` — for each documented no-op arm, feeding the event produces no state change (no dirty, no transient, no dialog change).

### Layer 2 — Event Handling
- [ ] `panel_ignores_unrelated_events` — dialog panel handler returns `Consumed` for relevant events, `Ignored` for explicitly-ignored ones.
- [ ] `markdown_ignores_non_block_tokens` — markdown block parser doesn't crash on unexpected token types.

### Layer 3 — Rendering
- [ ] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [ ] N/A — no new IO or async paths.

## Files touched

- `crates/runie-core/src/update/dialog/panel.rs` — 7 arms
- `crates/runie-core/src/markdown/blocks.rs` — 3 arms
- `crates/runie-core/src/update/system.rs` — 2 arms
- `crates/runie-core/src/update/login_flow.rs` (or `login_flow/handlers.rs`) — 2 arms
- `crates/runie-core/src/update/input/mod.rs` — 2 arms
- `crates/runie-core/src/update/tools.rs` — 1 arm
- `crates/runie-core/src/update/session.rs` — 1 arm
- `crates/runie-core/src/update/input/scroll.rs` — 1 arm
- `crates/runie-core/src/update/dialog/toggle.rs` — 1 arm
- `crates/runie-core/src/update/dialog/form.rs` — 1 arm

## Notes

This is primarily a readability/correctness task, not a LOC reduction. The real value is turning silent swallowing into compile-time exhaustiveness checking where feasible. For large enums (like `Event` with ~188 variants), exhaustive listing is impractical — use the `// intentionally ignored` comment there. For small enums (DialogEvent sub-enums, markdown token types), prefer explicit arms. Do NOT add a generic `noop()` helper — that's worse than the comment because it hides intent.
