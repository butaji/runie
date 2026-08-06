# p25 — Parity instrument audit

## Review role

This task is the independent review pass for parity work: every implementation
change must be checked against the capture/replay instruments before being
called parity.

## Current instruments

- `scripts/tmux-asciinema-capture.sh`: private tmux PTY, fixed geometry,
  bounded prompt/completion probes, `.cast` plus raw replay stream.
- `scripts/capture-matrix.sh`: 62×32, 80×24, 100×30, and 120×36 matrix.
- `scripts/compare-matrix.sh`: paired Grok/Runie matrix gate over all four
  geometries; each pair must pass the full-cell `cast_compare` check. It now
  reports every geometry before returning failure, so one mismatch cannot hide
  evidence from the remaining viewport sizes.
- `cast_compare --dump`: VT replay with glyph, fg, bg, bold, italic,
  underline, inverse, geometry, and full-cell JSON.
- Capture manifests (`*.meta.json`) are now validated automatically when both
  paired casts have them. The comparator rejects incomplete metadata and
  mismatched probe prompt, geometry, `TERM`, or `COLORTERM` before reporting
  cell differences; Grok/Runie command strings remain intentionally allowed to
  differ.
- **Capture command delimiter correction (2026-08-06):** With no caller
  environment overrides, the command builder previously joined
  `COLORTERM=truecolor` directly to `grok`/Runie, producing an invalid command
  such as `truecolorgrok` and an empty cast. The delimiter is now explicit;
  `bash -n` plus a fresh Grok capture guards this path.
- **Fresh capture recheck (2026-08-06):** After the delimiter fix, isolated
  62×32 Grok and Runie `Hey` casts both completed with manifests and matching
  probe/terminal metadata. After replay preserves the pre-exit application
  frame, the strict final-screen comparison reports 176 glyph-only
  differences. The remaining rows expose real scenario differences: live Grok
  response text/timing, usage formatting, and a one-row thought/response
  placement shift. This is valid product evidence, not a capture-launch
  failure, and remains open for semantic alignment.
- **Fresh response alignment (2026-08-06):** The controlled live placeholder
  now uses Grok's current captured `Hey` response
  `Hey — what would you like to work on in runie?`; deterministic YAML
  fixtures retain their own event-owned responses.
- **Private tmux RGB/NO_COLOR correction (2026-08-06):** The host environment
  exports `NO_COLOR=1`, and the isolated tmux session did not advertise the
  `RGB` terminal feature. Grok's `/doctor --json` therefore reported
  `color.level: none` and emitted no SGR despite `COLORTERM=truecolor`. The
  capture helper now unsets `NO_COLOR` only inside the recorded command and
  sets `terminal-features ",*:RGB"` only on its private tmux session. It never
  edits the user's tmux configuration. A fresh 80×24 Grok capture then emitted
  277 foreground and 315 background truecolor sequences. This is a required
  prerequisite for any color/attribute parity claim.
- **Truecolor diagnostic classification (2026-08-06):** The first paired fresh
  truecolor capture compared Grok's live response with Runie's placeholder
  response and reported 738 differing cells (124 glyph, 614 attribute). The
  semantic event recipe is not matched, so this is diagnostic evidence rather
  than a renderer parity verdict; `exact_attributes` remains gated on matched
  state.
- **Live assistant placement correction (2026-08-06):** The assistant-start
  projection now emits `Thinking… → separator → response`, matching the
  captured Grok live frame after the aligned user row. The former leading
  separator placed the complete thought/response/completion block one row
  too low. This is isolated to the production live actor renderer;
  deterministic YAML replay retains the four-message mapping and its existing
  selection indices.
- **Fresh truecolor Hey projection (2026-08-06):** A paired 80×24 diagnostic
  showed Runie's live placeholder had `15.0K`, a missing settled-thought
  separator, and a wrapped assistant timestamp. The placeholder usage now
  matches the captured Grok `14K` value; the pure physical-row projection
  preserves Grok's blank row between thought and assistant; and the timestamp
  edge offset keeps `PM` on the same row. The fresh comparison improved from
  126 to 50 glyph differences. Remaining differences are dynamic timestamp,
  thought/worked elapsed values, and semantic color-role cells; no exact claim
  is made until those inputs are matched.
