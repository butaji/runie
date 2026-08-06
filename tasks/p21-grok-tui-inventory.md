# p21 — Grok TUI parity inventory

Status: active. Audited `/Users/admin/Code/agents/grok-build`, especially
`crates/codegen/xai-grok-pager` and `xai-grok-pager-render`.

**Construction audit (2026-08-05):** Grok composes the agent view from a
two-column outer horizontal pad, a scrollback `HorizontalLayout` with a
one-cell accent rail, two-cell left block padding, flexible content, and a
one-cell right pad. Entries are measured before painting; display mode and
vertical padding affect height, and timestamps are overlaid on the first
content row rather than appended to message text. Runie now names and tests
these source-derived layout tokens and uses them in the chat layout.

## Theme-token parity (2026-08-05)

Opaline semantic style projections now accept the actor-selected `ThemeKind`;
status styling no longer resolves `GrokNight` unconditionally. A GrokDay
regression verifies that foreground, background, and accent cells come from
the day palette's tokens. The feed projection now receives the selected theme
through its pure render path; broader Grok state/variant coverage remains
before this inventory can be marked complete.

The selected theme is now also propagated to the feed actor on
`ThemeChanged`; `visual-theme-day.yaml` exercises the complete event-driven
path without recompilation. Feed style unit coverage verifies assistant and
activity cells resolve day-theme tokens.

**Command-palette boundary (2026-08-05):** Cast inventory exposed Grok's
`Echo Command Query Title` modal. Runie now has an actor-owned
`command_palette_open` MVU state, `Ctrl+P`/`?` actions, themed modal chrome,
and Esc close behavior. Query text, filtering, and selection movement are also
actor-owned, with a pure `CommandPaletteWidget` shared by the live binary and
YAML visual runner. `visual-command-palette.yaml` covers the filtered modal;
full Grok command registry/action execution remains open.

`New Session` now follows the event boundary: palette activation is reduced
by the UI actor, and the binary/YAML paths publish the existing
`AgentEvent::Reset`. `visual-command-palette-activate.yaml` covers that action;
other Grok commands remain unimplemented until their corresponding core/TUI
event contracts are added.

The pure palette registry now mirrors the first 15 stable Grok labels
(`New Session`, worktree/session actions, context actions, model switch, and
shortcuts), with filtering exercised by YAML. Action dispatch remains an
explicit follow-up because it must publish core/UI events rather than mutate
loop or session state from the modal.

## Findings

- Grok has 402 pager source files and a typed `RenderBlock` projection:
  `UserPrompt`, `AgentMessage`, `ToolCall`, `Thinking`, `System`,
  `SessionEvent`, `BgTask`, `Subagent`, `Workflow`, `Btw`, `ContextInfo`,
  `CreditLimit`, and `Stub`.
- Tool rendering is specialized for execute, read, edit, list-dir, search,
  web-search, web-fetch, memory-search, lifecycle, generic use, and other
  tools. Blocks own output, grouping, selection, display mode, accent/bullet,
  and running/finished behavior.
- `BlockContext` carries display mode (`Collapsed`, `Truncated`, `Expanded`),
  wrapping mode, selection, width, raw mode, appearance, and cwd to pure
  renderers. This is the correct reference model for replacing Runie's flat
  `LineKind` projection.
- Typed turn activity includes Thinking, Responding, ToolRunning,
  AutoCompacting, Retrying, and WaitingReason variants for model, subagent,
  task output, tasks complete, and sleep. Runie's generic waiting status is
  insufficient for parity.
- `xai-grok-pager-render` provides GrokNight, GrokDay, TokyoNight,
  RosePineMoon, OscuraMidnight, and Auto themes, with truecolor definitions and
  terminal-capability quantization. Appearance configuration covers animation,
  block backgrounds, layout, prompt, scroll, scrollbar, tool bullets, edit
  headers, todos, mermaid, and terminal behavior.
- Animation is demand-driven: the event loop schedules ticks only while a
  view reports demand. Shared ticks drive spinner frames, animated accents,
  running task bullets, thinking, workflows, and overlays. The source also has
  explicit tmux/color capability probes.
- PTY tests cover waiting labels, spinner resumption, verb-group fold/expand,
  streaming/thinking folds, prompt geometry, resize, scroll, queue/interjection
  lifecycles, cancellation, overlays, welcome variants, and terminal color
  paths. This is the authoritative feature/state matrix, not isolated row
  snapshots.

## Runie gap map and order

1. Replace ad-hoc visual flags with an actor-owned typed presentation model
   carrying block/display/waiting state.
2. Add typed waiting reasons and animation demand/frame events to the event bus.
3. Add theme/config and terminal capability projections with canonical truecolor
   comparison.
4. Model tool groups/member visibility as foldable blocks with exact gap
   geometry.
5. Add YAML event/state scenarios and full PTY captures for every state and
   animation variant; keep TestBackend as the fast deterministic path.

Implementation progress: `runie-core::types::WaitingReason` now mirrors
Grok's model/subagent/task-output/tasks-complete/sleep variants, and
`AgentEvent::Waiting` is projected by the TUI status actor. The event kind is
covered by the shared DSL and YAML event oracle. `just ci` passes after this
change.

The YAML runner now accepts `waiting: { waiting: model }` and the other typed
variants as declarative events. It omits control events from the provider
stream, then appends them to the recorded bus sequence before replay, so
scenario edits do not require recompiling the test harness.

Animation scheduling now follows the same demand boundary: the renderer owns
one restartable timer and arms it only while the status actor projects an
animated state. Idle, ready, aborted, and error states do not schedule timer
wakes. The demand predicate is covered by a unit test.

