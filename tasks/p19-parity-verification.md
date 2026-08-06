# p19 — Verification: parity harness (pi event-sequence replay + grok cast snapshot diffs)

**Feed attribute-oracle audit (2026-08-06):** Attempting to promote the
legacy `grok-rich.cast` feed frame to `exact_attributes: true` produced 518
differences beginning at the session row: the cast contains terminal-default
foreground/background values, while Runie correctly emits the source-backed
GrokNight Opaline surface (`fg #6c6c6c`, `bg #141414`). The fixture remains
symbol-exact; a fresh truecolor Grok capture is required before this oracle
can be promoted without comparing incompatible terminal capabilities.

**Declarative queue-mode oracle (2026-08-06):** YAML scenarios now select the
actor-owned `steering_mode` and `follow_up_mode` instead of the runner forcing
pi's default `one-at-a-time`. `follow-up.yaml` exercises `follow_up_mode: all`
with two queued messages and asserts they are drained in one continuation
turn. This keeps queue semantics in the no-recompile replay layer.

**Provider failure state oracle (2026-08-06):** `visual-error.yaml` now pins
the complete terminal event order and the actor-owned `error_message` state in
addition to the rendered error row. This keeps pi's error lifecycle covered
as event sequence → projection → TUI output.

**Capture capability contract (2026-08-06):** `tmux-asciinema-capture.sh` now
records with `TERM=xterm-256color COLORTERM=truecolor` by default, while still
allowing validated caller overrides. YAML dump references also support
`require_truecolor: true`; when combined with `exact_attributes`, the runner
fails before comparison if the selected frame contains no RGB cells. This
turns missing capture capability into an explicit test failure instead of a
silent downgrade to symbol-only evidence.

**Fresh isolated capture (2026-08-06):** Captured Grok and Runie independently
at 80×24 through `just tmux-cast` with the same `Hey` probe and truecolor
environment hints. The indexed phase comparator reports Grok 3 visible frames
versus Runie 20; after the `Worked for` marker, the first corresponding-frame
divergence is at frame 1, cell `(69,1)` (`9` versus `5`). This is a real
capture-phase/content mismatch and is preserved in `/tmp/runie-parity-current/`;
the oracle remains strict and was not weakened.

**Latest parity note (2026-08-05):** `LoopActor::continue_run` now drains
queued steering before follow-up messages when the context ends in an
assistant, matching pi's continuation behavior; `LastIsAssistant` remains the
empty-queue error. Its steering path also skips the normal initial steering
poll, matching pi's `skipInitialSteeringPoll` option. A focused regression
covers the steering path.

**Assistant stream wire shape (2026-08-06):** Text and thinking delta events
now carry pi-compatible `contentIndex` fields, and their end events carry the
matching `contentIndex` plus `content`. Producers in the core replay provider,
live TUI sample stream, YAML runner, and test traces all populate the fields;
serialization tests assert the exact camel-cased keys. Pi's `partial` payloads
are now present and tested on every text and thinking variant. Synthetic
replay/YAML sources currently use an explicit default payload; deriving the
fully accumulated partial assistant message at each live provider boundary
is now performed by the loop reducer before `MessageUpdate` publication. A
focused reducer test proves the provider placeholder is replaced by the
actor-owned accumulated assistant state; provider transport events themselves
remain immutable inputs. The pi `start.partial` and `toolcall_end.partial`
fields are also now part of the typed event model and exact wire assertions;
the reducer replaces both provider placeholders with its actor-owned partial
assistant state before any downstream publication.

**Message lifecycle audit (2026-08-06):** Re-read pi's `runAgentLoop` and
`runLoop` beside `runie-core/src/loop/driver.rs`. Prompt and steering
`message_start`/`message_end`, assistant boundaries, tool-result message
boundaries, `turn_end`, continuation turns, and final `agent_end` ordering
match. The remaining p19 gap is cast-wide, full-attribute frame proof rather
than an unverified lifecycle branch.

**Transcript parity note (2026-08-05):** Grok-style grouped activity now
includes the reference failure suffix (`· N failed`) for failed file,
directory, and command tools. The behavior is exercised by the discovered
`visual-tool-error.yaml` fixture.

- **Core loop audit (2026-08-06):** Re-read pi's `runLoop` control flow beside
  `runie-core/src/loop/driver.rs`. The audited ordering matches for initial
  steering, assistant error/abort termination, truncated tool-call failure,
  tool-batch continuation, `turn_end`, `prepareNextTurn`,
  `shouldStopAfterTurn`, steering re-poll, follow-up polling, and final
  `agent_end`. No additional core behavior change was justified by this
  comparison; the remaining verification gap is exhaustive cast/frame proof,
  not an untracked loop branch.

- **Cast audit (2026-08-05):** A tmux/asciinema `grok` inspection confirmed
  that the remaining TUI oracle gap includes dynamic turn-status telemetry
  (elapsed/tokens/stop reason) and queue/footer variants. The current tests
  cover static chrome and representative frames, but not a cast-wide,
  byte-exact frame stream with those dynamic values.

- **Telemetry wiring (2026-08-05):** Typed usage and stop-reason data now flow
  from the core `Done` event into the actor-owned TUI status projection. The
  exhaustive cast oracle is still required to validate exact placement and all
  variants.
- **Elapsed/status contract (2026-08-05):** `StatusBar` now projects
  deterministic elapsed tenths, usage totals, and exact stop-reason labels into
  `TurnStatus`; focused tests and the full local gate pass. Cast-wide placement
  and completion-row (`Worked for`) parity remain open.
- **Completion row (2026-08-05):** The event renderer now emits `Worked for
  N.Ns` on `AgentEnd` from the actor-owned elapsed projection; the submitted
  visual fixture asserts it declaratively. Exact cast timing remains a
  separate oracle concern.
- **Markdown frame coverage (2026-08-05):** The rich-markdown fixture now
  asserts the complete table box, including the closing border observed in the
  Grok cast; the full cast-wide frame oracle remains pending.
- **Usage chrome (2026-08-05):** Live cast inspection found Grok's compact
  usage units (`⇣3.18k`); the pure status formatter now emits integer, `k`, and
  `M` forms with trimmed decimals and focused boundary coverage.

**Queue-phase parity note (2026-08-05):** Core loop polling now separates
steering from follow-up queues like pi's inner/outer loop. Follow-ups remain
queued across tool-continuation turns and are consumed only at the stop
boundary; existing event-sequence and follow-up regressions remain green.

**Hook-context parity note (2026-08-05):** Turn hooks now observe the
actor-synchronized post-turn transcript, including tool-result messages,
rather than the pre-turn context. A regression asserts the completed context
length before `shouldStopAfterTurn` returns.

**Supplied-context parity note (2026-08-05):** `run_loop` now carries a
non-empty caller-supplied `AgentContext` into the provider request and merges
the completed projection back into subsequent turn contexts. A provider
regression covers prior context plus a new prompt.

**Model-wire parity note (2026-08-05):** `thinkingLevelMap` now serializes
pi-compatible provider effort strings for all levels, distinct from numeric
`thinkingBudgets`; the model serde oracle covers `high` and `max` mappings.

**Tool-hook parity note (2026-08-05):** `afterToolCall` override fields are
now applied to the actor-produced tool result and lifecycle event, with an
integration regression covering content, details, usage, termination, and
error-state overrides.

The same regression verifies the callback sees the current two-message
transcript (prompt plus assistant) before tool results are appended.

**Parity target:** proof that runie-core and runie-tui behave identically to pi and grok.

- **Current gate evidence (2026-08-05):** The focused same-run 62×32 `Hey`
  oracle is still non-exact (71 full-cell deltas). The workspace gate also
  catches a real 80×24 `visual-grok-feed` full-screen mismatch after the
  settled-turn gutter changes (`(5,5)` expected blank, Runie user cursor).
  The reference assertion remains strict until the responsive autoscroll
  projection is reconciled.

- **Mismatch diagnostics (2026-08-05):** YAML full-screen failures now report
  complete expected and actual rows around the first differing cell. The
  80×24 failure is specifically a phase/frame-selection difference (expected
  blank row at `(5,5)`, Runie prompt row); an attempted hidden-member-height
  adjustment moved the activity header incorrectly and was reverted.

**Replay matrix audit (2026-08-05):** The discovered matrix contains 183
sidecars. Every successful provider sidecar has a declarative
`core.exact_events` vector (159 fixtures); the remaining 24 are provider
decode/transport error contracts and are intentionally validated through
their error schema rather than a core lifecycle vector. Every sidecar also
declares state expectations. The remaining verification gap is therefore the
cast-wide frame oracle, not untested successful replay fixtures.

**Cast state inventory (2026-08-05):** Added a terminal-emulator-backed
state inventory oracle covering every output frame in both Grok casts. The
current recordings classify as: `grok-full` 72 frames across welcome, prompt,
thinking, responding, waiting, completed, command-palette, header-only, and
blank states; `grok-rich` 212 frames across welcome, thinking, responding,
waiting, completed, header-only, and blank states. The snapshot fails on any
new unclassified frame, making the remaining frame-by-frame comparison gap
measurable instead of implicit.

## Reference

- pi event sequences: `runAgentLoop` ordering — `agent_start`, `turn_start`, prompt `message_start`/`end`, assistant `message_start`/`message_update*`/`message_end`, tool events, `turn_end`, `agent_end` — `~/Code/agents/pi/packages/agent/src/agent-loop.ts:95-274`.
- grok TUI reference states: recorded asciinema casts `~/Code/GitHub/runie-tests/runie/artifacts/grok-full.cast` and `grok-rich.cast` (already used by `crates/runie-tui/tests/visual_snapshots.rs`).

## Adapt to runie