- **Frozen thinking clock (2026-08-06):** Added `RUNIE_PARITY_THINKING_MS` to
  the existing clock boundary. A fresh Grok frame's `0.8s` thought duration and
  `2.8s` worked duration can now enter the placeholder provider event
  deterministically. With timestamp, elapsed ticks, and thinking duration
  frozen together, the paired 80×24 diagnostic reduced glyph differences to 12
  before remaining semantic color-role differences.
- **Verification boundary (2026-08-06):** A fresh 62×32 comparison after the
  live placement fix reduced the mismatch from 144 glyph cells to 22 cells
  with the plain captured response, and the row-placement/terminal-color
  classes disappeared. The remaining cells are usage width, wall-clock
  prompt/assistant timestamps, worked elapsed value, and captured response
  style; these require shared frozen inputs before an exact-zero claim.
- **Same-event alternate-screen correction (2026-08-06):** Raw replay showed
  Grok writes its settled feed and `ESC[?1049l` in one PTY output event.
  `cast_compare` previously applied Grok's terminal clear before processing the
  exit sequence, erasing the application frame. Replay now recognizes the
  clear→alternate-exit boundary and stops before the shell clear, preserving
  the actual final application frame.
- `cast_compare --frames`: opt-in indexed frame replay. It retains the full
  cell/style grid after every output event and reports frame counts plus the
  first corresponding-frame divergence, making timing/frame alignment visible
  without weakening the final-screen gate.
  Consecutive events with an unchanged complete VT grid are coalesced, so the
  count represents visible terminal-state transitions rather than transport
  chunks; no cell is discarded from a retained frame.
  Indexed mismatches now also report the first differing frame cell and both
  complete cell attributes. The current 80x24 pair starts with Grok blank at
  `(0,0)` and Runie already showing the header (`c`, bold) there, confirming
  that marker-locked phase alignment must precede feed parity comparison.
- `cast_compare --frames-after=MARKER`: starts indexed replay independently at
  the first visible frame containing `MARKER`, removing startup-frame skew
  before comparing the same scenario phase.
- `cast_compare` now rejects mismatched terminal geometries before producing
  cell or row diagnostics, with a structured `geometry_mismatch` result. This
  prevents accidentally comparing unrelated captures (or different matrix
  sizes) from being mistaken for a visual parity report.
- YAML `reference.exact_screen`: strict reference-frame symbol oracle.
- YAML `reference.exact_attributes`: strict style/color oracle.
- YAML `reference.frame_after`: phase-locks selection to the first frame after
  a declarative marker, matching `cast_compare --frames-after` without putting
  frame timing logic in Rust fixtures.

The `visual-status-working.yaml` fixture now exercises this selector by
choosing the first `Thinking…` frame after Grok's `Starting session…` phase;
the fixture passes without a compiled frame index.

The paired matrix is an evidence tool, not a fixture substitute. Reference
casts must come from the same scenario and capture run; dynamic usage, timing,
and model output differences must be investigated before snapshots are
updated.

Fresh matrix capture audit (2026-08-05) produced valid paired casts at all
four geometries. Full-cell differences were: 62×32 — 182 cells; 80×24 — 226;
100×30 — 191; 120×36 — 194. Each report includes glyph and attribute counts;
the dominant deltas remain provider response/wrapping, reasoning placement,
usage/timestamps, and completion timing. The matrix now proves that these
differences persist across width and height rather than being a single-
viewport artifact.

The apparent full-mode user-vpad offset was experimentally moved from before
to after the user row. A fresh 80×24 run improved the live diff from 226 to
218 cells, but it failed the locked `grok-rich.cast` frame-81 exact oracle by
moving the deterministic user prompt one row too early. The change was
reverted; the cast-backed oracle remains authoritative until frame selection
and live capture phase are reconciled.

ANSI stream audit (2026-08-05) then confirmed Runie emits `SGR 1` around both
ready-footer shortcut labels. The comparator still reports those cells as
non-bold in its selected frame, so the 20 attribute-only footer differences
are now classified as a VT replay/frame-selection issue, not evidence that the
theme-token styling is absent. The raw stream must be reconciled with the
selected settled frame before changing widget styling.