The event contract now also includes Grok's six named theme variants
(`GrokNight`, `GrokDay`, `TokyoNight`, `RosePineMoon`, `OscuraMidnight`, and
`Auto`). YAML can declare `theme: grok_day`; the status actor owns the active
projection. Full palette propagation and terminal color quantization remain
open.

Feed progress: completed directory/file tool cards now retain Grok's semantic
card headers and cardinality (`List … (entries)`, `Read … (lines)`) instead of
the generic `→ ✓` suffix. Generic tools retain their status marker. This is
covered by renderer tests and the visual tool replay.

Working-view baseline progress: the literal `Hey` → short assistant response
scenario is pinned in `visual-hey.yaml`. The full-mode layout now uses Grok's
transcript rail, assistant body alignment, and quiet branch/path
header; the fixture explicitly rejects the prior `user>`/`assistant>` feed
labels.

The same replay now verifies the turn lifecycle: the provisional `◆
Thinking…` row folds to `❙  Thought` when a non-empty assistant response
commits, while empty tool/waiting captures retain their reference chrome.

Live submitted prompts now carry a wall-clock timestamp through the user
message event and render Grok's short `h:mm AM/PM` prompt-row format. Replay
fixtures with synthetic timestamps remain deterministic; the formatter has
direct unit coverage. YAML also accepts `prompt_timestamp`, and the `Hey`
fixture pins the complete timestamped user row without recompilation.

Color parity progress: the live frame now paints the GrokNight truecolor base
(`bg #141414`, `fg #e1e1e1`) before widgets render, and feed semantics use the
Grok accent/muted/success/error palette instead of terminal-default colors.
Color-sensitive visual snapshots were refreshed and the palette projection is
unit-tested; full theme switching and terminal quantization remain open.

Live `just tui` audit also found and fixed a separate input-path issue: the
binary discarded Shift+Char events, so typing `Hey` produced `y`. The direct
terminal path now forwards shifted characters to the prompt actor; tmux
replay shows `❯ Hey` and the timestamped row before submission.

The reference feed component matrix is now recorded explicitly:

| Family | Grok variants/states | Runie coverage |
|---|---|---|
| User/assistant | prompt, markdown, tables, code, links, streaming | YAML + snapshots |
| Thinking | running spinner, collapsed `Thought`, expanded reasoning | YAML + renderer tests |
| Tool cards | execute, read, edit, list-dir, search, web, lifecycle, generic | partial; typed core events, line projection |
| Tool display | collapsed, truncated, expanded, running/finished/error | collapsed/expanded activity fixtures; block model open |
| Verb groups | Read, Listed, Searched, Ran, subagent counts; running/past verbs | activity summary projection |
| Background work | subagent, workflow, task output, waiting reasons | waiting reasons; block rendering open |
| Chrome | header meter, status telemetry, prompt/footer, doctor hint | strict feed/waiting frames |
| Effects | braille/dot spinners, animated accents, overlays, terminal capability paths | demand-driven spinner; broader effects open |

The strict fixed-grid oracle now covers both the Grok feed and waiting frames;
component/state fixtures remain the next expansion target for the partial tool
and effect families above.

Per-tool display state is now event-driven: `ToolDisplayModeChanged` carries a
tool-call ID and `Collapsed`/`Truncated`/`Expanded` mode. Tool lines retain
their originating call ID. Collapsed cards keep their semantic header while
hiding output/result rows; truncated cards keep the first output/result row.
The mixed activity and truncated activity YAML fixtures exercise these rules
independently, including a complete fixed-grid replay assertion.

The immediate task remains broader tool-card parity (specialized execute/edit/
search/web/background cards), followed by theme palette propagation and the
remaining YAML state/effect matrix. Strict feed, waiting, collapsed, mixed, and
truncated fixed-grid replays are green.

Source audit: Grok's `UserMessageBlock` is the only standard conversation block
that enables prompt vpad; system, session-event, thinking, background, and all
specialized tool blocks explicitly disable it. The renderer also suppresses
vpad when the available content area is smaller than three rows. Runie's
`Line::has_vpad` metadata and terminal-height-aware renderer now preserve that
distinction, with unit coverage for full, compact, and undersized panes.

Execute-card progress: shell/exec tool headers now use Grok's semantic `Run`
label with the command argument, and `visual-execute.yaml` exercises the path
through the real event replay and rendered screen assertions.

Integration discovery progress: `search_tools` now uses Grok's `Search Tools`
header and `(N results)` completion cardinality, covered by
`visual-search-tools.yaml`.

Subagent-card progress: subagent/agent/task tools now render explicit
`Subagent started:` and `Subagent completed:` lifecycle text, covered by
`visual-subagent-card.yaml`. Duration text remains intentionally deferred to
the actor-owned tool timing projection.

Architecture audit note: `PromptActor` and `UiActor` own mailbox/watch state,
but `Scrollback` and `StatusBar` are still shared behind `parking_lot::Mutex`
and are mutated by `EventRenderer` and the render loop. This is a remaining
SSOT/MVU migration target, not evidence of completed actor parity.

The first migration seam is now in place for status: `StatusMsg` is the
explicit reducer input, `StatusBar::apply` is the pure transition boundary,
and the reducer has regression coverage. The next step is moving that reducer
behind a mailbox/watch actor and replacing the compatibility mutex with its
read-only snapshot.

`StatusActor` now provides that mailbox/watch boundary with owned task
lifetime, acknowledged `StatusMsg` application, and a snapshot regression
test. Existing rendering still uses the compatibility path; migrating
`EventRenderer` and the live/YAML renderers to this handle is the next step.