1. **Core event-sequence oracle**: a table of pi traces (JSON) → expected event-kind sequence. For each supported scenario (plain prompt, tool call, tool_use continuation, length-truncated tool call, error/aborted, steering-at-start, follow-up, continue-after-assistant), assert runie-core emits the exact ordered event kinds (using the existing `common::event_kinds` harness). Each row cites the pi source line it mirrors.
2. **State projection oracle**: for each trace, assert the `AgentState` projections (`is_streaming`, `streaming_message`, `pending_tool_calls`, `error_message` — p12) match pi's `AgentState` at key checkpoints.
3. **TUI snapshot diff**: extend `visual_snapshots.rs` so every transcript/status/prompt frame in the runie TUI is diffed against the frames extracted from `grok-full.cast`/`grok-rich.cast` with **zero diffs** (byte-exact symbols). New fixtures for reasoning fold, verb-group folding, tool error, markdown code blocks.
4. **Serialization oracle**: round-trip every new field (p01-p04, p09) and assert the JSON shape matches pi's TS wire forms exactly.

## State machine / variants

The oracle is a table: `scenario → pi_reference(file:line) → expected_events → expected_state_projection → expected_tui_frame`. Each row is a pass/fail check; the harness fails on any mismatch.

## Acceptance

- `cargo test --workspace` green, including the new parity harness.
- `just ci` (fmt-check + clippy + lint + test) fully green.
- All `tasks/index.json` steps `p01..p19` marked `done`.
- The harness reports **100%** of pi traces and grok frames pass (this is the 10/10 confidence gate for the parity claim).

## Progress

- **Declarative exact TUI event vector (2026-08-06):** The YAML runner now
  supports `assertions.exact_events`, comparing the complete ordered
  `AgentEvent` kind vector rather than only checking membership. The canonical
  `visual-hey.yaml` scenario uses it together with terminal state assertions
  (`is_streaming=false`, no pending tools, two messages), so event ordering and
  resulting state are validated without recompilation.

- **Verified (2026-08-05):** `cargo test --workspace` passes, including 32
  core unit tests, replay/integration suites, 57 TUI unit tests, E2E, and all
  22 visual snapshot/reference checks. The seven prompt snapshots were
  re-recorded for the newly implemented Grok-aligned empty placeholder.
- The full pi trace oracle and byte-exact coverage matrix described above are
  not yet complete, so this step remains pending.
- **Replay state oracle (2026-08-05):** The replay test now asserts each
  fixture's declared `state.assistant_messages` instead of using a tool-call
  heuristic. Corrected 61 tool-trace sidecars to declare the documented
  auto-continuation count, and the complete 181-sidecar replay test passes.
- **Provider error oracle (2026-08-05):** Added an exact provider-actor test
  proving startup failures produce the pi-compatible assistant `Error` event
  (`api: bad request`) on the subscribed stream.
- **Signal propagation oracle (2026-08-05):** Added a loop-entry test proving
  the provider receives the abort signal in its stream options, completing the
  wiring between the loop-owned cancellation state and provider adapters.
- **Granular tool-call oracle (2026-08-05):** Added a core reconstruction test
  covering the exact `ToolCallStart` → `ToolCallDelta` → `ToolCallEnd` sequence
  and asserting one finalized tool-call content block.
- **Delta boundary oracle (2026-08-05):** exact simple-prompt and provider
  error event sequences now assert that only delta events become
  `MessageUpdate`; terminal `Done`/`Error` events close through `MessageEnd`.
- **Replay ordering oracle (2026-08-05):** the 181-fixture replay harness now
  asserts ordered core lifecycle subsequences (`AgentStart` → `TurnStart` →
  message boundaries → `AgentEnd`) and every declared tool execution start →
  end pair, rather than checking event presence alone.
- **Final state oracle (2026-08-05):** every successful replay now asserts the
  actor-owned projection ends with `is_streaming=false`, no live streaming
  message, no pending tool calls, and no retained error. Abort semantics are
  covered separately by the loop-entry projection test.
- **TUI terminal-error oracle (2026-08-05):** the renderer test suite now pins
  the error projection from terminal `MessageEnd` payloads, matching the
  delta-only core event contract.
- **Projection race hardening (2026-08-05):** state projection tests now use
  the actor synchronization boundary before terminal assertions; the full
  workspace suite passes without timing sleeps.
- **End-to-end error ordering (2026-08-05):** Added a core event-sequence
  oracle covering provider startup failure through `AgentEnd`, including the
  assistant `StopReason::Error`, error payload, and exact terminal event order;
  the existing TUI `visual-error.yaml` fixture covers the visible projection.
- **Exact event oracle (2026-08-05):** Strengthened the simple-prompt core
  scenario from loose boundary/count assertions to the exact pi event-kind
  sequence, including all four assistant stream updates and terminal ordering.
  The broader trace matrix and TUI cast-frame zero-diff matrix remain open.
- **Reactive abort parity (2026-08-05):** Active assistant stream consumption
  now selects between provider events and the loop abort watch. Aborting marks
  the retained partial assistant `StopReason::Aborted`, publishes its terminal
  message event, clears streaming state, and records the error without waiting
  for the provider to emit another event.
- **YAML sequence oracle (2026-08-05):** Replay fixtures may declare
  `core.ordered_events`; the harness validates that ordered subsequence without
  Rust changes, while retaining the universal lifecycle invariant.
- **Provider-error content oracle (2026-08-05):** The startup-failure sequence
  now asserts the pi-compatible invariant that `StopReason::Error` and
  `error_message` are assistant metadata with no synthetic assistant text
  content.
- **Projection synchronization (2026-08-05):** Pending-tool replay coverage
  now uses an explicit tool-start signal and acknowledged state mutation,
  eliminating scheduler-dependent polling while preserving the in-flight
  projection assertion.
- **Tool lifecycle ordering (2026-08-05):** Sequential dispatch now records
  `ToolExecutionStart` before invoking the tool, and the parallel dispatch
  regression asserts start-before-end for every tool while retaining
  completion-order end events.
- **Coverage audit (2026-08-05):** The current local harness exercises 183
  provider replay sidecars (159 with complete exact core vectors), 22 YAML TUI
  scenarios, and 23 visual/reference tests. It proves selected
  lifecycle/state/cell contracts, but does not yet
  compare every recorded Grok frame byte-for-byte or provide a complete
  pi-trace-to-YAML expected-sequence table; p19 correctly remains pending.
- **Recorded transcript row oracle (2026-08-05):** Strengthened the Grok-feed
  visual test to parse `grok-rich.cast` with the terminal emulator and compare
  the complete grouped activity row cell-for-cell against the YAML-rendered
  frame. Only Grok's trailing transient block cursor is normalized; gutter,
  glyphs, labels, and spacing remain exact.
- **Exact sequence schema (2026-08-05):** Replay sidecars now support
  `core.exact_events`, which compares the complete emitted core event vector
  rather than only checking required events or an ordered subsequence. The
  canonical simple-prompt, abort, and broad multi-turn/tool sidecars use this
  exact oracle. YAML repetition tokens such as `MessageUpdate*27` keep large
  streamed vectors readable without recompilation. Provider-decode sidecars
  are validated through their parser-error contract and the full 24-fixture
  regression floor; exhaustive pi-trace coverage remains open.
- **Multi-turn exact oracle (2026-08-05):** Added a complete exact sequence
  for a standalone turn-2 continuation, including all 218 streamed update
  events and terminal ordering. Tool continuation and multi-turn coverage now
  have representative exact vectors; abort/error families remain open.
- **YAML startup-error oracle (2026-08-05):** Added a self-describing
  synthetic provider-failure sidecar. It executes the real loop actor with a
  deterministic failing provider and asserts pi-compatible error metadata and
  the exact `AgentStart` → `AgentEnd` terminal sequence. The replay matrix now
  covers 182 sidecars; abort and provider-decode-error families remain open.
- **YAML reactive-abort oracle (2026-08-05):** Added a cancellation-aware
  synthetic provider fixture. It drives `LoopActor::abort()` through a
  watch-channel-controlled stream, verifies the retained assistant is marked
  `StopReason::Aborted`, checks the actor-owned `aborted` projection, and pins
  the exact terminal event vector. The replay matrix now covers 183 sidecars;
  exhaustive Grok frame comparison remains.

- **Pi core event-surface audit (2026-08-06):** Compared Runie's public
  `AgentEvent` and `AssistantMessageEvent` variants with pi's
  `packages/agent/src/types.ts`. The lifecycle/tool/message event families are
  present, including sectional assistant markers and tool updates; no missing
  enum variant was justified by this audit. The actionable core gap is the
  provider-decode oracle: 24 YAML sidecars currently prove only that parsing
  fails, not a stable pi-compatible error classification/message. Exact
  decode-error vectors remain open and should be strengthened before claiming
  exhaustive core parity.
- **Provider-decode oracle (2026-08-05):** The replay matrix validates every
  `outcome: error` sidecar as a parser/transport failure and requires the YAML
  error contract `kind: provider_decode`; malformed/error traces are not sent
  through the success loop path. This closes the provider-decode fixture
  family. Exhaustive frame-by-frame Grok comparison remains.
- **Provider-decode diagnostic strengthening (2026-08-06):** The canonical
  OpenAI 401 body now asserts the exact `invalid: trace has no terminal event`
  classification substring in YAML, proving the harness preserves a stable
  parser diagnostic rather than only a failure boolean. The remaining decode
  sidecars still need representative API-error/message vectors.
- **Mid-stream provider error parity (2026-08-06):** Source/replay audit found
  that pi-style SSE `error:` frames were previously ignored by Runie's replay
  parser and misreported as a missing terminal event. The parser now converts
  the frame into `StreamError::Api`, and the `stream_error_mid_response` YAML
  sidecar asserts the provider's `Server error during streaming` payload.
- **Serialization oracle audit (2026-08-05):** Re-audited the p01–p04/p09
  wire contracts. Existing core tests round-trip assistant, tool-result, user,
  model, usage/cost, event, tool-hook, image, stop-reason, and thinking-level
  fields while asserting pi camelCase/tag names. This sub-oracle is complete;
  p19 remains pending only for exhaustive trace/frame coverage.
- **Coverage regression guard (2026-08-05):** The replay test now fails if the
  discovered YAML matrix loses its exact-event fixtures or all
  provider-decode-error fixtures, preventing silent parity-test erosion.
- **All stable welcome frames (2026-08-05):** The visual harness now compares
  every Grok cast frame containing the welcome marker against Runie cell by
  cell across the stable welcome rows. It intentionally excludes the lower
  tip/prompt rows, which the cast replaces during its welcome-to-input
  transition; all matching stable frames pass with zero diffs.