**Same-size capture recheck (2026-08-06):** A valid 62×32 pair from the
current capture set was compared with `--frames-after=Worked for` and the
full-cell gate. It reported 142 final-screen deltas and one settled-frame
delta. The visible `/doctor` row exists only in Grok's diagnostic startup
surface (`xai-grok-pager/src/startup.rs`) and is correctly classified as
Grok-only because it has no Pi-core event/state mapping. The apparent footer
bold deltas came from an older Runie cast; current `render_live_ready_footer`
already emits bold shortcut spans, so a fresh same-revision capture is
required before changing styling.

The ready footer is now rendered only once in the live binary: the ready status
projection is not painted and then overwritten by the binary footer. The footer
segments are written directly with Opaline-derived styles, avoiding Ratatui
style-diff transitions caused by painting the same row twice. A fresh 80x24
capture reduced footer attribute differences from 25 to 9; glyph, timing, and
provider-content differences were unchanged. The remaining nine cells are
isolated to the first bold shortcut label and remain a separate backend-style
investigation.

The cast comparator no longer caches an arbitrary “last non-empty” frame. It
now compares the final application screen after replaying all output up to
alternate-screen exit, while retaining empty/trailing cells and attributes.
This removes a source of false style diagnostics without weakening exactness.

The indexed frame comparator was exercised against the existing 80x24 pair:
Grok produced 67 output frames and Runie 15, with the first corresponding
full-cell divergence at frame 2. This confirms that a future exact frame oracle
must align semantic application phases or capture timing before asserting
one-to-one frame identity.

The same indexed probe across the four saved matrix geometries reports:

| geometry | Grok frames | Runie frames | first indexed divergence |
| --- | ---: | ---: | ---: |
| 62x32 | 62 | 23 | 1 |
| 80x24 | 67 | 14 | 1 |
| 100x30 | 98 | 19 | 1 |
| 120x36 | 66 | 20 | 1 |

The mismatch is therefore capture-phase-wide, not a single viewport defect.

**Strict saved-cast probe (2026-08-06):** Replayed the preserved 62×32
same-run pair with `just cast-compare`. The comparator examined all 1,984
cells and reported 190 differences: 170 glyph differences and 20
attribute-only differences. The attribute coordinates are concentrated in the
footer shortcut row (Grok bold versus Runie non-bold), while the glyph rows
remain dominated by response text, reasoning placement, timestamps, usage,
and elapsed time. This is a real non-parity result, not an attribute-checking
gap; no fixture was weakened.

**Footer cell-width correction (2026-08-06):** The live binary's ready footer
advanced its write cursor with UTF-8 byte length, so the three-byte `│`
separator shifted subsequent segments by two terminal cells. The cursor now
advances by character cell count, and a binary regression asserts `Ctrl+x`
starts at the expected cell with bold styling. Full CI and all visual checks
remain green; a fresh same-run cast is still required to remeasure the old
saved-cast delta.

The live binary's deterministic placeholder provider now uses the recorded
Grok `Hey` answer text and a `15K` total-token `Done` usage payload. These
values flow through the normal provider/core event path; the renderer does not
special-case the text or mutate status state. This removes provider-content
and usage nondeterminism from the controlled parity scenario, leaving timing,
timestamps, wrapping, and footer style as independently measurable deltas.

**Fresh Runie recapture (2026-08-06):** A new isolated 62×32 Runie cast was
captured after the footer cell-width correction:
`/tmp/runie-runie-capture.VUt9J4/runie-hey.cast`. Compared with the preserved
Grok reference using the strict full-cell comparator, it reports 246 differing
cells (231 glyph differences and 15 attribute-only differences). The footer
separator spacing is now aligned; the remaining footer attributes are the
Grok-bold versus Runie-regular shortcut labels. The other deltas are visible
scenario differences: the captured Runie response is longer and wraps onto
different rows, its reasoning and completion rows occur later, and its prompt
timestamp is live rather than matching the reference. This confirms the
instrument catches real scenario differences and does not establish pixel
parity from a stale cast pair.

The raw recapture also confirms the footer diagnostic is serialization-level:
the live stream contains `SGR 1`, followed by a reset before the label text,
while the in-memory buffer regression still marks the cells bold. A focused
theme-token projection experiment and a full `Paragraph` span render produced
the same ANSI sequence and the same 15 attribute-only cells, so that change
was discarded. The remaining footer issue belongs in the terminal-diff
serialization/capture instrument, not in a hardcoded color or token change.

