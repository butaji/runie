# p13 — TUI: status/footer bar state machine (all variants + spinner frames + hints)

**Parity target:** grok-build pager status/footer surface.

## Grok reference

- Spinner frames — `~/Code/agents/grok-build/crates/codegen/xai-grok-pager-render/src/glyphs.rs:225`
  ```rust
  pub fn braille_spinner_frames() -> &'static [&'static str] {
      const FANCY: &[&str] = &["\u{280b}","\u{2819}","\u{2839}","\u{2838}","\u{283c}","\u{2834}","\u{2826}","\u{2827}"];
      const FALLBACK: &[&str] = &["|","/","-","\\"];
      if is_legacy_windows_console() { FALLBACK } else { FANCY }
  }
  ```
  (Braille: ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧; fallback `| / - \`.)
- Dot spinner — `glyphs.rs:238`: `⋅ : ⸬ ⁙` normally, `. : ·` on legacy ConHost.
- Status rendering consumes the spinner by `tick` — `~/Code/agents/grok-build/crates/codegen/xai-grok-pager/src/app/agent_view/render.rs:485,4042` (`frame_idx = (tick/4) % len`), and `"{spinner} Loading..."` label (line 4045).
- Footer hint vocabulary (idle vs active): `Enter`, `Shift+Tab`, `Ctrl+x`, `ctrl+q`/`ctrl+d` quit hints (`views/welcome/mod.rs:48-53`).

Typed waiting labels (2026-08-06): `WaitingReason::label()` now mirrors
Grok's model, subagent, task-output, tasks-complete, and sleep subjects. The
actor-owned `TurnStatus` carries that label into the pure view, and
`visual-waiting-reasons.yaml` replays all typed waiting events without sleeps.

## Current runie state

`~/Code/GitHub/runie-tests/runie/crates/runie-tui/src/widgets/status.rs`
- `TurnStatusPhase` + `TurnStatus` + `StatusBar` exist; the renderer-independent
  actor model uses `Status::{Ready,Loading,Thinking,Streaming,Waiting(_),Aborted,Error}`;
  typed `WaitingReason` covers model, subagent, task output, tasks complete,
  and sleep. Braille frames and deterministic animation ticks are implemented.

## Adapt to runie — state machine

Promote the status bar to an explicit state machine with **all** grok variants:

```
idle/user-editing ──submit──> thinking ──message_start(assistant)──> streaming
streaming ──message_end(assistant)──> ready/idle
thinking ──agent_end──> ready/idle
any ──error/aborted──> error(reason) ──ack/agent_end──> idle
ready/idle ──turn_start──> thinking          (each new turn)
active ──(no turn)──> idle                    (session idle)
```
Variants to implement:
- Spinner kind: `braille` (8 frames) vs `dot` (4 frames) vs legacy fallback (`|/-\`).
- Phase: `Idle` / `Thinking` / `Streaming` / `Error{reason}` / `Loading`.
- Footer hint set: idle hints (`Enter`, `Shift+Tab`, `Ctrl+x`) vs active hints (same vocabulary, different emphasis) — match grok exactly.
- Deterministic frame selection: `frame_idx = (tick / ticks_per_frame) % frames.len()` — animation owned by the status bar, not free-running.

## Acceptance

- Snapshot tests: each phase renders the exact grok footer text and spinner glyph; frame sequence is deterministic and stable across redraws (already patterns in `status.rs` tests — extend to cover error/loading/legacy-fallback variants).
- `cargo test -p runie-tui` green.