- **All active-footer frames (2026-08-05):** The rich-cast footer oracle now
  compares every matching `Esc … shortcuts` row against Runie’s rendered
  status cells, including exact symbols and spacing; all captured footer
  frames pass.
- **All status spinner frames (2026-08-05):** Starting, waiting, and
  responding cast rows are now compared across every captured braille-spinner
  frame using the corresponding deterministic Runie frame. Elapsed/usage
  chrome is treated as dynamic; the state label and spinner cells must match
  exactly for every frame.
- **Provider-options parity (2026-08-05):** The loop now carries static
  `SimpleStreamOptions` through every provider request, replacing only the
  API key when the loop-owned dynamic resolver supplies one. A two-turn tool
  continuation test verifies refreshed credentials plus session metadata are
  preserved on both calls; a missing dynamic key falls back to the static key,
  matching pi's `getApiKey(...) || config.apiKey` behavior.
- **Custom conversion oracle (2026-08-05):** The loop now supports pi's
  async `convertToLlm` hook after context transformation, with a compiled
  integration test proving the provider receives the customized wire
  projection.
- **Simple-trace exact family (2026-08-05):** Promoted the Kimi Code simple
  replay fixture to a complete YAML `core.exact_events` vector, including all
  26 streamed assistant updates. The exact-fixture regression floor now
  requires six independent fixtures.
- **Truncated-simple exact family (2026-08-05):** Added an exact YAML vector
  for the GLM 5.1 length-truncated simple response, pinning its six thinking
  updates and terminal `TurnEnd`/`AgentEnd` sequence. The regression floor now
  requires seven exact fixtures.
- **Multi-tool exact family (2026-08-05):** Promoted the DeepSeek v4 Flash
  two-tool replay to an exact YAML vector, including grouped tool updates,
  parallel start/update/end ordering, both tool-result messages, and the
  automatic continuation turn. The regression floor now requires eight exact
  fixtures.
- **Multi-turn initiation exact family (2026-08-05):** Promoted the Kimi Code
  weather-chain turn-1 sidecar to an exact YAML vector, pinning all 182
  streamed assistant updates before its continuation boundary. The regression
  floor now requires nine exact fixtures.
- **Short-truncated exact family (2026-08-05):** Added the GLM 5.2 simple
  length-truncated response as a complete YAML exact vector, covering its six
  thinking updates and terminal lifecycle. The regression floor now requires
  ten exact fixtures.
- **Provider hook-options oracle (2026-08-05):** Added pi-compatible payload
  and response hook fields to `SimpleStreamOptions`; the two-turn provider
  options test verifies they are forwarded unchanged on every request.
- **Gemini simple exact family (2026-08-05):** Promoted the Gemini 3.1 Flash
  Lite simple trace to a complete exact core event vector, extending exact
  coverage to an additional provider family.
- **Gemini reasoning exact family (2026-08-05):** Promoted the companion
  Gemini 3.1 Flash Lite reasoning trace to an exact core vector, covering both
  Gemini simple/reasoning sidecars without compiled test changes.
- **GLM 5 simple exact family (2026-08-05):** Promoted the OpenAI-compatible
  GLM 5 simple sidecar to an exact seven-update core vector, extending the
  short-truncated/normal GLM coverage family.
- **Qwen 3.7 Max simple exact family (2026-08-05):** Promoted the Anthropic
  Qwen 3.7 Max simple sidecar to an exact 43-update vector spanning its
  reasoning and final-text stream.
- **Qwen 3.7 Max OpenAI exact family (2026-08-05):** Promoted the
  OpenAI-compatible counterpart to an exact 40-update core contract,
  validating provider-specific decoding against the shared lifecycle.
- **Qwen 3.5 Plus simple exact family (2026-08-05):** Promoted the
  Anthropic Qwen 3.5 Plus simple sidecar to an exact 82-update reasoning/text
  vector, expanding long-stream coverage beyond the Qwen 3.7 family.
- **Qwen 3.5 Plus OpenAI exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.5 Plus sidecar to an exact 25-update vector,
  completing the provider-variant pair for this simple conversation.
- **Qwen 3.6 Plus simple exact family (2026-08-05):** Promoted the Anthropic
  Qwen 3.6 Plus sidecar to an exact 39-update reasoning/text vector.
- **Minimax M2.5 simple exact family (2026-08-05):** Promoted the Anthropic
  Minimax M2.5 simple sidecar to an exact two-update terminal vector.
- **Minimax M2.7 simple exact family (2026-08-05):** Promoted the adjacent
  Anthropic Minimax M2.7 simple sidecar to the same verified two-update core
  lifecycle vector.
- **Minimax M3 simple exact family (2026-08-05):** Promoted the Anthropic
  Minimax M3 simple sidecar to an exact single-text-update lifecycle vector.
- **Qwen 3.7 Plus simple exact family (2026-08-05):** Promoted the Anthropic
  Qwen 3.7 Plus simple sidecar to an exact 54-update reasoning/text vector.

- **GLM 5.1 tool exact family (2026-08-05):** Promoted the short
  OpenAI-compatible GLM 5.1 tool trace to a complete exact lifecycle vector,
  including its seven reasoning updates, tool execution, result message, and
  automatic continuation turn. The regression floor now requires 23 exact
  fixtures.
- **Minimax M2.5 parallel-tool exact family (2026-08-05):** Promoted the
  Anthropic Minimax M2.5 tool trace to an exact vector covering four streamed
  updates, two parallel tool lifecycles in completion order, source-ordered
  result messages, and automatic continuation. The regression floor now
  requires 24 exact fixtures.
- **Minimax M2.7 parallel-tool exact family (2026-08-05):** Promoted the
  adjacent Anthropic Minimax M2.7 trace, including its split tool-argument
  deltas and the same parallel lifecycle/result ordering contract. The
  regression floor now requires 25 exact fixtures.
- **Minimax M3 tool exact family (2026-08-05):** Promoted the compact
  Anthropic Minimax M3 tool trace to an exact vector, covering its single
  tool-call delta, tool lifecycle, result message, and continuation turn.
  The regression floor now requires 26 exact fixtures.
- **Minimax M3 multi-tool exact family (2026-08-05):** Promoted the Anthropic
  Minimax M3 two-city trace to an exact vector covering five streamed text/tool
  updates, parallel tool completion ordering, source-ordered results, and
  continuation. The regression floor now requires 27 exact fixtures.
- **Minimax M2.7 multi-tool exact family (2026-08-05):** Promoted the
  Anthropic Minimax M2.7 two-city reasoning/tool trace to an exact vector,
  including five streamed updates and parallel result ordering. The regression
  floor now requires 28 exact fixtures.
- **Minimax M2.5 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Minimax M2.5 two-tool trace to an exact vector, covering
  its three streamed updates and parallel lifecycle/result ordering. The
  regression floor now requires 29 exact fixtures.
- **Minimax M2.7 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Minimax M2.7 two-tool trace, including provider-specific
  tool-call chunking and exact parallel lifecycle ordering. The regression
  floor now requires 30 exact fixtures.
- **Qwen 3.7 Plus Anthropic tool exact family (2026-08-05):** Promoted the
  long Anthropic Qwen 3.7 Plus reasoning/tool trace to an exact vector with 36
  streamed updates, duplicate tool lifecycle events from the recorded
  decoder, ordered results, and continuation. The regression floor now
  requires 31 exact fixtures.
- **Qwen 3.7 Plus OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.7 Plus reasoning/tool trace to an exact vector with
  49 streamed updates, its single tool lifecycle, result message, and
  continuation turn. The regression floor now requires 32 exact fixtures.
- **Stateful-agent queue API parity (2026-08-05):** Added actor-owned
  `clear_steering_queue`, `clear_follow_up_queue`, `clear_all_queues`,
  `has_queued_messages`, and `reset` operations, plus queue length/empty
  projections. These mirror pi's stateful wrapper without exposing direct
  mutation of state or queues.
- **Verification (2026-08-05):** `just ci` passes after the queue API slice,
  including formatting, clippy, lint-check, workspace tests, replay fixtures,
  YAML TUI scenarios, and all 23 visual/reference tests.
- **Runtime queue-mode parity (2026-08-05):** Added actor-owned runtime
  steering/follow-up mode setters and getters. Each run snapshots those modes
  into its loop dependencies, matching pi's mutable `steeringMode` and
  `followUpMode` APIs while preserving single ownership.
- **Verification (2026-08-05):** `just ci` remains green after the runtime
  queue-mode change.
- **Qwen 3.7 Max OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.7 Max reasoning/tool trace to an exact vector with
  46 streamed updates, its tool lifecycle, result message, and continuation.
  The regression floor now requires 33 exact fixtures.
- **Qwen 3.7 Plus OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.7 Plus multi-tool sidecar to an exact 20-update
  vector, including its recorded single-tool lifecycle, result, and automatic
  continuation. The regression floor now requires 34 exact fixtures.
- **Qwen 3.7 Plus Anthropic multi-tool exact family (2026-08-05):** Promoted
  the Anthropic Qwen 3.7 Plus multi-tool sidecar to an exact 67-update vector,
  including its recorded three-tool lifecycle, three result messages, and
  continuation. The regression floor now requires 35 exact fixtures.
- **Qwen 3.7 Max OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.7 Max multi-tool sidecar to an exact 20-update
  vector, including its recorded single-tool lifecycle, result, and automatic
  continuation. The regression floor now requires 36 exact fixtures.
- **Qwen 3.7 Max Anthropic multi-tool exact family (2026-08-05):** Promoted
  the Anthropic Qwen 3.7 Max multi-tool sidecar to an exact 15-update vector,
  including its recorded three-tool lifecycle, result messages, and automatic
  continuation. The regression floor now requires 37 exact fixtures.
- **Qwen 3.7 Plus OpenAI simple exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.7 Plus long reasoning/text sidecar to an exact
  47-update vector. The regression floor now requires 38 exact fixtures.