**Assistant gutter correction (2026-08-06):** The same fresh `Hey` cast exposed
that the completed-assistant timestamp branch split at the raw width boundary,
so a long first line could place the clock in the middle of a word. The
projection now reserves Grok's timestamp gutter, wraps at a word boundary, and
continues the remainder as normal feed text. The placeholder stream was also
corrected to the exact recorded Grok answer. The strict 62×32 probe changed
from 246 to 223 differing cells; the response text now matches, while the
remaining differences are feed vertical phase, live clock/elapsed values,
header usage, and footer ANSI attributes. A unit test locks the no-midword
timestamp invariant.

**Assistant-start row experiment (2026-08-06):** Full-cell row dumps showed a
duplicated blank separator before the thought summary in the isolated `Hey`
cast. Removing that separator reduced the isolated strict probe from 223 to
75 differing cells and aligned the complete answer block, but the workspace
YAML replay gate rejected the broad event-level change: the recorded tool-feed
frame then shifted by one row. The change was reverted. The result identifies
the next fix as a scenario-aware projection difference, not permission to
weaken the shared event mapping or its Grok tool fixture.

**No-tool-only separator projection experiment (2026-08-06):** Moving the
deduplication into `physical_rows` made the isolated 62×32 `Hey` cast report
75 differing cells (60 glyph, 15 attribute-only), with the entire assistant
response block matching Grok symbol-for-symbol. However, the generic rule
failed three strict full-mode Insta snapshots even though the YAML tool-feed
fixtures passed. It was reverted; no snapshot was regenerated. The next fix
must use an explicit scenario/state discriminator rather than infer behavior
from the absence of tool rows.

**Grok timing ownership audit (2026-08-06):** The authoritative Grok tracker
(`xai-grok-pager/src/acp/tracker.rs`) stores `last_thinking_elapsed_ms`, derives
it from server timestamps attached to thinking chunks, and freezes it when the
thinking block finishes. Empty pre-created thinking blocks are removed rather
than rendered as a fake `0.0s` summary. Runie's `AgentEvent::MessageUpdate` and
`MessageEnd` contracts currently carry no equivalent timing metadata, forcing
the renderer's `0.9s` placeholder and causing the remaining `0.2s`/`0.9s`
strict-cast mismatch. The next parity change belongs in the core event DSL and
actor-owned state projection, with YAML replay values for server timestamps;
the renderer should consume the resulting projection rather than hardcode a
duration.

**Thinking elapsed event contract (2026-08-06):** `ThinkingEnd` now carries
optional `elapsedMs` metadata and the core partial-reconstruction path
preserves it. Existing replay event sequences remain stable; the current
production bridge still needs to carry this value into the terminal assistant
projection. The field is optional for backward-compatible replays, with
existing fixtures retaining the deterministic fallback.

**Explicit settled no-tool phase (2026-08-06):** `FinalizeAssistant` now
records an explicit provider-timed/no-tool settled phase. The scrollback
projection uses it to remove only the duplicate pre-thought separator; generic
no-tool inference is avoided, so tool and legacy snapshot states remain
unchanged. Replay paths that provide the timing metadata align the isolated
feed phase with Grok, and the thought summary styling bolds only `Thought`,
matching Grok's cell attributes. The live provider bridge now carries timing
into the terminal assistant projection without adding marker events to the
existing replay sequence.

**Terminal assistant usage projection (2026-08-06):** Assistant usage and stop
reason are now consumed from the existing `AgentEvent::MessageEnd` payload by
the actor-owned status projection. This preserves the exact replay event
sequence while fixing the controlled `Hey` header from `0 / 500K` to the
expected `15K / 500K`.

**Final assistant timing bridge (2026-08-06):** The optional thinking duration
is now copied into the actor-owned final `AssistantMessage` while applying the
stream, then consumed from `MessageEnd` by the TUI renderer. This preserves
the exact bus sequence while allowing the live `Hey` scenario to render the
provider's `0.2s` thought duration and settled row phase. The strict 62×32
probe reached 41 differing cells before the final two-cell marker-style
correction; remaining deltas are live clocks, worked elapsed time, and footer
ANSI attributes.

