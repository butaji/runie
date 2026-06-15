# Harness Skill: Loop Detector

**Status**: todo
**Milestone**: R3
**Category**: Tools
**Priority**: P1

**Depends on**: harness-skill-framework
**Blocks**: (none)

## Description

Detect repeated failed tool patterns (e.g., the same edit failing three times, or the same file being read repeatedly without progress) and inject a recovery prompt asking the model to reconsider. This prevents token-wasting doom loops.

## Acceptance Criteria

- [ ] Skill hooks `on_tool_call` to track recent tool calls and their outcomes.
- [ ] A loop is detected when the same operation on the same target repeats more than `max_repeats` times without success.
- [ ] On detection, a `LoopDetected` event is published and a recovery hint is injected into the next LLM prompt.
- [ ] Thresholds are configurable under `[harness.skills.loop_detector]`.
- [ ] Skill can be disabled.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `loop_detector_fires_on_repeated_failed_edit` — three failed edits on the same line trigger detection.
- [ ] `loop_detector_ignores_successful_repetition` — repeated successful reads do not trigger.

### Layer 2 — Event Handling
- [ ] `loop_detection_emits_recovery_event` — detection publishes `LoopDetected` and the agent prompt changes.

### Layer 3 — Rendering
- [ ] `loop_warning_renders_in_footer` — TUI shows a brief "reconsidering approach" hint.

### Layer 4 — Smoke / Crash
- [ ] `smoke_loop_recovery` — run binary on a task that causes a known edit loop, verify recovery prompt appears.

## Files touched

- `crates/runie-core/src/skills/loop_detector.rs`
- `crates/runie-core/src/config.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-core/src/event/variants.rs`

## Notes

- Keep detection state per turn, not global, to avoid false positives across unrelated tasks.
- See `docs/adr/0022-harness-middleware-plugins.md`.