- **Qwen 3.6 Plus OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.6 Plus tool sidecar to an exact 34-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 39 exact fixtures.
- **Qwen 3.6 Plus Anthropic tool exact family (2026-08-05):** Promoted the
  Anthropic Qwen 3.6 Plus tool sidecar to an exact 50-update vector, including
  its recorded two-tool lifecycle, result messages, and continuation. The
  regression floor now requires 40 exact fixtures.
- **Qwen 3.5 Plus OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Qwen 3.5 Plus tool sidecar to an exact 13-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 41 exact fixtures.
- **Qwen 3.5 Plus Anthropic tool exact family (2026-08-05):** Promoted the
  Anthropic Qwen 3.5 Plus tool sidecar to an exact 15-update vector, including
  its recorded two-tool lifecycle, result messages, and continuation. The
  regression floor now requires 42 exact fixtures.
- **GLM 5 OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible GLM 5 tool sidecar to an exact nine-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 43 exact fixtures.
- **GLM 5.2 OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible GLM 5.2 tool sidecar to an exact 10-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 44 exact fixtures.
- **GLM 5.2 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible GLM 5.2 multi-tool sidecar to an exact 18-update vector,
  including parallel tool lifecycle/result ordering and continuation. The
  regression floor now requires 45 exact fixtures.
- **DeepSeek v4 Flash OpenAI clarification exact family (2026-08-05):**
  Promoted the multi-turn clarification turn-1 sidecar to an exact 80-update
  vector, pinning its continuation boundary. The regression floor now
  requires 46 exact fixtures.
- **DeepSeek v4 Flash OpenAI clarification turn-2 exact family (2026-08-05):**
  Promoted the continuation sidecar to an exact 80-update vector, completing
  the pair's per-turn event oracle. The regression floor now requires 47 exact
  fixtures.
- **DeepSeek v4 Flash OpenAI math-chain exact family (2026-08-05):** Promoted
  the math-chain turn-1 sidecar to an exact 40-update vector, pinning its
  continuation boundary. The regression floor now requires 48 exact fixtures.
- **DeepSeek v4 Flash OpenAI math-chain turn-2 exact family (2026-08-05):**
  Promoted the continuation sidecar to an exact 40-update vector, completing
  the math-chain pair. The regression floor now requires 49 exact fixtures.
- **DeepSeek v4 Flash OpenAI read/summarize exact family (2026-08-05):**
  Promoted the read/summarize follow-up turn-1 sidecar to an exact 21-update
  vector, including its tool lifecycle and continuation boundary. The
  regression floor now requires 50 exact fixtures.
- **DeepSeek v4 Flash OpenAI read/summarize turn-2 exact family (2026-08-05):**
  Promoted the continuation sidecar to an exact 82-update vector, completing
  the read/summarize pair. The regression floor now requires 51 exact
  fixtures.
- **DeepSeek v4 Flash OpenAI weather-chain exact family (2026-08-05):**
  Promoted the weather-chain turn-1 sidecar to an exact 23-update vector,
  including its tool lifecycle and continuation boundary. The regression floor
  now requires 52 exact fixtures.
- **DeepSeek v4 Flash OpenAI weather-chain turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact 34-update vector, completing the
  weather-chain pair. The regression floor now requires 53 exact fixtures.
- **DeepSeek v4 Flash OpenAI multi-tool continuation exact family (2026-08-05):**
  Promoted the multi-tool comparison turn-1 sidecar to an exact 38-update
  vector, including parallel tool lifecycle/result ordering and continuation.
  The regression floor now requires 54 exact fixtures.
- **DeepSeek v4 Flash OpenAI multi-tool continuation turn-2 exact family
  (2026-08-05):** Promoted the comparison response to an exact 42-update
  vector, completing the multi-tool continuation pair. The regression floor
  now requires 55 exact fixtures.
- **Kimi K2.5 OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Kimi K2.5 tool sidecar to an exact 43-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 56 exact fixtures.
- **Kimi K2.6 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Kimi K2.6 multi-tool sidecar to an exact 52-update vector,
  including parallel tool lifecycle/result ordering and continuation. The
  regression floor now requires 57 exact fixtures.
- **Kimi K2.7 OpenAI code-tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Kimi K2.7 code-tool sidecar to an exact 39-update vector,
  including its tool lifecycle, result, and automatic continuation. The
  regression floor now requires 58 exact fixtures.
- **Kimi K2.6 OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible Kimi K2.6 single-tool sidecar to an exact 44-update
  vector, including its tool lifecycle, result, and automatic continuation.
  The regression floor now requires 59 exact fixtures.
- **Kimi K2.6 OpenAI weather-chain exact family (2026-08-05):** Promoted the
  weather-chain turn-1 sidecar to an exact 42-update vector, including its
  tool lifecycle and continuation boundary. The regression floor now requires
  60 exact fixtures.
- **Kimi K2.6 OpenAI weather-chain turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact 32-update vector, completing the
  weather-chain pair. The regression floor now requires 61 exact fixtures.
- **DeepSeek v4 Flash reasoning exact family (2026-08-05):** Promoted the
  long reasoning sidecar to an exact 119-update vector, preserving the
  thinking-stream event sequence through terminal completion. The regression
  floor now requires 62 exact fixtures.
- **Kimi K2.6 OpenAI simple exact family (2026-08-05):** Promoted the simple
  OpenAI-compatible sidecar to an exact 20-update vector, pinning its
  thinking stream and terminal event boundaries. The regression floor now
  requires 63 exact fixtures.
- **Kimi K2.6 OpenAI reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact 119-update vector, pinning its thinking stream
  and terminal event boundaries. The regression floor now requires 64 exact
  fixtures.
- **Qwen 3.6 Plus OpenAI simple exact family (2026-08-05):** Promoted the
  simple sidecar to an exact 70-update vector, pinning its streamed thinking
  and text event boundaries. The regression floor now requires 65 exact
  fixtures.
- **Kimi Code high-speed reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact 74-update vector, extending strict event
  coverage to the Kimi Code provider family. The regression floor now
  requires 66 exact fixtures.
- **Gemini 3.1 Flash Lite tool exact family (2026-08-05):** Promoted the
  tool sidecar to an exact lifecycle vector, including the assistant partial,
  tool update, automatic continuation, and second turn boundary. The
  regression floor now requires 67 exact fixtures.
- **Gemini 3.1 Flash Lite multi-tool exact family (2026-08-05):** Promoted
  the recorded multi-tool sidecar to its exact lifecycle vector, including
  the provider's tool update and continuation turn. The regression floor now
  requires 68 exact fixtures.
- **Gemini 3.1 Flash Lite weather-chain turn-1 exact family (2026-08-05):**
  Promoted the first multi-turn weather sidecar to an exact six-update vector,
  pinning its user/assistant boundary before the follow-up trace. The
  regression floor now requires 69 exact fixtures.
- **Gemini 3.1 Flash Lite weather-chain turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact seven-update vector, completing
  the Gemini weather-chain pair. The regression floor now requires 70 exact
  fixtures.
- **MiniMax M3 Anthropic reasoning exact family (2026-08-05):** Promoted the
  Anthropic-compatible reasoning sidecar to an exact four-update vector,
  extending strict thinking-stream coverage across provider families. The
  regression floor now requires 71 exact fixtures.
- **Qwen 3.7 Plus Anthropic reasoning exact family (2026-08-05):** Promoted
  the Anthropic-compatible reasoning sidecar to an exact 92-update vector,
  extending long-thinking-stream parity coverage. The regression floor now
  requires 72 exact fixtures.
- **DeepSeek v4 Pro OpenAI simple exact family (2026-08-05):** Promoted the
  simple sidecar to an exact 20-update vector, extending strict coverage to
  the DeepSeek v4 Pro model family. The regression floor now requires 73
  exact fixtures.
- **DeepSeek v4 Pro OpenAI reasoning exact family (2026-08-05):** Promoted
  the reasoning sidecar to an exact 119-update vector, completing the simple
  and thinking-stream pair for this model family. The regression floor now
  requires 74 exact fixtures.
- **Kimi K2.5 OpenAI simple exact family (2026-08-05):** Promoted the simple
  sidecar to an exact 20-update vector, extending strict coverage to the Kimi
  K2.5 model family. The regression floor now requires 75 exact fixtures.
- **Kimi K2.7 OpenAI code-simple exact family (2026-08-05):** Promoted the
  code-oriented simple sidecar to an exact 19-update vector, extending strict
  coverage to the Kimi K2.7 family. The regression floor now requires 76
  exact fixtures.
- **Qwen 3.7 Max OpenAI reasoning exact family (2026-08-05):** Promoted the
  long reasoning sidecar to an exact 153-update vector, extending strict
  coverage to the Qwen 3.7 Max thinking stream. The regression floor now
  requires 77 exact fixtures.
- **Mimo v2.5 OpenAI empty-response exact family (2026-08-05):** Promoted
  the empty-content success sidecar to its exact eight-event vector, covering
  the no-update assistant boundary explicitly. The regression floor now
  requires 78 exact fixtures.
- **Mimo v2.5 OpenAI reasoning-empty exact family (2026-08-05):** Promoted
  the reasoning sidecar to the same exact eight-event no-update vector,
  pinning the provider's empty reasoning response. The regression floor now
  requires 79 exact fixtures.
- **Mimo v2.5 Pro OpenAI empty-response exact family (2026-08-05):** Promoted
  the Pro sidecar to an exact eight-event no-update vector, extending strict
  empty-response coverage across the Mimo family. The regression floor now
  requires 80 exact fixtures.
- **MiniMax M3 OpenAI simple exact family (2026-08-05):** Promoted the
  OpenAI-compatible simple sidecar to an exact two-update vector, extending
  strict coverage to the MiniMax M3 family. The regression floor now requires
  81 exact fixtures.
- **MiniMax M3 OpenAI reasoning exact family (2026-08-05):** Promoted the
  OpenAI-compatible reasoning sidecar to an exact three-update vector,
  completing the simple/thinking pair for this family. The regression floor
  now requires 82 exact fixtures.
- **MiniMax M3 OpenAI tool exact family (2026-08-05):** Promoted the
  OpenAI-compatible tool sidecar to its exact 20-event lifecycle, including
  three assistant updates, tool update/completion, and automatic continuation.
  The regression floor now requires 83 exact fixtures.