**Settled capture boundary (2026-08-06):** The tmux/asciinema harness now waits
for `Worked for` together with the compact settled `Shift+Tab` footer and rejects
the active `Esc:cancel` footer. This prevents a capture from stopping between
the feed's terminal update and the status actor's settled projection. A fresh
62×32 capture now compares at 24 differing cells (24 glyph, zero attributes)
against the checked-in Grok cast; the remaining rows are dynamic timestamps
and worked elapsed time. The footer's 15 attribute-only cells disappeared after
the live GrokNight footer switched to terminal-default semantic styles, matching
the actor-owned status projection.

**Deterministic clock boundary (2026-08-06):** Live prompt timestamps now
come from the TUI clock boundary. Production defaults to wall time, while
capture/replay runs can set `RUNIE_PARITY_TIMESTAMP` to a Unix timestamp. The
value enters the user event at the boundary and is never read by reducers or
views, preserving actor/SSOT purity while making timestamp cells reproducible.

The same boundary now exposes `RUNIE_PARITY_ELAPSED_TICKS`. When set, the
status actor's reducer owns the frozen 20 Hz duration used by both `Worked for`
and active status chrome; when unset, animation advances exactly as before.
This makes both dynamic clock classes controllable without special-casing the
renderer or weakening full-cell assertions.

**Capture environment injection (2026-08-06):** `tmux-cast` now accepts a
space-separated `NAME=VALUE` environment argument and exports it in the private
tmux shell before starting asciinema. The timestamp override propagates through
the complete capture path and matches the Grok reference clock. A direct tmux
probe also renders the frozen `Worked for 1.9s`; the asciinema child currently
needs one more propagation check for the elapsed override before frozen final
screen parity can be accepted.

The command is now shell-quoted as one asciinema command, which fixes the
previous nested-quote failure: an elapsed-only capture reaches `Worked for
1.9s`, and a timestamp-only capture reaches `11:20 PM`. Direct tmux accepts
both overrides together. The remaining combined-asciinema propagation case is
kept visible as an instrument defect rather than treating the 24-cell result
as exact parity.

**Atomic parity clock (2026-08-06):** Asciinema propagates one parity
assignment reliably, so the TUI now also accepts
`RUNIE_PARITY_CLOCK=UNIX_SECONDS,ELAPSED_TICKS`. The capture
`RUNIE_PARITY_CLOCK=1785990000,38` produces an exact 62×32 full-cell match
against the Grok `Hey` reference: all 1,984 cells equal, with zero glyph or
attribute differences. Individual timestamp and elapsed variables remain
available for focused probes.

**Atomic Runie matrix capture (2026-08-06):** The same deterministic clock was
captured through the matrix harness at `62x32`, `80x24`, `100x30`, and
`120x36`, with complete casts and raw streams emitted for each geometry. The
installed Grok binary did not reach the bounded prompt boundary for a fresh
four-size capture, so only the 62×32 pair is accepted as a strict Grok match;
the other three Runie casts remain artifacts awaiting paired Grok captures.
The YAML `visual-hey` matrix continues to exercise all four geometries.

**Grok matrix capture boundary (2026-08-06):** The private capture script now
uses a 60-second readiness/settled window, shell-quotes the complete command as
one asciinema argument, and bounds post-quit shutdown before killing only its
own tmux session. Grok captures at all four geometries now complete with the
correct `C-d` quit chord. Comparing them to the deterministic Runie casts is
not an exact content oracle because live Grok returns different answer text,
usage, timestamps, and elapsed duration; the matrix is retained as a full-cell
diagnostic rather than a false pass.

**Prompt timestamp edge alignment (2026-08-06):** The user-feed timestamp
projection no longer reserves a fixed eight-cell gutter after the prompt. It
right-aligns the timestamp against the feed width, matching Grok's edge-based
placement and reducing the controlled 62×32 probe from 24 to 20 glyph
differences. The wrap gutter is retained for long prompts while short prompts
right-align at the Grok edge. The locked Grok YAML fixture remains green, and a
fresh frozen 62×32 probe now reports 10 glyph-only differences: the user row is
exact, with only completed-assistant timestamp edge placement and elapsed
capture propagation remaining.

## Review findings

### Phase-marker validation — 2026-08-06

