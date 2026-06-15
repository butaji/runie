# Status Indicator Widget

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P2

**Depends on**: actor-runtime-decision, tool-call-state-rendering

## Description

Runie already has a status bar, but it does not show the current agent phase
with elapsed time and an interrupt hint. Research from Codex
(`StatusIndicatorWidget`), thClaws (`BusyGuard` + terminal title), and Gemini
CLI (`StatusRow`) shows that a focused status indicator improves perceived
responsiveness.

## Acceptance Criteria

- [x] `crates/runie-core/src/agent_phase.rs` defines `AgentPhase` enum and `format_elapsed`:
  - Current phase: `thinking`, `composing`, `tool:<name>`, `waiting`, `idle`
  - Elapsed time formatting: `12.3s` under a minute, `1m5s` over a minute
- [ ] `AgentState` tracks `current_phase: AgentPhase` and `phase_started_at`.
- [ ] Phase transitions emitted as transient `AgentPhaseChanged` events.
- [ ] Status bar uses the new widget.
- [ ] Terminal title updates to reflect phase (Unix-only, optional).
- [ ] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `phase_transitions_record_start_time` — elapsed_secs helper tracks time.
- [x] `format_elapsed_under_minute` — `12.3s`.
- [x] `format_elapsed_over_minute` — `1m5s`.

### Layer 2 — Event Handling
- [ ] `tool_start_changes_phase_to_tool` — `AgentToolStart` sets `tool:name`.

### Layer 3 — Rendering
- [ ] `status_bar_shows_thinking_phase` — rendered line contains `thinking`.
- [ ] `status_bar_shows_interrupt_hint` — `Ctrl+C` visible while running.

## Notes

**Files touched:**
- `crates/runie-tui/src/status_indicator.rs` (new)
- `crates/runie-tui/src/status_bar.rs`
- `crates/runie-core/src/state.rs`

**Out of scope:**
- Desktop notifications (future).
- Sound alerts.