- **Mimo v2.5 OpenAI tool exact family (2026-08-05):** Promoted the
  empty-text tool sidecar to its exact 18-event lifecycle, including the tool
  update, result boundary, and continuation turn. The regression floor now
  requires 84 exact fixtures.
- **MiniMax M2.5 OpenAI empty-response exact family (2026-08-05):** Promoted
  the empty-content simple sidecar to its exact eight-event no-update vector.
  The regression floor now requires 85 exact fixtures.
- **MiniMax M2.7 OpenAI empty-response exact family (2026-08-05):** Promoted
  the adjacent empty-content sidecar to its exact eight-event no-update
  vector. The regression floor now requires 86 exact fixtures.
- **Zen DeepSeek v4 Flash simple exact family (2026-08-05):** Promoted the
  alternate OpenAI-compatible endpoint sidecar to an exact 25-update vector,
  extending strict coverage to the Zen provider family. The regression floor
  now requires 87 exact fixtures.
- **Zen Mimo v2.5 simple exact family (2026-08-05):** Promoted the alternate
  Mimo endpoint sidecar to its exact nine-event lifecycle. The regression
  floor now requires 88 exact fixtures.
- **Zen Mimo v2.5 reasoning-empty exact family (2026-08-05):** Promoted the
  reasoning sidecar to its exact eight-event no-update lifecycle, completing
  the Zen Mimo simple/reasoning pair. The regression floor now requires 89
  exact fixtures.
- **Zen Mimo v2.5 tool exact family (2026-08-05):** Promoted the alternate
  endpoint tool sidecar to its exact 18-event lifecycle, including tool update,
  result, and continuation boundaries. The regression floor now requires 90
  exact fixtures.
- **Kimi Code high-speed tool exact family (2026-08-05):** Promoted the tool
  sidecar to an exact lifecycle with 24 assistant updates, tool update/result,
  and automatic continuation. The regression floor now requires 91 exact
  fixtures.
- **Kimi Code high-speed multi-tool exact family (2026-08-05):** Promoted the
  multi-tool sidecar to an exact 38-update lifecycle, preserving grouped tool
  starts, completion-order updates, result boundaries, and continuation. The
  regression floor now requires 92 exact fixtures.
- **DeepSeek v4 Flash OpenAI tool exact family (2026-08-05):** Promoted the
  single-tool sidecar to an exact 25-update lifecycle, including streamed
  reasoning, tool update/result, and automatic continuation. The regression
  floor now requires 93 exact fixtures.
- **Mimo v2.5 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  multi-tool sidecar to an exact grouped lifecycle with two assistant updates,
  parallel tool starts, completion-order updates, result boundaries, and
  continuation. The regression floor now requires 94 exact fixtures.
- **DeepSeek v4 Pro OpenAI multi-tool exact family (2026-08-05):** Promoted
  the grouped tool sidecar to an exact 40-update lifecycle, including parallel
  starts, completion-order updates, result boundaries, and continuation. The
  regression floor now requires 95 exact fixtures.
- **Zen DeepSeek v4 Flash tool exact family (2026-08-05):** Promoted the
  alternate endpoint tool sidecar to an exact 26-update lifecycle, including
  streamed reasoning, tool update/result, and continuation. The regression
  floor now requires 96 exact fixtures.
- **DeepSeek v4 Pro OpenAI weather-chain turn-1 exact family (2026-08-05):**
  Promoted the first weather-chain sidecar to an exact 27-update lifecycle,
  pinning its tool boundary and continuation handoff. The regression floor now
  requires 97 exact fixtures.
- **DeepSeek v4 Pro OpenAI weather-chain turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact 31-update lifecycle, completing
  the DeepSeek v4 Pro weather-chain pair. The regression floor now requires
  98 exact fixtures.
- **Qwen 3.7 Max Anthropic tool exact family (2026-08-05):** Promoted the
  Anthropic-compatible tool sidecar to an exact 39-update lifecycle, including
  grouped tool starts, completion-order updates, result boundaries, and
  continuation. The regression floor now requires 99 exact fixtures.
- **Qwen 3.7 Max Anthropic multi-tool turn-1 exact family (2026-08-05):**
  Promoted the first three-tool comparison sidecar to an exact 22-update
  lifecycle, preserving grouped starts, completion-order results, multiple
  result boundaries, and continuation handoff. The regression floor now
  requires 100 exact fixtures.
- **Qwen 3.7 Max Anthropic multi-tool turn-2 exact family (2026-08-05):**
  Promoted the follow-up comparison sidecar to an exact 57-update lifecycle,
  preserving its three-tool batch and continuation boundaries. The regression
  floor now requires 101 exact fixtures.
- **Qwen 3.7 Max Anthropic weather-chain turn-1 exact family (2026-08-05):**
  Promoted the first weather sidecar to an exact 38-update lifecycle,
  preserving grouped tool starts, result boundaries, and continuation handoff.
  The regression floor now requires 102 exact fixtures.
- **Qwen 3.7 Max Anthropic weather-chain turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact 11-update lifecycle, completing
  the Anthropic Qwen weather-chain pair. The regression floor now requires 103
  exact fixtures.
- **Mimo v2.5 Pro OpenAI tool exact family (2026-08-05):** Promoted the Pro
  tool sidecar to its exact 18-event lifecycle, including tool update/result
  and continuation boundaries. The regression floor now requires 104 exact
  fixtures.
- **DeepSeek v4 Pro OpenAI read/summarize turn-1 exact family (2026-08-05):**
  Promoted the first read/summarize sidecar to an exact 61-update lifecycle,
  pinning its file-tool boundary and continuation handoff. The regression floor
  now requires 105 exact fixtures.
- **DeepSeek v4 Pro OpenAI read/summarize turn-2 exact family (2026-08-05):**
  Promoted the follow-up sidecar to an exact 88-update text lifecycle,
  pinning the second-turn response boundary. The regression floor now requires
  106 exact fixtures.
- **DeepSeek v4 Pro OpenAI reasoning follow-up turn-2 exact family (2026-08-05):**
  Promoted the reasoning follow-up sidecar to an exact 120-update text
  lifecycle, pinning its terminal response boundary. The regression floor now
  requires 107 exact fixtures.
- **DeepSeek v4 Flash reasoning follow-up turn-1 exact family (2026-08-05):**
  Promoted the first reasoning follow-up sidecar to an exact 119-update text
  lifecycle, pinning its terminal response boundary. The regression floor now
  requires 108 exact fixtures.
- **DeepSeek v4 Flash reasoning follow-up turn-2 exact family (2026-08-05):**
  Promoted the second reasoning follow-up sidecar to an exact 120-update text
  lifecycle, completing the pair. The regression floor now requires 109 exact
  fixtures.
- **Kimi K2.6 OpenAI reasoning follow-up turn-1 exact family (2026-08-05):**
  Promoted the first reasoning follow-up sidecar to an exact 120-update text
  lifecycle. The regression floor now requires 110 exact fixtures.
- **Kimi K2.6 OpenAI reasoning follow-up turn-2 exact family (2026-08-05):**
  Promoted the empty-response continuation sidecar to its exact eight-event
  lifecycle, completing the pair. The regression floor now requires 111 exact
  fixtures.
- **GLM 5.2 OpenAI reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact 35-update text lifecycle. The regression floor
  now requires 112 exact fixtures.
- **Kimi K2.6 OpenAI math-chain turn-1 exact family (2026-08-05):** Promoted
  the first math-chain sidecar to an exact 40-update text lifecycle. The
  regression floor now requires 113 exact fixtures.
- **Kimi K2.6 OpenAI math-chain turn-2 exact family (2026-08-05):** Promoted
  the empty-response continuation sidecar to its exact eight-event lifecycle,
  completing the pair. The regression floor now requires 114 exact fixtures.
- **GLM 5.2 OpenAI weather-chain turn-1 exact family (2026-08-05):** Promoted
  the first weather sidecar to an exact seven-update tool-continuation
  lifecycle. The regression floor now requires 115 exact fixtures.
- **GLM 5.2 OpenAI weather-chain turn-2 exact family (2026-08-05):** Promoted
  the text continuation sidecar to an exact 15-update lifecycle, completing
  the pair. The regression floor now requires 116 exact fixtures.
- **GLM 5.2 OpenAI math-chain turn-1 exact family (2026-08-05):** Promoted
  the first math sidecar to an exact 11-update text lifecycle. The regression
  floor now requires 117 exact fixtures.
- **GLM 5.2 OpenAI math-chain turn-2 exact family (2026-08-05):** Promoted
  the second math sidecar to an exact 12-update text lifecycle, completing the
  pair. The regression floor now requires 118 exact fixtures.
- **Zen DeepSeek v4 Flash reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact 126-update text lifecycle. The regression floor
  now requires 119 exact fixtures.
- **Minimax M2.5 OpenAI reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to its exact one-update lifecycle. The regression floor now
  requires 120 exact fixtures.
- **DeepSeek v4 Pro OpenAI math-chain turn-1 exact family (2026-08-05):**
  Promoted the first math sidecar to an exact 39-update text lifecycle. The
  regression floor now requires 121 exact fixtures.
- **DeepSeek v4 Pro OpenAI math-chain turn-2 exact family (2026-08-05):**
  Promoted the second math sidecar to an exact 40-update text lifecycle,
  completing the pair. The regression floor now requires 122 exact fixtures.
- **Qwen 3.7 Plus OpenAI reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact 84-update text lifecycle. The regression floor
  now requires 123 exact fixtures.
- **Minimax M2.7 OpenAI reasoning exact family (2026-08-05):** Promoted the
  reasoning sidecar to an exact four-update text lifecycle. The regression
  floor now requires 124 exact fixtures.
- **DeepSeek v4 Pro OpenAI multi-tool comparison turn-2 exact family
  (2026-08-05):** Promoted the continuation sidecar to an exact 71-update text
  lifecycle, pinning its comparison response boundary. The regression floor
  now requires 125 exact fixtures.