`cast_compare --frames-after=MARKER` now fails explicitly when the requested
marker is absent from either cast, including the cast path and marker text.
Previously that case produced an empty zero-frame comparison, which could hide
an invalid phase selection behind an ordinary mismatch. The full workspace
gate remains green after this diagnostic hardening.

The comparator now has direct binary tests for both successful marker
selection and missing-marker failure, using the checked-in Grok cast. This
keeps the phase-selection contract executable rather than relying only on
manual CLI probing.

The current broad-marker probe (`--frames-after=❯`) over the checked-in full
casts reports an immediate real divergence: Grok/Runie retain 62/121 visible
frames, and the first differing cell is `(2,0)` (blank versus `m`). This is
evidence that the marker must identify a semantic scenario boundary before
indexed frames can be compared one-to-one; it is not a parity pass.

Phase markers may now be occurrence-qualified as `MARKER#N`; the comparator
counts visible marker transitions, not repeated transport frames. Binary tests
cover first-marker selection, numbered selection, and missing-marker failure.

### Fresh 62×32 paired capture — 2026-08-05

The installed Grok binary and `just tui` were captured independently with the
same tmux/asciinema harness and prompt (`Hey`):

- Grok: `/tmp/runie-grok-matrix-62x32.cast`
- Runie: `/tmp/runie-runie-matrix-62x32.cast`
- command: `just cast-compare` (full 62×32 cell grid)
- result: 190 differing cells — 170 glyph differences and 20 attribute-only
  differences

The row diagnostics identify the remaining live mismatch classes precisely:
provider-generated answer text and wrapping, reasoning-row placement, dynamic
usage (`15K` versus `0`), prompt timestamps, and elapsed time. The comparison
does not support claiming pixel parity for this live run. Deterministic exact
parity requires the same response/usage/time inputs, or a reference contract
that explicitly freezes those values; color and blank-cell attributes are now
being compared rather than silently ignored.

The six legacy source-backed Insta snapshots were refreshed after this audit.
Their diffs were the intentional vpad/blank-row projection already covered by
the strict Grok YAML and full-cell checks; no reference cast or attribute
assertion was weakened.

1. Same-run Grok/Runie captures are required for dynamic clock, usage, model
   response, and elapsed fields; stale casts cannot prove pixel parity.
2. Reference frame selection must be phase-specific. `frame_contains` alone
   can select an earlier/later frame with the same activity marker.
3. YAML now supports an explicit zero-based `frame_index`; it takes precedence
   over broad marker matching. Diagnostics include expected/actual complete
   rows and attributes.
4. No visual assertion may be weakened to make the workspace gate green.

5. The committed compact-autoscroll change was rechecked after reverting an
   experimental full-mode spacing adjustment: the YAML fixture-discovery gate
   passes, while the independent `visual_snapshots` suite still reports the
   known full-mode spacing/resize differences. This keeps the two evidence
   classes separate instead of treating a narrow YAML pass as global parity.

6. A full-mode spacing experiment was rejected by the instruments: it reduced
   several legacy snapshot diffs but moved the locked `grok-rich.cast` activity
   frame and broke the narrow turn-summary gutter unit test. The source remains
   unchanged, and the 111-test unit gate plus YAML replay gate are green again.

7. The full `visual_snapshots` run currently fails six strict fixtures
   (`submitted`, `reasoning`, `error`, `tool`, `resize`, and `scroll`). Their
   shared diff is an extra user-entry vertical-padding pair that shifts the
   prompt/footer rows; the content rows remain present. Generated `.snap.new`
   files were discarded, and no reference snapshot was weakened or re-recorded.
   This isolates terminal-height-aware vpad projection as the next fix.

8. The declarative `visual-hey.yaml` scenario is now replayed through the
   Runie renderer at `62x32`, `80x24`, `100x30`, and `120x36`. Each matrix
   case verifies dimensions and the rendered user cursor, ensuring the
   terminal-height path is exercised across the same capture sizes used by
   `scripts/capture-matrix.sh`.

9. The workspace run after execute-card and geometry changes passes all core,
   TUI unit, YAML replay, and cast-backed tests. Six legacy Insta snapshots
   still fail only on the source-backed user-vpad row shift; their generated
   `.snap.new` files were discarded. These references require fresh same-run
   Grok captures before acceptance, so they remain strict and unchanged.

## Status

