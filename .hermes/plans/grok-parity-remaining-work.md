# Grok Look-and-Feel Parity — Plan & Status

## Where we are
12 review cycles + 14 perf/parity commits landed since the last
baseline. `grok_parity_test` reports **47/47 pass** (visual +
behavioral baseline). Three live scenarios replay against real
Grok reference dumps.

## Current 03_chat_simple diff (3 cosmetic lines)
Mine matches ref on every meaningful line:
- `     ❯ hello` ✓
- `     ◆ Thought for 1.2s` ✓
- `     Hello. How can I help with the runie project?` ✓
- `     Turn completed in 3.6s.` ✓

Only deltas are **trailing whitespace** in the top-bar and
hotkey bar (one row of pad chars at the right edge). The diff tool
flags every padded row as different even when no glyph is wrong.

## Speeds (already done)
- `bin/runie-rerun` skip-rebuild mtime check: 1.24s → **0ms** on
  no-change iteration.
- `crates/runie-tui/src/bin/scenario_fasthot.rs`: single-process
  hot loop. 10ms poll + 1-3ms render = sub-15ms save-to-render.
- `bin/runie-parity`: one-shot `scenario_replay --diff` per dump.
- Reply provider simulation:
  - `reply_to_scenario` (provider → JSONL)
  - `scenario_replay` (one-shot render+diff)
  - `scenario_fasthot` (long-running hot loop)

## Remaining work (9 items, sorted by ROI)

### 1. `parity_03` — finish 03_chat_simple (in_progress)
3 trailing-space deltas in top bar (`│ 20K / 512K │        │`)
and hotkey row. Need to either:
  (a) trim trailing spaces before writing the buffer, or
  (b) align widths so `set_string` ends exactly at column N.
  Option (a) is local to `top_bar/render.rs` and `hotkeys.rs`
  (one `trim_end` per span). Estimated 2-3 deltas closed.

### 2. `parity_04` — 04_chat_complete (12 deltas)
The current 04 scenario lacks:
  - a tool-call event (the Grok dump shows `✓ bash` rows with
    `→ ok` results)
  - a multi-line assistant reply (the dump has 6-line response
    with bullet list)
  - a `◦ Waiting` placeholder shown when the agent is idle
  - a final "question" prompt (the █-block cursor with `Would
    you like me to …?`)
Requires:
  - `ToolExecutionStart` + `ToolExecutionEnd` events in scenario
  - new `Wait` or `Idle` item in the message list (currently
    absent — the renderer doesn't have a state for it)
  - streaming cursor block on the last assistant message
  (matching the `█` and `██` markers in the ref dump)

### 3. `parity_05` — 05_input_typing (7 deltas)
Currently shows `List .` (text only) instead of:
  - `List .▊` streaming with the `▊` cursor block
  - `⠋ Thinking…` indicator with elapsed seconds (`0s`)
  - `Ctrl+c:cancel` + `Ctrl+Enter:interject` hotkey row
Requires:
  - streaming cursor rendering inside `render_assistant_msg`
    when `agent_running` (already partially present)
  - an `AgentState::Thinking` overlay row in the message list
  - hotkey row layout changes when agent is running vs idle

### 4-9. Add scenarios for the other 49 dumps
Picks from `ui/dumps/grok/` (sorted by feature surface):
  - `01_welcome` — home screen (10 deltas, 0/0 currently)
  - `02_session_list` — sessions picker modal
  - `05_slash_menu` — `/` commands palette
  - `06_command_palette` — Ctrl+K palette
  - `07_plan_modal` — plan mode toggle
  - `08_extensions_modal` — extensions overlay
  - `09_theme_switch` — theme picker
  - `11_model_picker` — model selector
  - `12_settings` — settings screen
  - `20_fork_confirmation` — fork action confirm
  - `25_context_confirmation` — context compaction confirm

Each new dump requires a scenario JSONL with the right
**event sequence** to drive the TUI into the same state. The
scenario_fasthot loop is fast enough that authoring + verifying
each one is ~5-10 minutes.

## Architectural work needed (not dump-specific)
- `AgentEvent::AgentState` (or new `AgentPhase` enum) to drive
  the `Waiting` / `Thinking…` overlay rows that several dumps
  show
- A `Question` item type in `MessageItem` for the `█`-block
  cursor that shows while the agent awaits user input
- Streaming cursor block in `render_assistant_msg` (currently
  renders `█` at the right edge but only when `agent_running`)

## Speed wins still on the table
- **Scenario authoring hot path**: `bin/runie-rerun` already
  skips rebuild on no-change, but for new scenarios we still
  pay 1.4s of cargo build. Adding a `cargo check`-based
  hot-reload would drop that to ~0.3s for pure-Rust edits.
- **`scenario_fasthot` to multiple files**: today it watches
  one file. Watching a directory and re-running on any
  `*.jsonl` change would let us sweep 49 dumps without
  restarting the binary.
- **`reply_to_scenario` to stdout diff**: pipe its output
  through `scenario_replay --diff` so the author sees the
  parity delta inline while writing the provider fixture.

## Next decision
Drive 03 trailing-space fix + add a `AgentState::Waiting` item +
3 new scenarios (01_welcome, 02_session_list, 05_slash_menu) in
that order, each as a separate commit. Then re-run the full
52-dump parity test once a scenario exists for each grok file.