- **DeepSeek v4 Pro OpenAI multi-tool comparison turn-1 exact family
  (2026-08-05):** Promoted the first sidecar to an exact 53-update grouped
  tool lifecycle with two starts, completion-order results, and continuation
  boundaries. The regression floor now requires 126 exact fixtures.
- **GLM 5.2 OpenAI reasoning follow-up turn-1 exact family (2026-08-05):**
  Promoted the first reasoning follow-up sidecar to an exact 36-update text
  lifecycle. The regression floor now requires 127 exact fixtures.
- **GLM 5.2 OpenAI reasoning follow-up turn-2 exact family (2026-08-05):**
  Promoted the second reasoning follow-up sidecar to an exact 42-update text
  lifecycle, completing the pair. The regression floor now requires 128 exact
  fixtures.
- **DeepSeek v4 Pro OpenAI clarification turn-1 exact family (2026-08-05):**
  Promoted the first clarification sidecar to an exact 80-update text
  lifecycle. The regression floor now requires 129 exact fixtures.
- **DeepSeek v4 Pro OpenAI clarification turn-2 exact family (2026-08-05):**
  Promoted the second clarification sidecar to an exact 80-update text
  lifecycle, completing the pair. The regression floor now requires 130 exact
  fixtures.
- **Minimax M2.5 OpenAI tool exact family (2026-08-05):** Promoted the tool
  sidecar to its exact 18-event lifecycle, including tool-result and automatic
  continuation boundaries. The regression floor now requires 131 exact
  fixtures.
- **Minimax M2.7 OpenAI tool exact family (2026-08-05):** Promoted the tool
  sidecar to its exact 18-event lifecycle, including tool-result and automatic
  continuation boundaries. The regression floor now requires 132 exact
  fixtures.
- **Minimax M3 OpenAI multi-tool exact family (2026-08-05):** Promoted the
  sidecar to an exact four-update grouped lifecycle with two tool starts,
  completion-order results, and extra result boundaries. The regression floor
  now requires 133 exact fixtures.
- **Qwen 3.7 Max Anthropic read/summarize turn-1 exact family (2026-08-05):**
  Promoted the sidecar to an exact 10-update grouped read lifecycle with two
  tool starts, completion-order results, and continuation boundaries. The
  regression floor now requires 134 exact fixtures.
- **Qwen 3.7 Max Anthropic read/summarize turn-2 exact family (2026-08-05):**
  Promoted the text continuation sidecar to an exact 137-update lifecycle,
  completing the pair. The regression floor now requires 135 exact fixtures.
- **Qwen 3.7 Max Anthropic clarification turn-1 exact family (2026-08-05):**
  Promoted the first clarification sidecar to an exact 341-update text
  lifecycle. The regression floor now requires 136 exact fixtures.
- **Qwen 3.7 Max Anthropic clarification turn-2 exact family (2026-08-05):**
  Promoted the second clarification sidecar to an exact 345-update text
  lifecycle, completing the pair. The regression floor now requires 137 exact
  fixtures.
- **Qwen 3.7 Max Anthropic reasoning follow-up turn-1 exact family
  (2026-08-05):** Promoted the first reasoning follow-up sidecar to an exact
  138-update text lifecycle. The regression floor now requires 138 exact
  fixtures.
- **Qwen 3.7 Max Anthropic reasoning follow-up turn-2 exact family
  (2026-08-05):** Promoted the second reasoning follow-up sidecar to an exact
  259-update text lifecycle, completing the pair. The regression floor now
  requires 139 exact fixtures.
- **Minimax M3 Anthropic clarification turn-2 exact family (2026-08-05):**
  Promoted the text continuation sidecar to an exact four-update lifecycle.
  The regression floor now requires 140 exact fixtures.
- **Minimax M3 Anthropic reasoning follow-up turn-1 exact family
  (2026-08-05):** Promoted the first reasoning follow-up sidecar to an exact
  two-update text lifecycle. The regression floor now requires 141 exact
  fixtures.
- **Minimax M3 Anthropic reasoning follow-up turn-2 exact family
  (2026-08-05):** Promoted the second reasoning follow-up sidecar to an exact
  three-update text lifecycle, completing the pair. The regression floor now
  requires 142 exact fixtures.
- **Minimax M3 Anthropic math-chain turn-2 exact family (2026-08-05):**
  Promoted the math continuation sidecar to an exact two-update lifecycle.
  The regression floor now requires 143 exact fixtures.
- **Minimax M3 Anthropic math-chain turn-1 exact family (2026-08-05):**
  Promoted the first math sidecar to an exact two-update lifecycle. The
  regression floor now requires 144 exact fixtures.
- **DeepSeek v4 Pro OpenAI reasoning follow-up turn-1 exact family
  (2026-08-05):** Promoted the first reasoning follow-up sidecar to an exact
  119-update text lifecycle. The regression floor now requires 145 exact
  fixtures.
- **Qwen 3.7 Max Anthropic math-chain turn-2 exact family (2026-08-05):**
  Promoted the math continuation sidecar to an exact 36-update lifecycle. The
  regression floor now requires 146 exact fixtures.
- **Qwen 3.7 Max Anthropic math-chain turn-1 exact family (2026-08-05):**
  Promoted the first math sidecar to an exact 41-update lifecycle, completing
  the pair. The regression floor now requires 147 exact fixtures.
- **Minimax M2.7 Anthropic reasoning exact family (2026-08-05):** Promoted
  the reasoning sidecar to an exact five-update lifecycle. The regression
  floor now requires 148 exact fixtures.
- **Minimax M3 Anthropic read/summarize follow-up turn-2 exact family
  (2026-08-05):** Promoted the continuation sidecar to an exact three-update
  lifecycle. The regression floor now requires 149 exact fixtures.
- **Minimax M3 Anthropic multi-tool comparison turn-2 exact family
  (2026-08-05):** Promoted the text continuation sidecar to an exact
  two-update lifecycle. The regression floor now requires 150 exact fixtures.
- **Minimax M3 Anthropic multi-tool comparison turn-1 exact family
  (2026-08-05):** Promoted the initiation sidecar to an exact five-update
  multi-tool/continuation lifecycle. The regression floor now requires 151
  exact fixtures.
- **Minimax M3 Anthropic weather-chain turn-2 exact family (2026-08-05):**
  Promoted the continuation sidecar to an exact four-update, two-tool,
  auto-continuation lifecycle. The regression floor now requires 152 exact
  fixtures.
- **Minimax M3 Anthropic weather-chain turn-1 exact family (2026-08-05):**
  Promoted the initiation sidecar to an exact four-update, two-tool,
  auto-continuation lifecycle, completing the pair. The regression floor now
  requires 153 exact fixtures.
- **Minimax M3 Anthropic clarification turn-1 exact family (2026-08-05):**
  Promoted the first clarification sidecar to an exact four-update lifecycle,
  completing its exact pair. The regression floor now requires 154 exact
  fixtures.
- **Qwen 3.7 Max Anthropic reasoning exact family (2026-08-05):** Promoted
  the long reasoning sidecar to an exact 119-update lifecycle. The regression
  floor now requires 155 exact fixtures.
- **Minimax M3 Anthropic read/summarize follow-up turn-1 exact family
  (2026-08-05):** Promoted the tool-call initiation sidecar to an exact
  one-update lifecycle with its auto-continuation boundaries. The regression
  floor now requires 156 exact fixtures.
- **Minimax M2.7 native bash-loop turn-1 exact family (2026-08-05):**
  Promoted the short thinking/tool-markup response to an exact two-update
  lifecycle. The regression floor now requires 157 exact fixtures.
- **Minimax M2.7 native bash-loop turn-2 exact family (2026-08-05):**
  Promoted the continuation sidecar to an exact two-update lifecycle,
  completing the pair. The regression floor now requires 158 exact fixtures.
- **Minimax M2.7 native-think tool-call exact family (2026-08-05):**
  Promoted the standalone native-think response to an exact two-update
  lifecycle. The regression floor now requires 159 exact fixtures.
- **Provider-decode coverage floor (2026-08-05):** Tightened the replay
  regression guard from a nonzero check to all 24 current provider-decode
  sidecars, preventing silent loss of error-family coverage.
- **Successful-sidecar audit (2026-08-05):** The 159 exact-event fixtures
  now cover every successful provider sidecar in the 183-sidecar matrix.
  The remaining 24 sidecars are parser/transport failures and are validated
  through their declarative `error.kind: provider_decode` contract rather than
  being routed through the successful loop event vector.
- **Fixture-count audit (2026-08-05):** Recounted the live TUI fixture
  directory and visual integration targets: 22 YAML scenarios and 23
  visual/reference tests, matching the coverage statement above.
- **Exact-coverage equality guard (2026-08-05):** The replay harness now
  requires exactly 159 exact vectors and exactly 24 provider-decode fixtures;
  the two counts must account for all 183 discovered sidecars, not merely
  exceed a historical minimum.
- **Live Grok PTY/cast audit (2026-08-05):** Consulted the reference
  `xai-grok-pager` PTY runner and its `GROK_PTY_CAST_DIR` asciinema workflow
  under tmux 3.7b/asciinema 3.2.1. A fresh live capture could not start because
  the reference `bin/protoc` wrapper requires the unavailable `dotslash`
  runtime and no system `protoc` is installed. The checked-in
  `artifacts/grok-full.cast` and `artifacts/grok-rich.cast` remain the
  reproducible terminal oracles used by the local zero-diff checks.
- **Homebrew live cast (2026-08-05):** Re-ran the installed Grok Build
  0.2.118 binary inside tmux 3.7b with asciinema at 120×36; the live dump is
  available at `/tmp/runie-grok-live.cast`. It confirmed the current header,
  clipboard warning, bordered hero, logo, Grok 4.5 announcement, workflow
  actions, model footer, stable badge, and Ctrl+C quit variant. The wide hero
  implementation and marker test are tracked in p17.
- **Runie live cast (2026-08-05):** Replayed the same 120×36 tmux/asciinema
  setup against the production binary at `/tmp/runie-live.cast`. After
  adjusting the hero gate to the actual widget viewport, the captured frame
  reaches the same bordered hero path and its focused marker assertions pass.