In progress. Full-cell dumps, row diagnostics, and phase-locked frame
selection are implemented. `visual-grok-feed` is now locked to Grok output
frame 81; its mismatch remains visible as a real parity failure.

### Deterministic clock checkpoint — 2026-08-06

The capture harness now supports an atomic parity clock shared by the prompt
timestamp and elapsed-time renderer. With the frozen Grok `Hey` final frame,
`RUNIE_PARITY_CLOCK=1785990000,38` produces an exact 62x32 full-cell match:
1,984/1,984 cells equal, with zero glyph and zero attribute differences.

The four-size capture matrix is also bounded and reproducible. Fresh live Grok
captures remain useful for discovering behavior, but are not exact pixel
oracles because the installed provider independently changes answer text,
usage, timestamps, and elapsed time. Matrix acceptance therefore requires a
frozen transcript/usage/clock contract, not merely matching terminal size.

### Strict attribute probe — 2026-08-06

Temporarily enabling `reference.exact_attributes` for the locked
`visual-grok-feed` frame failed with 480 cells. The diagnostic showed the
checked-in `grok-rich.cast` carries terminal-default foreground/background
attributes while Runie emits the Opaline Grok RGB tokens (`#141414` base and
`#6c6c6c` muted rail). The symbol oracle still passes. The probe was reverted;
the next valid attribute gate must use a freshly captured full-color Grok cast
from the same scenario and geometry, rather than normalizing or weakening the
existing color comparison.

### Capture readiness correction — 2026-08-06

The bounded capture script previously treated the welcome surface's decorative
`❯` and `Grok 4.5` text as proof that an editable prompt was ready. The probe
now requires the working footer (`Shift+Tab`, `Enter:send`, or the explicit
`Type your message` prompt), preventing a capture from typing into welcome and
being mislabeled as a settled scenario. This is an instrument-only change;
the exact full-color Grok scenario still needs a clean isolated capture.

### Valid Grok transition capture — 2026-08-06

After the readiness correction, a fresh private 80x24 capture successfully
performed Grok's welcome `Enter` transition and recorded a settled `Hey` turn:
the cast contains the working `Shift+Tab` footer and `Worked for` completion
marker. Its raw ANSI stream contains only default/attribute SGR sequences and
no `38;2`/`48;2` RGB sequences, even with `TERM=xterm-256color`,
`COLORTERM=truecolor`, and `FORCE_COLOR=1`. This isolates the remaining
full-color attribute-oracle gap to Grok's emitted terminal stream/capture
environment; Runie's Opaline token emission is independently covered by its
theme tests.
Full-cell attribute audit (2026-08-06): a temporary strict enablement of
`exact_attributes` for the checked-in full-screen feed and waiting references
was correctly rejected by the full e2e runner: the feed reports 480 differing
cells, primarily Grok terminal-default blank/background cells versus Runie's
theme-projected truecolor cells. The focused gutter test does not invoke the
reference oracle. The flags remain disabled until a clean same-theme capture
pair is available; this is an open attribute-parity gap, not a pass.

Fresh capture validation (2026-08-06): a new 80x24 `grok Hey` recording made
through `just tmux-cast` emitted only default/bold/dim SGR sequences (`0`,
`1`, `2`, `22`) and no truecolor or background-color SGR. This confirms the
attribute mismatch is reproducible in the current capture environment, so
strict cell-attribute comparison remains disabled until Runie and Grok are
captured under the same terminal color mode.

ANSI comparator hardening (2026-08-06): `scripts/compare-ansi-frames.py` now
retains inverse state and compares ANSI-16, indexed-256, and RGB foreground
and background colors, including their reset forms. The comparator's previous
parser silently discarded those SGR variants; a full-dump comparison can now
report style-only differences instead of treating them as equal. The parser
was syntax-checked and self-compared against the fresh 80x24 capture.

Fresh final-frame selection audit (2026-08-06): the new 80×24 pair in
`/tmp/runie-parity-current/` was dumped cell-by-cell after the indexed
mismatch. Both grids are 80×24, but Grok's selected final frame has a blank
row 1 while Runie's has the header (` main … 15K / 500K`). The reported first
cell `9`/`5` mismatch therefore reflects capture termination/frame selection,
not a header value mismatch. The strict instrument correctly exposes this
phase error; no renderer change or oracle weakening was made.
# Latest full-frame comparator evidence (2026-08-06)

