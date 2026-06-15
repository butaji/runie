# Inline Tool Call Rendering (Grok Parity GAP #4)

**Status**: done
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

## Description

Tool calls in the scrollback are stored in state but rendered with
basic, static text. The Grok Build TUI shows them as a rich
single-line block with:

```
⠴ Run List `.` 1.8s                                         5.7s ⇣21.2k [✗]
```

Components:
- **Spinner frame** — `⠦⠴⠋⠼⠴⠦⠂⠇` animated (~8fps) while running
- **Label** — `Run <name> '<args>'` (vs current `Running <name>...`)
- **Elapsed duration** — `1.8s` updates while running
- **Total duration** — `5.7s` shown on completion
- **Bytes transferred** — `⇣21.2k` for downloads
- **Status icon** — `✓` / `✗` / `[✗]` for success/failure
- **State-aware format** — running vs done vs error

## Acceptance Criteria

- [ ] `render_tool_running` shows: animated spinner + `Run <name> '<args>'` + elapsed secs
- [ ] `render_tool_done` shows: `✓ Run <name> '<args>'` + total duration + bytes (if any) + status
- [ ] `render_tool_summary` keeps a single-line `Run <name> '<args>' <duration> [+]` form
- [ ] Spinner animates at 8fps (10 frames)
- [ ] Bytes formatting: `⇣1.2k`, `⇣3.4M`, `⇣567` (humanize)
- [ ] Error state shows `[✗]` icon at end
- [ ] Empty args handled: `Run <name>` (no quotes, no trailing space)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` succeeds

## Tests

### Layer 1 — State/Logic (pure functions)
- [ ] `format_tool_label_with_args` — produces `Run <name> '<args>'`
- [ ] `format_tool_label_no_args` — produces `Run <name>` (no quotes)
- [ ] `format_tool_label_truncates_long_args` — args >40 chars truncated with `…`
- [ ] `format_bytes_small` — 0..999 → `567`
- [ ] `format_bytes_kb` — 1000..999_999 → `1.2k`
- [ ] `format_bytes_mb` — 1_000_000+ → `3.4M`
- [ ] `spinner_frame_tick_zero` — returns frame 0
- [ ] `spinner_frame_tick_advances` — consecutive ticks cycle frames
- [ ] `spinner_frame_wraps` — tick beyond array length wraps to 0
- [ ] `format_tool_duration_runs` — `< 60s` → `12.3s`
- [ ] `format_tool_duration_minutes` — `>= 60s` → `1m5s`

### Layer 2 — Event Handling
- [ ] `agent_tool_start_renders_running` — `AgentToolStart` updates `ToolRunning.started`
- [ ] `agent_tool_end_renders_done` — `AgentToolEnd` with success renders `✓`
- [ ] `agent_tool_end_error_renders_failure` — error state renders `✗`

### Layer 3 — Rendering (TestBackend)
- [ ] `render_tool_running_shows_spinner` — output line contains a spinner char
- [ ] `render_tool_done_shows_checkmark` — output line contains `✓`
- [ ] `render_tool_done_shows_bytes` — output line contains `⇣` and formatted bytes
- [ ] `render_tool_error_shows_x` — output line contains `✗`
- [ ] `render_tool_summary_is_one_line` — collapsed form is exactly one line

### Layer 4 — Smoke (tmux)
- [ ] Tool running state shows animated spinner
- [ ] Tool completion shows `✓` with duration

## Notes

**Related files:**
- `crates/runie-tui/src/message/mod.rs` — `render_tool_running`, `render_tool_done`, `render_tool_summary`
- `crates/runie-tui/src/ui.rs:400-413` — dispatch from `Element` to render functions
- `crates/runie-core/src/snapshot.rs` — `ToolRunning`, `ToolDone`, `ToolSummary` data types

**Grok reference** (removed; original noted inline tool block with spinner,
duration, bytes, and status icon):
> ⠴ Run List \`.\` 1.8s                                         5.7s ⇣21.2k [✗]

**Existing tests:**
- `crates/runie-tui/src/tests/render/toggle_expand.rs` — `ctrl_shift_e_collapses_tool_post_in_feed`
- `crates/runie-tui/src/tests/color_restraint.rs` — `tool_header_is_low_contrast_not_success`

**Design constraints:**
- Spinner frames must match the existing 10-frame Braille array used elsewhere in the TUI
- `⇣` (U+21E3) and `✓` (U+2713) / `✗` (U+2717) are already defined in `glyphs.rs`
- Tool call must remain a single terminal line (no wrap) for visual consistency

## Out of scope

- MCP server status badges (separate task)
- Per-tool sub-panels (preview, detailed output)
- Cross-tool timeline (gantt-style view)
- Subagent inline tracking
- Animated progress bars