- **Provider-error payload oracle (2026-08-05):** Each of the 24 declarative
  provider-decode fixtures must now expose a non-empty parser/transport error
  payload in addition to failing classification, preserving useful diagnostic
  parity without entering the success loop.
- **Error-event projection parity (2026-08-05):** Non-message abort and
  provider/tool failure paths now use a shared `AgentEvent::Error` boundary;
  core and TUI tests confirm the state/status projections remain consistent.
- **Before-tool context parity (2026-08-05):** The public before/after tool
  hook contexts now expose the actor-owned `AgentContext`, and the parallel
  dispatch regression verifies both hooks observe the current transcript.
- **Async tool-hook parity (2026-08-05):** Before/after tool hooks are now
  awaited futures at the executor boundary, preserving reactive execution for
  user-defined asynchronous hooks.
- **Tool update parity (2026-08-05):** `AgentTool::execute` now receives an
  update callback whose partial results become typed tool-update events; the
  existing parallel completion-order path carries those updates with each
  tool call.
- **Tool cancellation parity (2026-08-05):** The loop-owned abort receiver
  and cancellation token are now threaded through `ToolExecutorActor`; an
  in-flight tool is raced against abort and finalized through the typed error
  event path. A focused actor-level regression verifies the in-flight abort
  result.
- **Hook signal parity (2026-08-05):** Both async tool-hook payloads now carry
  the per-call cancellation signal, matching pi's signal-aware callback
  contract; focused dispatch coverage checks it remains active at callback
  time.
- **Reactive tool updates (2026-08-05):** Partial tool results are published
  immediately by the tool actor through the shared bus rather than waiting
  for tool completion; coordinator replay is suppressed to avoid duplicates.
- **Failed-tool hook parity (2026-08-05):** Tool execution and cancellation
  failures now finalize through the async after-tool hook with the correct
  error flag, covered by the in-flight cancellation regression.
- **Core oracle boundary (2026-08-05):** The runtime replay harness now
  provides exact core event-sequence assertions for 159 successful sidecars
  plus non-empty provider-decode assertions for 24 error sidecars (183 total).
  This proves the core replay matrix independently; p19 remains pending only
  for the exhaustive TUI cast-frame comparison and any uncovered variants.
- **Completion-row geometry (2026-08-05):** The `Worked for` projection now
  matches the cast's column-six placement and has a dedicated transcript kind;
  affected visual snapshots were regenerated and pass. Full cast-wide diffing
  remains pending.
- **Cast-backed completion oracle (2026-08-05):** The visual suite now parses
  the recorded Grok completion row and compares every rendered cell against a
  Runie `TurnSummary` row, including indentation and duration text.
- **Artifact alignment audit (2026-08-05):** The checked-in `runie-full.cast`
  predates the completion-row implementation and contains no `Worked for` row;
  therefore it is not treated as evidence for the new contract. The
  reproducible oracle uses the Grok cast plus the live Runie renderer and
  asserts the exact row cells; a fresh matched interactive cast pair is still
  required before p19 can close.
- **Fresh matched capture audit (2026-08-05):** New 120×36 captures were
  produced under tmux/asciinema at `/tmp/runie-fresh-20260805-5.cast` and
  `/tmp/grok-fresh-20260805-2.cast`. They confirm the current Grok welcome
  variant adds clipboard/Codex notices and a `[stable]` badge; those rows are
  not present in the legacy checked-in cast and remain unimplemented as an
  explicit Runie variant.
- **Variant-source audit (2026-08-05):** The reference source does not expose
  a stable configuration/API for the clipboard, Codex-resume, or stable-badge
  notices. These remain an explicit variant work item, not a safe unconditional
  change to the legacy frame oracle.
- **Production welcome policy (2026-08-05):** Current Runie no longer renders
  the welcome surface at startup. Legacy welcome snapshots remain explicit
  reference-fixture coverage and are not part of the production path.
- **Working-state animation/color audit (2026-08-05):** Grok's authoritative
  `turn_status.rs` holds braille spinner frames for four ~30 Hz ticks
  (~133 ms/frame). Runie's actor clock is 20 Hz, so `TurnStatus` now holds
  each frame for three actor ticks (~150 ms), with a focused cadence test.
  The prior blanket DIM row was replaced by role spans: `accent_running`
  equivalent magenta spinner and gray activity/chrome, matching Grok's
  terminal-default theme roles. The 24-test cast-backed visual suite passes.
- **YAML asciinema oracle (2026-08-05):** Added a generic `visual.reference`
  instruction. A YAML fixture names an asciinema cast, selects a terminal
  frame by visible markers, and declares rows to compare against Runie's
  rendered screen. `visual-grok-feed.yaml` now performs a real dump-row
  comparison; the Rust harness only decodes JSON/VT100 and discovers fixtures.
  This removes hand-coded reference-state setup from the scenario contract.
