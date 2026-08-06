# p28 — Parity decision data contract

## Purpose

Parity decisions are currently slowed by missing runtime facts rather than
compilation. This contract lists the minimum data required before a pi/core or
Grok-rendering mismatch can be classified as a product defect.

## Required capture record

Every accepted paired run must retain, beside each cast or YAML replay:

- reference commit/version and Runie commit;
- Grok binary path plus version/hash and relevant config (`screen_mode`, theme,
  native-color lock, model, approval mode);
- exact prompt, provider response/tool outputs, and serialized pi-compatible
  event sequence;
- terminal `cols × rows`, `TERM`, `COLORTERM`, color level, alternate-screen
  mode, and tmux/asciinema versions;
- one frozen timestamp source, usage/token values, thinking/turn elapsed values,
  and animation phase/tick inputs;
- frame-selection markers, occurrence/index, selected frame number, and the
  reason the frame represents the intended lifecycle boundary;
- raw `.cast`, raw ANSI stream, parsed full cell grid, and comparator output.

The repository capture harness now writes `<cast>.meta.json` automatically with
the capture command, requested environment, geometry, repository revision,
Grok path/version, tmux/asciinema versions, and cast/raw artifact paths.
Scenario-specific provider and clock fields remain YAML responsibilities.

Live validation (2026-08-06): a 62×32 `runie --terminal-native` capture
produced a valid manifest containing revision `ed238f61`, Grok `0.2.118`, tmux
`3.7b`, asciinema `3.2.1`, requested truecolor environment, geometry, and both
raw artifacts. The provenance path is operational.

## Decision rules

1. Missing any required field means “insufficient evidence,” not “Runie bug.”
2. Different prompt/provider/tool/clock inputs cannot support pixel parity.
3. Native-color Grok captures compare against `TerminalNative`; themed Grok
   captures compare against the matching Opaline theme and color capability.
4. Dynamic fields must be event-owned/frozen in YAML before exact comparison.
5. A parity claim must include all four default geometries and both symbols and
   attributes where the capture proves the relevant color capability.

## Current missing evidence

- The checked-in `grok-rich.cast` lacks a reliable version/config and emits
  terminal-default SGR, so it cannot prove truecolor parity.
- Fresh live pairs still lack one shared provider response and frozen elapsed
  telemetry; their remaining mismatches are therefore not actionable layout
  evidence.
- The current YAML reference uses a fixed frame index from an older capture;
  marker-locked selection is now available but not yet used for every fixture.
- Promoting `visual-grok-feed` to attribute comparison exposed 230 mixed-theme
  cells in the legacy frame (mostly `GrokNight` RGB prompt/background cells),
  even though the event sequence now selects `TerminalNative`. This confirms
  that the fixture needs a clean same-event native recapture before promotion.
- After fixing prompt event propagation, the legacy frame still exposes 73
  RGB cells (row 14/status or prompt chrome), so strict attribute promotion
  remains correctly blocked on a clean same-event reference rather than being
  weakened.

This contract is the review checklist for p19, p21, p25, and p27. It is the
data needed to make the next implementation decision faster and with fewer
false regressions.
