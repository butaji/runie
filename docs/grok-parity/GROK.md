# Runie vs Grok Build TUI — Gap Analysis

This document compares xAI's Grok Build TUI against Runie's current implementation and identifies concrete improvements to borrow.

## Legend

| Status | Meaning |
|--------|---------|
| ✅ Implemented | Feature exists and is functional |
| 🟡 Partial | Skeleton or partial implementation |
| ❌ Missing | Not implemented |
| 🔵 N/A | Not applicable to current scope |

## Gap Analysis Table

| # | Area | Feature | Grok Build | Runie Current | Gap | Priority |
|---|------|---------|------------|---------------|-----|----------|
| 1 | **Terminal** | Capability detection (truecolor, mouse, clipboard, focus, multiplexer, brand) | Probes at startup; adapts behavior | ✅ Implemented (`crates/runie-term/src/terminal/caps.rs`) | Uses env heuristics only; does not do OSC probes or synchronized updates | **P1** |
| 2 | **Input** | Mouse support (SGR mode, click, scroll, drag) | Enables `?1000/1002/1003/1015/1006`; scroll feed, click blocks, drag select | 🟡 Detection + `EnableMouseCapture` exists, but no hit-testing or routing | Wire crossterm mouse events to feed scroll, block selection, and prompt focus | **P0** |
| 3 | **Chrome** | Dynamic contextual footer hints | Hints change by state (idle/running/scrollback/modal) | 🟡 Already state-aware (vim nav, @refs, active turn, input, empty) | Add hints for scrollback focus, Team mode, modal open, and mouse-enabled actions | **P0** |
| 4 | **Chrome** | Execution mode in input title | `grok-build · plan/subagents/always-approve` | 🟡 Input title shows `provider/model` only | Add mode suffix for existing/planned Runie modes (`Solo`/`Team`/`read-only`) | **P0** |
| 5 | **Chrome** | Status bar metadata | Git branch, cwd, token budget, activity spinner | 🟡 Git info, token usage piece, spinner, queue count shown | Keep chess-piece usage indicator; add worktree label + orchestrator mode indicator only | **P1** |
| 6 | **Navigation** | Vim mode | Opt-in `hjkl`, `gg/G`, `/`, `y`, `H/L` turns | ✅ Implemented (tests in `vim_mode.rs`, `vim_element_jump.rs`) | Minor: add `y` copy block, `o` open link | **P1** |
| 7 | **Clipboard** | OSC 52 + platform fallback | Copies blocks via OSC 52; falls back to OS binaries | 🟡 OSC 52 implemented in `runie-term/src/terminal/clipboard.rs`; effects wired for `/copy` and `/copy-last` | Add block-level `y` copy; add platform fallback when `caps.clipboard` is false | **P1** |
| 8 | **Commands** | Fuzzy command palette with ranking | `Ctrl+P`, fuzzy + recency + usage ranking | 🟡 Palette exists with filtering and selection; no ranking | Add recency/usage scoring | **P1** |
| 9 | **Feed** | Consistent block iconography | `❯` user, `◆` tool, `∘` assistant, `✓`/`✗` status | 🟡 `Element` system has `PostKind` (UserInput, AgentResponse, Thinking, ToolRunning, etc.) | Standardize glyphs/accent colors and add inline duration/byte count/status | **P1** |
| 10 | **Feed** | Collapsible thinking/tool blocks | `h`/`l` or `e` fold/unfold | ✅ Implemented (collapse/expand tests, `ThoughtSummary`/`ToolSummary`) | Add keyboard hints for folding | **P2** |
| 11 | **Agents** | Subagent sidebar / panel | Dedicated panel with labels, dot-grid activity, `Ctrl+1..9` | ❌ Planned for R4 but not implemented | Visible in Team mode; rows: `Ctrl+0` Orchestrator + `Ctrl+1..9` subagents; click/hotkey switches feed focus | **P1** |
| 12 | **Agents** | Agent profiles from TOML | JSON/TOML-defined agents with tool allow/deny lists | ✅ Implemented (`agent_profiles.rs`, `~/.runie/agents/*.toml`) | None significant | **P2** |
| 13 | **Context** | `@file` references | `@src/main.rs:10-50` attaches files with line ranges | 🟡 `@file` picker exists (`file_refs.rs`) | Add `start-end` line-range parsing and render attached context block | **P2** |
| 14 | **Input** | Multiline input / Shift+Enter | `Shift+Enter` inserts newline | ✅ Implemented (existing tests) | — | **P2** |
| 15 | **Onboarding** | Welcome / launcher screen | ASCII art, New worktree / Resume / Quit | ❌ No welcome screen | Add launch view with recent sessions | **P2** |
| 16 | **Permissions** | Tool confirmation UI | Diff/Write/Bash modal + always-approve mode | ✅ Implemented (`confirmation.rs` with Diff/Write/Bash) | Add `/always-approve` command and mode indicator | **P2** |
| 17 | **Theme** | Color quantization to ANSI 256/16 | Quantizes RGB based on detected depth; hides truecolor-only themes | 🟡 `quantize.rs` exists but not wired to runtime detection | Quantize theme at startup based on `caps.truecolor`; skip quantization when truecolor is supported | **P2** |
| 18 | **Theme** | OS-adaptive / auto theme | `theme = "auto"` with dark/light variants | ❌ Not implemented | Add auto theme + dark/light pair | **P3** |
| 19 | **Animation** | Configurable FPS, spinners, wave effects | `[animation] fps = 30`, wave rows, spinner config | 🟡 Braille spinner frames exist; no config | Skip for now; keep hard-coded spinner | **P3** |
| 20 | **Focus** | Focus tracking for notifications/pause | `CSI ? 1004 h`, pause animations when unfocused | 🟡 Enabled in `terminal_setup.rs`; events consumed but not acted on | Use focus events to pause animations / notify | **P3** |
| 21 | **Settings** | Settings dialog | Unified settings with categories | ✅ Implemented (Models, Appearance, Behavior, Safety) | Could add Mouse, Notifications, Animation categories | **P2** |
| 22 | **Git** | Worktree awareness | Auto-detects worktree, `Ctrl+W` new worktree | 🟡 Detects worktree git info for status bar | No worktree launch UX | **P3** |
| 23 | **Diff** | Inline diff viewer | Inline `+`/`-` lines with gutter | 🟡 `diff.rs` exists | Verify it covers edit previews from `confirmation.rs` | **P2** |
| 24 | **Notifications** | Notifications on completion when unfocused | `[ui.notifications] condition = "focused"` | ❌ Not implemented | Add optional BEL/notification on turn complete | **P3** |
| 25 | **MCP** | MCP server status indicator | `⛔ 6 MCP servers unavailable` | 🟡 `mcp.rs` stub exists | Add status bar / modal indicator | **P3** |

## Summary by Priority

### P0 — Do First
1. Real mouse handling (hit-testing + routing)
2. State-aware footer hints
3. Mode suffix in input title

### P1 — High Value
4. Richer status bar
5. Block-level copy (`y`) and OSC 52 wiring
6. Palette ranking
7. Standardized block glyphs
8. Terminal capability OSC probes + synchronized updates
9. R4 subagent sidebar

### P2 — Polish
10. `@file` line ranges
11. Welcome screen
12. Theme quantization wiring
13. Inline diff polish

### P3 — Future
15. OS-adaptive theme
16. Focus-based notifications
17. Worktree launch UX
18. MCP status indicator

## Key Insight

Runie already has the *core machinery* for many of Grok's features: terminal capability detection, `Element` types, agent profiles, command palette, confirmation flow, theme system, vim mode, and file refs. The biggest gaps are **integration and polish** — wiring mouse/focus/clipboard events, adding synchronized updates, and making the chrome (status bar, hints, input title) adapt to state. Those are lower-risk wins than building entirely new subsystems.