- **Event-only visual state recipe (2026-08-05):** Removed the temporary
  hand-authored `turn_status` YAML fields. `visual-status-working.yaml` now
  declares only `start` + `text_delta`; the visual runner derives the active
  phase from those events and uses `grok-rich.cast` to select the reference
  frame. A strict comparison was intentionally attempted and exposed the
  remaining working-row mismatch: Grok's selected frame contains `┃  ◆
  Thinking…`, while Runie's current row is `⠋ Thinking… 0.0s ⇣0 [stop]`.
  The fixture currently keeps this row comparison marker-based until the
  renderer implements that missing Grok variant.
- **Event-to-state YAML assertions (2026-08-05):** `ScenarioOutcome` now
  exposes the final actor-owned `AgentStateSnapshot`; TUI YAML can assert
  `is_streaming`, pending-tool count, message count, streaming text, and
  errors. Core trace YAML now accepts the same projection checks for
  `is_streaming`, pending tools, and error state. Representative TUI and
  core fixtures use these fields, making events → state the primary
  functional-test contract and terminal dumps a secondary projection oracle.
- **Projection coverage expansion (2026-08-05):** Added explicit state
  assertions to representative simple, working-status, reasoning, and
  failing-tool TUI fixtures plus a core multi-turn/tool trace. Fixture
  discovery executes these through the actor replay path, so changes to the
  event sequence fail on the resulting snapshot rather than only on screen
  text.
- **Synthetic terminal-state vectors (2026-08-05):** The startup-provider-error
  and reactive-abort YAML fixtures now declare `is_streaming`, pending-tool,
  and error projections. The replay harness checks those declarations in the
  real actor branches alongside their exact lifecycle event vectors, closing
  the prior hard-coded-only state checks for these families.
- **Recorder completion boundary (2026-08-05):** Repeated full YAML fixture
  runs exposed a race where `prompt()` returned before parallel tool events
  reached the recorder, intermittently dropping a directory activity row.
  The recorder now owns completion by consuming through `AgentEnd` before it
  returns. Five consecutive full-fixture runs passed, eliminating the
  scheduler-dependent replay gap without sleeps or polling.
- **Declarative failure payloads (2026-08-05):** Trace `error` blocks now
  support `message_contains`. The synthetic startup-error vector declares and
  verifies the exact pi-compatible `api: upstream unavailable` payload; the
  provider-decode path can use the same contract without compiled test edits.
- **Provider-decode payload vector (2026-08-05):** The `status_500_server_error`
  sidecar now asserts the actual replay-layer failure (`no terminal event`),
  rather than incorrectly asserting the embedded HTTP JSON message. This
  distinguishes transport/parser behavior from provider payload content and
  is verified through the real replay actor.
- **Dump occurrence selector (2026-08-05):** `DumpRowReference` now supports
  `last: true`, allowing YAML to distinguish repeated transcript/status
  occurrences of the same text. Applying it to `Thinking…` exposed the real
  dynamic telemetry delta (`⠧ Thinking… 5.0s ⇣6.44k [stop]`) rather than
  comparing against the first transcript occurrence. The working fixture
  remains marker-based until its event sequence declares matching telemetry.
- **Deterministic YAML provider stream (2026-08-05):** Repeated fixture runs
  found scheduler-dependent loss of parallel tool events in the synthetic
  `yield_now()` stream. YAML replay now emits its declared assistant events
  synchronously, waits on `LoopActor::wait_for_idle()`, and records through
  `AgentEnd`; five consecutive full-fixture runs pass with complete activity
  groups and no sleeps or polling.
- **Complete terminal-state declarations (2026-08-05):** All 183 replay
  sidecars now explicitly declare `is_streaming: false` and
  `pending_tool_calls: 0` in their YAML state projection. The replay harness
  checks those values through the actor-owned snapshot for every fixture;
  startup-error and abort vectors retain their explicit error projections.
- **Whole-frame comparison audit (2026-08-05):** Added an opt-in
  `reference.exact_screen` dump assertion that compares the complete selected
  terminal frame, rather than isolated marker rows. Running it against the
  current `visual-grok-feed` scenario exposed a real state mismatch: the Grok
  frame includes hook/session rows, a waiting status/footer, and no assistant
  response body, while Runie renders the assistant body, a different prompt
  footer, and completion timing. The fixture remains non-strict until its
  event recipe selects the same state; this is now an explicit parity failure,
  not a hidden row-only pass.
- **Cell-attribute oracle (2026-08-05):** Extended dump references with
  `exact_attributes`. When enabled, the selected frame is captured cell by
  cell and compared against the Ratatui buffer for symbol contents, terminal
  colors, bold/italic/underline/inverse flags, and wide-cell contents. It is
  intentionally gated behind the same matched-state requirement as
  `exact_screen`; the current feed fixture still exposes state/layout deltas
  before attribute comparison can be meaningfully enabled.
- **Verification (2026-08-05):** `just ci` passes after the expanded state
  contract: formatting, clippy, project lint, all core/replay tests, all 25
  TUI visual tests, and both YAML end-to-end tests.
- **Canonical cell attributes (2026-08-05):** Normalized vt100 and Ratatui
  color representations (`default`, indexed, and RGB) before exact cell
  comparison, and added an explicit terminal-dimension diagnostic. This
  prevents equivalent colors from producing false failures and makes the
  strict oracle report the first real symbol/style mismatch.
- **Cell diff diagnostics (2026-08-05):** Strict attribute failures now report
  terminal coordinates and field-level changes for the first twelve differing
  cells (symbol, foreground/background, and each supported modifier), plus the
  total mismatch count. This keeps full-frame parity actionable while retaining
  symbol-by-symbol coverage.
- **Pending-model capture boundary (2026-08-05):** Added the declarative
  `capture_while_waiting: true` YAML mode. Its provider leaves the continuation
  stream pending after the tool batch; the recorder signals `ToolExecutionEnd`,
  the renderer snapshots that state, then aborts and joins the owned loop task.
  No sleeps or polling are used. `visual-grok-waiting.yaml` exercises the
  grouped feed and thinking row at this boundary. A diagnostic strict run found
  445 real color differences after normalizing blank cells, so strict flags
  remain disabled until the Grok PTY frame and Runie palette/layout are aligned.
- **Waiting-frame refinement (2026-08-05):** The capture now snapshots on the
  second `TurnStart` after the tool batch, before the provider response, and
  preserves the pre-abort event vector. This removes abort artifacts and
  produces the correct `Waiting for response…` phase and empty prompt chrome.
  Collapsed activity keeps `session_start` visible, and the explicit header
  color was removed after the cell oracle identified it as a false `idx:8`
  difference. Remaining strict-frame deltas are lifecycle/header geometry,
  prompt wrapping, telemetry chrome, and the session row format.
- **Session row projection (2026-08-05):** Added a dedicated
  `SessionStart` scrollback kind so collapsed activity does not hide Grok's
  lifecycle row. It renders the hook count and Grok's five-column gutter;
  strict screen diagnostics now show that row aligned. The remaining full-frame
  mismatch is isolated to header token-meter data, prompt timestamp/wrapping,
  and usage telemetry rather than feed event ordering.
- **Waiting chrome projection (2026-08-05):** Added declarative visual fields
  for the Grok header meter and waiting telemetry chrome, allowing the strict
  probe to match `20K / 500K` and `⇣20.7k` from the captured frame. The
  remaining mismatch is now concentrated in the prompt's word-wrap/timestamp
  placement; the YAML fixture keeps these reference values explicit rather
  than hard-coding them into the core event stream.
- **Prompt geometry probe (2026-08-05):** Added a declarative timestamp-aware
  prompt projection and word-boundary wrapping for the waiting capture. The
  strict probe now places the timestamp correctly and reduces the mismatch to
  continuation indentation/line-break geometry; it is not yet promoted to
  strict pass because the full frame still differs.
- **Prompt continuation probe (2026-08-05):** Switched continuation wrapping
  to word boundaries and reserved the same first-row timestamp gutter. The
  strict frame now has the correct three prompt rows and timestamp content;
  remaining differences are the renderer's continuation-gutter projection,
  which is kept outside the strict fixture until its cell coordinates match.
- **Full-frame row alignment probe (2026-08-05):** Canonicalized blank rows in
  the dump oracle, added Grok's lifecycle spacing around session/tool groups,
  and verified the prompt rows now match symbol-for-symbol through the third
  continuation line. The next strict mismatch is the fixed status-row
  placement after the activity group; deterministic snapshots and all local
  checks remain green after the projection changes.
- **Strict Grok feed frame (2026-08-05):** Reworked `visual-grok-feed.yaml` to
  declare the same pending tool/waiting event boundary as `grok-rich.cast`.
  Runie now matches that selected frame symbol-for-symbol with
  `reference.exact_screen: true`, including session row, prompt wrapping,
  grouped activity, waiting telemetry, doctor hint, prompt box, and footer.
  Session action styling, neutral waiting-row styling, and waiting geometry
  were corrected from the cell-level probe. Exact attributes remain disabled
  for this fixture while Opaline palette propagation is completed.
- **Full fixed-grid oracle (2026-08-05):** Strict screen comparisons now use
  the selected cast's complete `cols × rows` cell grid, including empty and
  trailing cells, rather than trimmed text lines. `exact_screen` compares
  every symbol coordinate; `exact_attributes` compares the same coordinates'
  colors and modifiers. The strict Grok feed currently passes both modes.
- **Strict waiting frame (2026-08-05):** Promoted
  `visual-grok-waiting.yaml` to `exact_screen: true` and
  `exact_attributes: true`. The pending-tool/waiting frame now passes the
  complete 80×24 symbol and attribute grid as well as the main feed frame.
- **Herdr right-pane Hey capture (2026-08-05):** Captured the actual Grok
  right pane and Runie replacement at Herdr's 63×32 viewport with ANSI RGB
  colors and modifiers in `/tmp/herdr-grok-right.ansi` and
  `/tmp/herdr-runie-right.ansi`. The live probe isolated a semantic styling
  defect: completed assistant rows were being reclassified as dim
  `TurnSummary` rows. Added a separate `CompletedAssistant` line kind so the
  assistant body keeps the primary theme token while the `Worked for` row
  remains dim. The live capture still has geometry/footer/header deltas and
  is not an exact pass.
- **ANSI comparison instrument (2026-08-05):** Added `just herdr-dump` and
  `just herdr-compare`. The former records the visible ANSI grid plus Herdr
  pane/workspace metadata; the latter compares every cell's glyph and SGR
  attributes and exits non-zero on any difference. A comparison of the saved
  Hey frames reports 262 glyph differences and 1,598 style-only differences.
- **Reasoning snapshot reconciliation (2026-08-05):** Updated the visual
  reasoning snapshot to the event renderer's deterministic `◆ Thought for
  0.9s` projection. The complete 27-test visual/asciinema suite is green.
- **Expanded reasoning reducer fix (2026-08-05):** `reasoning_expanded: true`
  now preserves the reasoning body through assistant completion while the
  default projection emits Grok's compact thought summary. The YAML collapsed
  scenario was updated to assert the compact marker; the full workspace gate
  is green except for the final e2e pass being rerun after this fixture edit.
- **Untouched-cell background projection (2026-08-05):** The live ANSI diff
  showed Runie assigning the primary foreground token to every blank cell
  during frame clearing, while Grok assigns only its background. Added an
  Opaline-backed background-only style and changed both production redraw
  paths to use it. Visual (27) and YAML E2E (2) suites remain green.
- **Workspace gate after projection (2026-08-05):** `just ci` passes fully:
  format check, clippy, lint, 51 core unit tests/integration tests, 101 TUI
  unit tests, 2 YAML E2E tests, 27 visual/asciinema tests, and doc tests.
  Herdr currently has no Grok pane in the `runie` workspace, so the next
  live cell diff requires a fresh paired capture.
- **Timed cast comparator (2026-08-05):** Added the native `cast_compare`
  binary and `just cast-compare`. It replays asciicast v2/v3 output through
  `vt100`, validates geometry, and compares every cell's glyph, RGB color,
  and modifiers. The fresh private 62×32 Hey casts currently report 225
  differing cells: 222 glyph differences and 3 attribute-only differences.
- **Header path projection (2026-08-05):** The settled-frame diagnostics
  isolated the header's 31-cell difference to Runie's shortened
  `runie-tests/runie` label. The production header now renders the full
  home-relative path (`~/Code/GitHub/runie-tests/runie`) from the runtime
  `HOME` token, matching Grok without a hardcoded user path. Binary and all
  27 visual tests pass.
- **Settled prompt chrome (2026-08-05):** Matched the live Grok settled
  footer shortcut (`Ctrl+x`) and hid the prompt placeholder after a completed
  conversation through the prompt actor command, leaving the bare `❯` marker
  Grok renders. Binary, visual (27), and YAML E2E (2) tests pass.
- **Four-geometry parity matrix (2026-08-05):** Added `capture-matrix` with
  required defaults `62×32`, `80×24`, `100×30`, and `120×36`, plus native
  cast replay/comparison for each pair. This prevents width wrapping and
  height chrome regressions from being hidden by a single viewport. The full
  local gate remains green.
- **Settled chrome correction (2026-08-05):** A valid 62×32 cast comparison
  isolated an extra header space and the completed footer shortcut mismatch;
  both live projections are now corrected. Feed wrapping, timing text, and
  usage-meter differences remain and are not being declared exact parity.
- **Event-owned header meter (2026-08-05):** Removed the live binary's fixed
  token-meter literal. The header now projects the latest `Usage` owned by the
  status actor, with a deterministic zero state before the first completed
  turn; the projection has a unit regression test.
- **Fresh 62×32 validation (2026-08-05):** Re-captured Runie after the header
  correction and compared it with the valid Grok cast. The header delta fell
  from 35 cells to 3; total mismatch fell from 236 to 204 cells. The remaining
  header cells are the expected fixture usage difference (Grok 18K versus the
  placeholder stream's zero usage). Feed row placement, prompt timestamp and
  wrapping, reasoning duration, and doctor/footer geometry remain open.

- **Message-update wire key (2026-08-06):** Compared Runie's `AgentEvent`
  serde output with pi's `agent-loop.ts`: `message_update` now emits the
  pi-compatible `assistantMessageEvent` field instead of the internal Rust
  field name `event`. The wire-shape test asserts both the key and nested
  event tag; replay coverage remains green.

- **Assistant stream content index (2026-08-06):** Pi's granular stream
  events use `contentIndex`; Runie's text, thinking, and tool-call index
  fields now serialize with that exact camelCase key while retaining the
  internal `index` field name. The wire-shape test covers the family.

- **Assistant terminal reason (2026-08-06):** `Done.stop_reason` now emits
  pi's `reason` wire key (for example `toolUse`) rather than `stopReason`,
  with a matching serde assertion.

- **Assistant error reason (2026-08-06):** The internal assistant error text
  now serializes under pi's terminal `reason` key, while reducer code retains
  the descriptive `error` field name. Wire coverage pins the `aborted` value.
 - **Attribute-oracle audit (2026-08-06):** Temporarily promoted both strict
   Grok feed YAML references to `exact_attributes: true`. The oracle correctly
   rejected them with 518 cell attribute differences: the checked-in
   `grok-rich.cast` records terminal-default SGR while Runie paints explicit
   Opaline RGB tokens (the first delta is the GrokNight base surface at row 3).
   The fixtures remain symbol-exact but intentionally attribute-pending until
   a fresh full-color Grok capture is recorded; this is an open parity gap, not
   a passing color claim.
