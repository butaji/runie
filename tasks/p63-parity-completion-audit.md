# P63 — Pi/Grok parity completion audit

Status: active; source-backed gap matrix (2026-08-08)

This task is the authoritative checklist for the remaining 100% parity work.
It keeps “implemented and replay-verified” separate from “typed but awaiting a
provider-specific contract”. The source of truth is the current Pi tree at
`/Users/admin/Code/agents/pi` and the Grok pager/render trees at
`/Users/admin/Code/agents/grok-build`.

## Verified boundaries

- Pi lifecycle and assistant event names are validated by
  `scripts/validate-pi-event-contract.py`.
- Core state transitions use actor-owned reducers and typed mailbox/events.
- TUI feed/status/prompt projections are actor snapshots rendered by pure
  functions; `scripts/validate-feed-actor-boundary.py` enforces the feed seam.
- YAML replay, whole-screen cell/style assertions, asciinema references, and
  four capture sizes run through `just ci`.
- `session_flush: true` is exercised by
  `visual-operation-queue-lifecycle.yaml`, proving that YAML can place an
  awaited session-actor barrier between declared lane facts without sleeps.
- Runtime TUI modules reject synchronous filesystem/process APIs through
  `lint-check`; offline YAML/capture binaries remain explicitly isolated.

## Open Pi-core contracts

1. Provider-scoped Codex WebSocket parity from Pi's
   `openai-codex-responses.ts`: URL resolution, Responses envelope, socket
   decoder, continuation cache, pre-stream retry, SSE fallback, and owned
   session/account cleanup. The generic HTTP actor must not emulate these.
   Runie's replay adapter now also accepts Pi's OpenAI Responses text-delta and
   completion event names, plus function-call argument deltas keyed by
   `output_index`, and normalizes them to the ordinary assistant event
   contract. `response.failed` now preserves the provider error code/message
   as a typed replay failure; this is replay-format coverage, not a claim of
   live Codex socket parity.
2. Provider-specific deferred-response polling/decoding and cancellation.
   Runie already exposes actor-owned `fetch_deferred` and `cancel_deferred`
   capability commands. An injected adapter contract now proves both commands
   cross the `ProviderActor` mailbox and publish a deferred terminal event;
   provider-specific polling/decoding and a real provider fixture remain open.
3. Full Pi telemetry callback/span nesting and provider-specific diagnostics;
   generic lifecycle events are not evidence of span parity.
4. Remaining session storage record families and direct lane identity
   migration where the compatibility `entry_lanes` projection is still used.

## Open Grok TUI contracts mapped to Pi features

1. Exact grouped member-card geometry and reflow across all display modes.
2. Complete Pi-mappable command registry/action execution.
3. Cast-wide zero-diff coverage for every Pi lifecycle/error/abort family,
   including terminal capability variants and dynamic timing metadata.

## Required evidence for closure

Each item above needs: an upstream source citation in its task, a typed
actor/event boundary, a YAML scenario that can be changed without recompiling,
an expected state assertion, and whole-screen cell/style/ANSI verification at
all four capture sizes. Unsupported behavior must remain an explicit typed
capability result until its source-backed adapter data exists.

This audit prevents green generic tests from being mistaken for 100% parity.