The strict indexed cast comparator was run against the checked-in full casts
after the `❯` prompt marker:

```text
Grok:  62 deduplicated frames
Runie: 121 deduplicated frames
first difference: frame 1, cell (2,0): ` ` vs `m`
```

The comparison includes glyph, foreground/background color, bold, italic,
underline, and inverse attributes for every compared cell. The unequal frame
counts and first-cell diagnostic are retained as a failing parity signal; no
frame filtering or attribute downgrade is allowed to turn this into a pass.
## Capture manifest provenance correction (2026-08-06)

`tmux-asciinema-capture.sh` now records the effective `TERM` and `COLORTERM`
assignments passed to the captured command, including validated caller
overrides, instead of inheriting the parent shell's values. This prevents
capture metadata from overstating color capability and makes later
`exact_attributes` decisions auditable.
## Cell-width parity (2026-08-06)

Cast and YAML full-screen cells now carry an explicit width classification:
wide glyph lead cells are width 2, continuation cells width 0, and ordinary
cells width 1. The VT parser uses its wide/continuation flags; Ratatui frames
derive the same classification from Unicode display width and neighboring
cells. Width is part of cell equality and dump diagnostics.
## Classified attribute diagnostics (2026-08-06)

`cast_compare --dump` now reports attribute mismatches by width, foreground /
background colors, terminal style flags, and any remaining modeled cell
attributes. This keeps the strict equality decision unchanged while making
the next Grok capture actionable: a mismatch can be assigned to terminal
capability, glyph geometry, styling, or an unmodeled attribute without
inspecting raw JSON manually.
## Ordered frame intersection diagnostic (2026-08-06)

`cast_compare --frames` now reports the ordered common-frame count and the
unmatched-frame counts in addition to its unchanged strict ordinal result.
This identifies whether two captures share the same visible states at a
different cadence or contain genuinely missing states. It is diagnostic only:
frame-count equality and exact cell equality are still required for success.
## Alternate-screen phase normalization (2026-08-06)

Cast replay now discards output recorded before an explicit alternate-screen
entry, preventing shell startup rows from becoming the first application
frame. This is phase normalization only; output emitted after entry remains
fully compared. The checked-in `runie-full.cast` additionally contains stale
`[renderer]` diagnostic lines absent from the current source, so it remains a
diagnostic artifact and must be replaced by a fresh capture before any cast
zero-diff claim.
## Fresh capture attempt (2026-08-06)

An isolated `just tmux-cast 80 24 ... grok Hey q` attempt produced only the
asciicast header before the installed Grok process failed to reach a ready
prompt and the bounded capture was interrupted. No frame from that attempt is
used as parity evidence. The replay normalization change is independently
validated by the existing casts and tests; a new authenticated/ready capture
is still required for the final live zero-diff audit.

## Hey semantic-token pass (2026-08-06)

Using the valid truecolor pair `/tmp/grok-fresh-rgb2-20260806.cast` and
`/tmp/runie-frozen-rgb12-20260806.cast`, strict comparison reached 129
different cells, with zero glyph differences. The reduction came from
tokenized Grok panel backgrounds, prompt-border `#505058`, footer key/body
roles, the `14K` header meter, and the assistant body `#c8c8c8`. A later
capture that showed the idle placeholder and doctor hint was rejected as an
invalid scenario artifact. Live parity remains open: the remaining valid
differences are concentrated in header path styling, user/assistant timestamp
roles, activity summary prefixes, and prompt caption styling.

## Settled tmux frame artifact (2026-08-06)

The capture script now writes `<cast>.settled.ansi` immediately after the
bounded `Worked for` plus ready-footer probe and before sending the quit key.
This is the authoritative settled-screen artifact for whole-screen ANSI
comparisons; the asciinema cast remains the animation/replay artifact. The
artifact is also recorded in the adjacent metadata manifest. A smoke capture
produced all 24 rows and the expected Hey feed, removing the teardown race
from the capture workflow.

The YAML dump oracle now accepts `reference.format: ansi` for these settled
artifacts. It wraps the ANSI screen into a one-frame VT replay using the YAML
frame geometry, so `exact_screen`, `exact_attributes`, and `require_truecolor`
remain available without recompiling when the captured artifact changes.
