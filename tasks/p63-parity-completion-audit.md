# P63 — Pi/Grok parity completion audit

Status: active; source-backed gap matrix (2026-08-08, refreshed after model-selector activation)

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
   Runie's provider-scoped `CodexWebSocketAdapter` now owns the injected
   connector/socket lifecycle, URL/header/envelope construction, Responses
   message validation/decoding, explicit fallback capability, and close/error
   cleanup. `TokioCodexWebSocketConnector` now supplies the production socket
   and timeout/header/frame boundary; cached continuation retry, bounded
   pre-stream fallback, and provider-owned cleanup are implemented in the
   adapter. Live-environment deployment evidence remains separate from the
   deterministic replay/actor contract.
   Runie's replay adapter now also accepts Pi's OpenAI Responses text-delta and
   completion event names, plus function-call argument deltas keyed by
   `output_index`, and normalizes them to the ordinary assistant event
   contract. `response.failed` now preserves the provider error code/message
   as a typed replay failure; this is replay-format coverage, not a claim of
   live Codex socket parity.
2. Provider-specific deferred-response polling/decoding and cancellation.
   Runie already exposes actor-owned `fetch_deferred` and `cancel_deferred`
   capability commands. An injected adapter contract now proves both commands
   cross the `ProviderActor` mailbox and publish a deferred terminal event.
   The injected replay adapter now also proves ordered provider-scoped polling
   batches, cancellation, handle-scope validation, and exhaustion errors.
   Provider-specific live polling/decoding and a real provider fixture remain
   open; replay polling must not be mistaken for live adapter parity.
3. Full Pi telemetry callback/span nesting and provider-specific diagnostics;
   generic lifecycle events are not evidence of span parity.
   Provider cancellation and supersession now settle the active request span
   through the owning actor with structured abort details; typed schema and
   exporter/backend conformance remain open.
4. Session storage wire typing and compaction lifecycle. All nine Pi session
   operation families now have live producer transitions and actor-owned
   JSONL round-trip coverage (see p52); canonical message lane identity is
   carried by `SessionEntry.lane`, with `entry_lanes` retained only for older
   callers and serialized snapshots. The remaining gaps are the generic
   `(record_type, data)` compatibility edge and concrete-provider compaction
   summarization/publication, not missing live record producers.

## Open Grok TUI contracts mapped to Pi features

1. Exact grouped member-card geometry and reflow across all display modes.
2. Complete Pi-mappable command registry/action execution.
   The source-backed Pi built-in slash-command vocabulary is now present as a
   pure macro-generated `runie-core::commands` registry (P64). `/new`,
   `/hotkeys`, `/model`, `/name`, `/compact`, and `/quit` now have a shared
   async route through their owning actor/event boundaries, with YAML action or
   post-state coverage. Model catalog selection, scoped-model filtering, and
   session-info projection now have actor/YAML coverage; the remaining commands
   remain open.
   The model selector contract is now implemented through catalog/UI/loop
   actors, including async search, scoped rows, Ctrl-L routing, selection
   commit, `/scoped-models`, `/session`, and YAML state/render coverage. Remaining command gaps are the
   other Pi commands not yet backed by an executable Runie capability.
3. Cast-wide zero-diff coverage for every Pi lifecycle/error/abort family,
   including terminal capability variants and dynamic timing metadata.

The Runie side of p53 runtime resize evidence is now complete: all four
standard initial geometries observed the declared `80×12` and `100×24`
transitions with valid reports. Paired Grok settled-frame cell comparison is
still excluded from closure because the current Grok capture does not produce
valid settled artifacts under the same schedule.

## Required evidence for closure

Each item above needs: an upstream source citation in its task, a typed
actor/event boundary, a YAML scenario that can be changed without recompiling,
an expected state assertion, and whole-screen cell/style/ANSI verification at
all four capture sizes. Unsupported behavior must remain an explicit typed
capability result until its source-backed adapter data exists.

This audit prevents green generic tests from being mistaken for 100% parity.

Response failure evidence (2026-08-08): the replay parser now rejects Pi
`response.failed` frames with a typed `StreamError::Api`, preserving
`error.code` plus `error.message` (or `incomplete_details.reason`). The
regression runs through the provider replay boundary and the full `just ci`
matrix; live provider transport behavior remains a separate open contract.

Response incomplete evidence (2026-08-08): `response.incomplete` with
`incomplete_details.reason: max_output_tokens` now reduces to
`StopReason::MaxTokens`; other incomplete reasons reduce to `Error`, matching
Pi's `mapStopReason`. The 183-trace replay matrix was rerun after preserving
the legacy chat-completions tool ordering contract.

Response usage evidence (2026-08-08): terminal Responses usage now maps
`input_tokens`, `output_tokens`, `total_tokens`, cached input tokens, and
reasoning tokens into Runie's existing `Usage` payload on the acknowledged
assistant `Done` event. The full workspace gate and replay matrix pass; cost
calculation and live provider transport remain adapter-specific open work.

Codex transport audit correction (2026-08-09): production deployment and the
source-aligned bounded pre-stream retry policy are now implemented and tested.
The remaining Codex item is provider-specific deferred behavior and any
future live-environment integration evidence; the generic HTTP actor remains
intentionally transport-neutral.
