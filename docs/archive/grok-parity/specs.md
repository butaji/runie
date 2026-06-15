# Grok UI Element Specification - Runie Implementation Mapping

**Document Version:** 1.0
**Generated:** 2026-06-03
**Grok Reference:** GROK.md (v0.2.15 Beta)
**Runie Source:** crates/runie-tui/src/

---

## Overview

This document maps each Grok Build TUI element to its exact Runie implementation. For each element, we document the Grok reference format, position, states, Runie implementation file/function, current status, known issues, and test coverage.

---

## 1. Header Bar (Status Line)

### Grok Reference
```
   main ~/Code/GitHub/runie                                    │ 21K / 512K │
```

### Position
- Line 1 (top of viewport)
- Column 0
- Width: full terminal width

### States
| State | Display |
|-------|---------|
| **Idle** | Branch + path + token count, no spinner |
| **Streaming** | Spinner at start (`⠦⠴⠋⠼`), token count updates |
| **HomeScreen** | Hidden (no token meter) |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/top_bar/render.rs`
- **Function:** `render_top_bar`
- **ViewModel:** `crates/runie-tui/src/components/top_bar/mod.rs` → `TopBarViewModel`
- **Helpers:** `crates/runie-tui/src/components/top_bar/helpers.rs` → `build_left_spans`
- **Gauge:** `crates/runie-tui/src/components/top_bar/gauge.rs` → `format_token_count`, `format_context_window`

### Status: ✅ Matching
- Branch indicator: `GIT_BRANCH_SYMBOL` from `style/selection.rs`
- Path shortening via `shorten_path`
- Token meter: `│ {} / {} │` format
- Spinner: `braille_frame` with `spinner_frame()` from `glyphs.rs`

### Known Issues
- None

### Test Coverage
- `crates/runie-tui/src/components/top_bar/tests.rs`
- `crates/runie-tui/src/components/top_bar/helpers_test.rs`

---

## 2. Welcome Screen

### Grok Reference
```
                      New worktree                   ctrl-w
                      ─────────────────────────────────────
                      Resume session                 ctrl-s
                      ─────────────────────────────────────
                      Quit                           ctrl-q

  Tip: Press Ctrl-W to start a parallel task in its own worktree.
```

### Position
- Centered in viewport
- Width: `MENU_WIDTH` from `style/layout.rs`
- Height: `MENU_HEIGHT` from `style/layout.rs`

### States
| State | Display |
|-------|---------|
| **Idle** | 3 menu items with dividers |
| **Session List** | `show_sessions=true` shows session picker |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/home_screen/mod.rs`
- **Function:** `render_home_screen`
- **Constants:** `HOME_MENU_ITEMS` static array
- **Menu Items:** Lines 15-19

### Status: ✅ Matching
- Menu items match exactly: `("New worktree", "Start a parallel task", "ctrl-w")`, etc.
- Dividers via `draw_divider` using `BOX_H` character
- Tip banner rendered at bottom
- Version badge: NOT shown (Runie omits version in home screen)

### Known Issues
- Missing version badge at bottom-right

### Test Coverage
- `crates/runie-tui/src/components/home_screen/mod_test.rs`
- `crates/runie-tui/src/components/home_screen/render_test.rs`

---

## 3. Scrollback (Main Conversation Area)

### Grok Reference
```
     ❯ grok                                                         11:28 PM

     ◆ Thought for 0.9s
     ◆ Read ~/.grok/docs/user-guide/README.md
     ◆ Read Cargo.toml
     ◆ List .
     ◆ Search "AGENTS\\.md|Claude\\.md|Agent\\.md|Agents\\.md" (no files)

     ⠴ Run List `.` 2.9s                                         11s ⇣22.2k [✗]
```

### Position
- Full viewport between header and input bar
- Scrollable content area

### Block Types
| Symbol | Type | Runie Glyph |
|--------|-------|-------------|
| `❯` | User Prompt | `CHEVRON` (`\u{276F}`) |
| `◆` | Tool Call | `DIAMOND` (`\u{25C6}`) |
| `∘` | Assistant Response | `ASSISTANT_BULLET` (`\u{2218}`) |
| `⠦⠴⠋⠼` | Animations | `SPINNER_FRAMES` array |
| `✓` / `✗` | Status | `CHECK_MARKER` / `INTERRUPT` |

### Runie Implementation
- **Feed Data:** `crates/runie-tui/src/components/message_list/feed/mod.rs` → `Feed`, `FeedItem`
- **Render Module:** `crates/runie-tui/src/components/message_list/render/mod.rs`
- **User Render:** `crates/runie-tui/src/components/message_list/render/user.rs`
- **Assistant Render:** `crates/runie-tui/src/components/message_list/render/assistant.rs`
- **Tool Render:** `crates/runie-tui/src/components/message_list/render/tool.rs`
- **Tool Call:** `crates/runie-tui/src/components/message_list/render/tool_call.rs`

### Status: ✅ Matching
- Block types correctly mapped to glyphs
- Streaming indicators present
- Duration and byte counters supported

### Known Issues
- None

### Test Coverage
- `crates/runie-tui/src/components/message_list/feed/feed_tests.rs`
- `crates/runie-tui/src/components/message_list/feed/examples_tests.rs`
- `crates/runie-tui/src/components/message_list/render/messages_test.rs`

---

## 4. Activity Panel (Right Side)

### Grok Reference
```
                                                                               █
                                                                               █
     ◆ List .                                                                  █
     ◆ Read /Users/admin/.grok/docs/user-guide/README.md                       █
     ◆ Read Cargo.toml                                                         █
     ◆ List .                                                                  █
```

### Position
- Right side of viewport (when `screen_width >= 100`)
- Width: `ACTIVITY_PANEL_WIDTH` from `style/layout.rs`
- Auto-hides in narrow terminals

### States
| State | Display |
|-------|---------|
| **Empty** | "No active tasks" |
| **Running** | Tool name + progress bar + percentage |
| **Idle** | Progress bar empty |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/activity_panel/mod.rs`
- **Function:** `render_activity_panel`
- **Progress:** `PROGRESS_FILLED` / `PROGRESS_EMPTY` from `style/selection.rs`

### Status: ✅ Matching
- Shows tool list with diamond bullets
- ASCII progress bars (`█` blocks)
- Auto-scrolling follows agent activity
- Left border uses `accent_primary` color

### Known Issues
- None

### Test Coverage
- No dedicated tests (integration tested via full render tests)

---

## 5. Input Prompt

### Grok Reference
```
  ╭──────────────────────────────────────────────────────────────────────────╮
  │ ❯                                                                        │
  ╰───────────────────────────────────────────────────────────── Grok Build ─╯
```

### Position
- Bottom of viewport
- Full width minus margins
- Height: dynamic based on textarea content

### States
| State | Display |
|-------|---------|
| **Focused** | Cyan border (`accent_primary`) |
| **Unfocused** | Gray border (`border_unfocused`) |
| **Multiline** | Expands vertically |
| **With Mode** | Title shows mode: `Grok Build · plan` |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/input_bar/mod.rs`
- **Function:** `render_input_bar`
- **Builder:** `crates/runie-tui/src/components/input_bar/builder.rs`
- **Prompt Glyph:** `INPUT_PROMPT` = `CHEVRON_WITH_SPACE`

### Status: ✅ Matching
- Top border: `╭───...───╮` via `ratatui::symbols::border::ROUNDED`
- Bottom title: Grok-style dashes with version
- Mode indicator support: `· plan`, `· always-approve`
- File attachments rendered as pills with 📄 emoji

### Known Issues
- None

### Test Coverage
- No dedicated unit tests (rendered via integration tests)

---

## 6. Shortcuts Bar (Contextual Footer)

### Grok Reference
```
  Shift+Tab:mode  │  Ctrl+.:shortcuts
```

### Position
- Bottom of viewport (above status bar in some layouts)
- Full width

### Contextual Hints
| Context | Shortcuts |
|---------|-----------|
| Idle Prompt | `Shift+Tab:mode` `Ctrl+.:shortcuts` |
| Agent Running | `Shift+Tab:mode` `Ctrl+c:cancel` `Ctrl+Enter:interject` `Ctrl+.:` |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/shortcuts_panel/mod.rs`
- **Constants:** `SHORTCUTS` static array with 26 shortcuts across 5 sections
- **Sections:** Essentials, Input, Navigation, Actions, Session

### Status: ⚠️ Partial Match
- Shortcuts panel exists but rendered differently
- Grok shows inline hints; Runie has dedicated overlay panel (`Ctrl+.`)
- Hotkeys correctly contextualized in `status_bar/mod.rs` via `hotkeys_for_mode`

### Known Issues
- Inline shortcuts bar not rendered; only accessible via overlay

### Test Coverage
- No dedicated tests

---

## 7. Modes

### Grok Reference
| Mode | Title Suffix | Description |
|------|--------------|-------------|
| Normal | (none) | Standard operation |
| Plan | `· plan` | Plan-only mode |
| Always-Approve | `· always-approve` | Auto-approve mode |
| Subagents | `· subagents` | Parallel subagent panel |
| Ask | `· ask` | Questionnaire active |

### Runie Implementation
- **Enum:** `crates/runie-tui/src/tui/state/enums.rs` → `TuiMode`
- **Variants:** `Chat`, `HomeScreen`, `Plan`, `Onboarding`, `Subagents`, `Questionnaire`, `Overlay`, `Select`, `Permission`, `CommandPalette`, `DiffViewer`, `FullscreenViewer`, `SessionTree`

### Status: ✅ Matching
- Mode indicator shown in input bar title
- `Shift+Tab` cycles modes
- `hotkeys_for_mode` returns context-appropriate shortcuts

### Known Issues
- Mode cycling not explicitly tested

### Test Coverage
- `crates/runie-tui/src/tui/state/enums.rs` (implicit tests via integration)

---

## 8. Thinking Block

### Grok Reference
```
  ┃  ◆ Thinking…

  ┃  The user said "list src". They want to list the source files, probably
  ┃  the src directories across the crates, or perhaps the main source
  ┃  structure.
```

### Position
- Inline in scrollback
- Collapsible with `h` or `e`

### States
| State | Display |
|-------|---------|
| **Collapsed** | `┃  ◆ Thinking…` |
| **Expanded** | Header + content lines with `┃  ` prefix |
| **Streaming** | Animated accent line |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/message_list/render/thinking.rs`
- **Struct:** `ThinkingBlock`
- **Render:** `render_thinking_block`, `render_thought_indicator`
- **Glyphs:** Uses `spinner_frame` for animation

### Status: ✅ Matching
- Collapsed indicator: `┃  ◆ Thinking…`
- Vertical accent bar animates while reasoning
- Content lines prefixed with `┃  `
- Theme colors: `accent.thinking` for accent, `bg.panel` for background

### Known Issues
- None

### Test Coverage
- `crates/runie-tui/src/components/message_list/render/` (render tests)

---

## 9. Tool Call Block

### Grok Reference
```
  ⠴ Run List `.` 1.8s                                        5.7s ⇣21.2k [✗]
```

### Position
- Inline in scrollback
- Single line per tool

### Components
| Component | Grok | Runie |
|-----------|------|-------|
| Spinner | `⠦⠴⠋⠼` | `SPINNER_FRAMES` array |
| Label | `Run` + tool name + args | `format: "{} Run {} '{}' {:.1}s"` |
| Duration | `1.8s` | `elapsed_secs` |
| Total | `5.7s` | `total_secs` |
| Bytes | `⇣21.2k` | `format_bytes()` |
| Status | `[✗]` | `✓` / `✗` |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/message_list/render/tool_call.rs`
- **Struct:** `ToolCallBlock`
- **Status Enum:** `ToolStatus` (Running, Complete, Error)
- **Spinners:** Lines 27: `&['⠦', '⠴', '⠋', '⠼', '⠦', '⠴', '⠂', '⠇']`

### Status: ✅ Matching
- Spinner animation present
- Tool name and args displayed
- Duration and byte transfer shown
- Success/error indicators with colors

### Known Issues
- None

### Test Coverage
- No dedicated unit tests (rendered via feed tests)

---

## 10. Response Block

### Grok Reference
```
     Want me to:
     • Show a specific crate in detail?
     • List only source files (exclude target/)?
     • Read a particular file (e.g. root Cargo.toml, README, or
     main entrypoint)?
```

### Position
- Inline in scrollback
- Follows assistant message header

### Features
- Bullet points: `•` for lists
- Tree structure: `└── tests/` for directory listings
- Suggestions: Next-step recommendations

### Runie Implementation
- **File:** `crates/runie-tui/src/components/message_list/render/assistant.rs`
- **Markdown:** `crates/runie-tui/src/components/message_list/render/markdown.rs`
- **Bullet Glyph:** `BULLET` (`\u{2022}`) from `glyphs.rs`

### Status: ✅ Matching
- Markdown rendering with syntax highlighting
- Bullet points correctly styled
- Tree structure support via `branch.rs`

### Known Issues
- None

### Test Coverage
- `crates/runie-tui/src/components/message_list/render/messages_test.rs`

---

## 11. Extensions Modal

### Grok Reference
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Hooks] [Plugins] [Marketplace] [Skills] [MCP Servers]                      │
├─────────────────────────────────────────────────────────────────────────────┤
│ / to search                                        Workspace ⌄             │
├─────────────────────────────────────────────────────────────────────────────┤
│ › team-tool                                    (project)  [install]         │
│ › browser-review v0.8.2                      (workspace) [install]         │
│ › github-flow v2.1.0                         (workspace) [installed]        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Position
- Centered modal overlay
- Full-width on narrow terminals

### Tabs
| Tab | Purpose |
|-----|---------|
| Hooks | Pre/post action hooks |
| Plugins | Local plugin management |
| Marketplace | Browse and install |
| Skills | Skill bundles |
| MCP Servers | Protocol connections |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/extensions_modal/mod.rs`
- **Builder:** `crates/runie-tui/src/components/extensions_modal/builder.rs`
- **Render:** `crates/runie-tui/src/components/extensions_modal/render.rs`
- **Enums:** `ExtensionTab`, `ExtensionScope`, `ExtensionAction`
- **Items:** `ExtensionItem` struct

### Status: ✅ Matching
- All 5 tabs implemented
- Tab labels match exactly
- Scope badges: `(project)`, `(workspace)`
- Action buttons: `[install]`, `[installed]`, `[update]`
- Search prompt: `/ to search`
- Expand indicator: `›`

### Known Issues
- Mock data only (not connected to actual extension system)

### Test Coverage
- No dedicated tests

---

## 12. Interactive Questionnaire Panel

### Grok Reference
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ● ○ ● ○ ● ○  Waiting on answers for 3 questions              [turn: 7.1s]  │
├─────────────────────────────────────────────────────────────────────────────┤
│  1  ○  Minimal & terminal-native                                         │
│          Clean, keyboard-first, no excess chrome                           │
│  2  ○  Bold & expressive                                                  │
│          Strong visuals, gradients, animations                             │
│  3  ○  Developer-focused                                                  │
│          Code-first aesthetic, technical precision                          │
│  4  ○  Other                                                              │
│          Define custom principles                                           │
│  z  ○  Type your answer here                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  [1/3]  ↑/↓ navigate  ←/→ question  Enter:select                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Position
- Centered modal overlay
- Full-width on narrow terminals

### Elements
| Element | Grok | Runie |
|---------|------|-------|
| Dot Grid | `● ○ ●` | `STATUS_ACTIVE` / `STATUS_IDLE` |
| Radio | `○` / `◉` | `RADIO_UNSELECTED` / `RADIO_SELECTED` |
| Navigation | `[1/3]` | `current_question + 1` / `total` |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/questionnaire_panel/mod.rs`
- **Structs:** `QuestionnaireState`, `Question`, `QuestionOption`
- **Render:** `render_questionnaire`
- **Glyphs:** `STATUS_ACTIVE` (`●`), `STATUS_IDLE` (`○`), `RADIO_SELECTED` (`◉`), `RADIO_UNSELECTED` (`○`)

### Status: ✅ Matching
- Dot grid pattern correctly rendered
- Numbered options with subtitles
- Custom input option (`z`)
- Progress indicator `[current/total]`
- Footer navigation hints

### Known Issues
- None

### Test Coverage
- No dedicated tests

---

## 13. Plan Modal

### Grok Reference
```
┌─────────────────────────────── plan.md ───────────────────────────────┐
│                                                                        │
│   1  Install Docs Refresh Plan                                         │
│   2  Quick Assessment                                                  │
│   3  • docs/install.md skips headless mode...                          │
│   4  Implementation Plan                                               │
│   5  1. Replace the install snippet with curl bootstrap                │
│   6  2. Document `-p` headless mode                                   │
│   7  3. Point users to config.toml for models                         │
│   8  4. Cross-link the auth and feedback sections                      │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### Position
- Centered modal
- Width: auto-sized to content

### Elements
| Element | Description |
|---------|-------------|
| Title Divider | `─── plan.md ───` |
| Section Headers | Colored labels |
| Numbered Steps | Sequential implementation items |
| Bullet Points | Supporting details |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/plan_modal/mod.rs`
- **Structs:** `PlanModal`, `PlanDocument`, `PlanSection`, `PlanItem`
- **Render:** `render_plan_document`, `render_step_item`, `render_bullet_item`
- **Title Divider:** `render_title_divider` using `box_chars::H`

### Status: ✅ Matching
- Title divider: `─── plan.md ───`
- Section headers in colored text
- Numbered steps with prefix
- Bullet points with `•`
- Footer shortcuts: `Enter:approve  Esc:close  ↑↓:scroll`

### Known Issues
- None

### Test Coverage
- No dedicated tests (structural tests via integration)

---

## 14. Subagent Panel

### Grok Reference (Dedicated Panel Mode)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ projects/main jasong/folder                                       77.54% │
├─────────────────────────────────────────────────────────────────────────────┤
│ [explore] Explore checkout flow                     explore · grok-build    │
│ [explore] Explore infra and CI                     explore · grok-build    │
│ [explore] Explore shared Go libraries              explore · grok-build    │
│ [explore] Explore order services                    explore · grok-build    │
│ [explore] Explore fulfillment jobs                  explore · grok-build    │
│ [explore] Explore pricing engine                    explore · grok-build    │
├─────────────────────────────────────────────────────────────────────────────┤
│ ❯ find the source of the p99 latency regression                        │
└─────────────────────────────────────────────────────────────────────────────┘
                               ┌──────────────────────────────────┐
                               │ ❯ find the source of the...      │
                               │         grok-build · subagents    │
                               └──────────────────────────────────┘
```

### Dot Grid Pattern
```
● ○ ●    ● = Active/working
● ○ ●    ○ = Idle/waiting
```

### Runie Implementation
- **File:** `crates/runie-tui/src/components/subagent_panel/mod.rs`
- **Structs:** `SubagentPanel`, `Subagent`, `SubagentStatus`
- **Render:** `render_subagent_panel`, `render_subagent_grid`, `render_subagent_list`
- **Glyphs:** `STATUS_ACTIVE` (`●`), `STATUS_IDLE` (`○`)

### Status: ✅ Matching
- Header with context name and progress percentage
- 2x3 dot grid for activity status
- Agent labels: `[explore]`, `[general]`, `[code]`
- Footer with shortcuts

### Known Issues
- Context suffixes (`grok-build · plan`, etc.) not shown in Runie

### Test Coverage
- No dedicated tests

---

## 15. Inline Diff Viewer

### Grok Reference
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ◆ Thought for 2.3s                                                        │
│                                                                             │
│  41  Install the CLI with...                                               │
│  42+ curl -fsSL x.ai/cli/install.sh | bash                                 │
│  43                                                                         │
│  44- Run the CLI and follow the prompts.                                   │
│  44+ Run `grok-build -p` to use the CLI in headless ACP-compatible mode.   │
│  45+ Sign in once, then configure models and API keys in `config.toml`.     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Diff Color System
| Variable | Default | Purpose |
|----------|---------|---------|
| `--terminal-diff-gutter` | `#6c6c6c` | Line number color |
| `--terminal-diff-insert-bg` | `#202a16` | Added line background |
| `--terminal-diff-delete-bg` | `#32181c` | Removed line background |
| `--terminal-diff-insert-fg` | `#9ece6a` | Added line (green) |
| `--terminal-diff-delete-fg` | `#f7768e` | Removed line (red) |

### Runie Implementation
- **File:** `crates/runie-tui/src/components/diff_viewer.rs`
- **Struct:** `DiffViewer`
- **Diff Lines:** `DiffLine` enum (Removed, Added, Context)
- **Render:** `render_diff_lines`, `render_diff_line`
- **Colors:** Theme colors for `diff.removed`, `diff.added`, `diff.removed_bg`, `diff.added_bg`

### Status: ✅ Matching
- Line numbers in gutter
- Prefix chars: `-` (removed), `+` (added), ` ` (context)
- Background colors for added/removed lines
- Right-aligned line numbers

### Known Issues
- Gutter width fixed at `BORDER_WIDTH * 5`
- Scroll offset implemented but limited

### Test Coverage
- `crates/runie-tui/src/components/diff_viewer.rs` tests (lines 240-322)

---

## 16. Additional Color Variables

### Grok Reference
| Variable | Default | Purpose |
|----------|---------|---------|
| `--terminal-editor-bg` | (theme base) | Editor/panel background |
| `--terminal-surface` | `#202020` | Elevated surface |
| `--terminal-popover` | `#1a1a1a` | Modal/popover background |
| `--terminal-teal` | `#29c6be` | Teal accent |
| `--terminal-orange` | `#d59556` | Orange for file labels |
| `--terminal-purple` | `#bc97ff` | Purple for thinking blocks |
| `--terminal-yellow` | `#cfb47c` | Yellow for plan indicator |
| `--terminal-blue-bright` | `#88a6ff` | Bright blue for active |

### Runie Implementation
- **File:** `crates/runie-tui/src/theme/themes/` (theme definitions)
- **GrokNight:** `grok_night.rs`
- **TokyoNight:** `tokyo_night.rs`
- **RosePineMoon:** `rose_pine_moon.rs`
- **GrokDay:** `grok_day.rs`
- **Accent Colors:** `accent_teal`, `accent_orange`, `accent_purple`, `accent_yellow`, `accent_blue_bright` in `ThemeColors`

### Status: ✅ Matching
- All color slots defined in `ThemeColors` struct
- Theme files define RGB values for each theme
- Color quantization for 256-color and 16-color terminals

### Known Issues
- None

### Test Coverage
- Theme tests via rendering integration

---

## 17. Glyphs Reference

### Grok-to-Runie Mapping
| Grok Glyph | Unicode | Runie Constant | Value |
|------------|---------|----------------|-------|
| `❯` | U+276F | `CHEVRON` | `'\u{276F}'` |
| `◆` | U+25C6 | `DIAMOND` | `'\u{25C6}'` |
| `∘` | U+2218 | `ASSISTANT_BULLET` | `'\u{2218}'` |
| `›` | U+203A | (text) | `>` |
| `│` | U+2502 | `VERTICAL` | `'│'` |
| `─` | U+2500 | `HORIZONTAL` | `'─'` |
| `╭╮╰╯` | Box | ratatui | `border::ROUNDED` |
| `█` | U+2588 | `PROGRESS_FILLED` | `'█'` |
| `⇣` | U+21E3 | (text) | download indicator |
| `✓` | U+2713 | `CHECK_MARKER` | `'✓'` |
| `✗` | U+2717 | `INTERRUPT` | `'✗'` |
| `●` | U+25CF | `STATUS_ACTIVE` | `'●'` |
| `○` | U+25CB | `STATUS_IDLE` | `'○'` |
| `◉` | U+25C9 | `RADIO_SELECTED` | `'◉'` |
| `⠦⠴⠋⠼` | Braille | `SPINNER_FRAMES` | 10-frame array |
| `` | Powerline | `GIT_BRANCH_SYMBOL` | `'\u{E0A0}'` |

### Runie Implementation
- **File:** `crates/runie-tui/src/glyphs.rs`
- **Selection:** `crates/runie-tui/src/style/selection.rs`

### Status: ✅ Matching
- All key glyphs defined
- Spinner animation via `spinner_frame(tick)`
- Git branch symbol for Powerline users

### Known Issues
- None

---

## 18. Status Bar (Shortcuts Bar)

### Grok Reference
Context-aware hints at bottom of screen, not shown in main viewport (separate overlay).

### Runie Implementation
- **File:** `crates/runie-tui/src/components/status_bar/mod.rs`
- **ViewModel:** `StatusBarViewModel`
- **Hotkey Methods:**
  - `agent_running_hotkeys()` - 4 hints during agent execution
  - `chat_hotkeys()` - 2 hints for idle/input
  - `permission_hotkeys()` - y/n/a for approvals
  - `palette_hotkeys()` - navigation for command palette

### Status: ✅ Matching
- Context-aware hotkey selection
- Agent running: `Shift+Tab:mode`, `Ctrl+c:cancel`, `Ctrl+Enter:interject`, `Ctrl+.:shortcuts`
- Idle: `Shift+Tab:mode`, `Ctrl+.:shortcuts`

### Known Issues
- Shortcuts panel is overlay, not inline bar

### Test Coverage
- `crates/runie-tui/src/components/status_bar/tests_status_bar_onboarding.rs`

---

## 19. Permission Modal

### Grok Reference
Tool call permission prompts during execution.

### Runie Implementation
- **File:** `crates/runie-tui/src/components/permission_modal/builder.rs`
- **ViewModel:** `PermissionModalViewModel` (in `tui/view_models.rs`)
- **Builder:** `PermissionBuilder`

### Status: ⚠️ Partial
- Builder pattern exists
- Not fully integrated with tool execution flow

### Known Issues
- Permission modal view model may not be fully connected

### Test Coverage
- No dedicated tests

---

## 20. Command Palette

### Grok Reference
Triggered via `Ctrl+P` or `?`

### Runie Implementation
- **File:** `crates/runie-tui/src/components/command_palette/mod.rs`
- **Builder:** `crates/runie-tui/src/components/command_palette/builder.rs`
- **Render:** `crates/runie-tui/src/components/command_palette/render.rs`
- **Tests:** `crates/runie-tui/src/components/command_palette/tests.rs`

### Status: ✅ Implemented
- Fuzzy search
- Section filtering
- Scoring system

### Known Issues
- None

### Test Coverage
- `crates/runie-tui/src/components/command_palette/tests.rs`
- `crates/runie-tui/src/components/command_palette/tests_scoring.rs`
- `crates/runie-tui/src/components/command_palette/tests_bugs.rs`

---

## Summary

| Element | Status | Implementation File |
|---------|--------|-------------------|
| 1. Header Bar | ✅ | `top_bar/render.rs` |
| 2. Welcome Screen | ✅ | `home_screen/mod.rs` |
| 3. Scrollback | ✅ | `message_list/` |
| 4. Activity Panel | ✅ | `activity_panel/mod.rs` |
| 5. Input Prompt | ✅ | `input_bar/mod.rs` |
| 6. Shortcuts Bar | ⚠️ | `status_bar/mod.rs` |
| 7. Modes | ✅ | `tui/state/enums.rs` |
| 8. Thinking Block | ✅ | `message_list/render/thinking.rs` |
| 9. Tool Call Block | ✅ | `message_list/render/tool_call.rs` |
| 10. Response Block | ✅ | `message_list/render/assistant.rs` |
| 11. Extensions Modal | ✅ | `extensions_modal/mod.rs` |
| 12. Questionnaire Panel | ✅ | `questionnaire_panel/mod.rs` |
| 13. Plan Modal | ✅ | `plan_modal/mod.rs` |
| 14. Subagent Panel | ✅ | `subagent_panel/mod.rs` |
| 15. Diff Viewer | ✅ | `diff_viewer.rs` |
| 16. Color Variables | ✅ | `theme/` |
| 17. Glyphs | ✅ | `glyphs.rs`, `style/selection.rs` |
| 18. Status Bar | ✅ | `status_bar/mod.rs` |
| 19. Permission Modal | ⚠️ | `permission_modal/` |
| 20. Command Palette | ✅ | `command_palette/` |

**Total: 17 ✅ Matching, 3 ⚠️ Partial**

---

## Test Coverage Summary

| Component | Test File |
|-----------|-----------|
| Top Bar | `top_bar/tests.rs`, `top_bar/helpers_test.rs` |
| Home Screen | `home_screen/mod_test.rs`, `home_screen/render_test.rs` |
| Message List | `message_list/feed/feed_tests.rs`, `message_list/render/messages_test.rs` |
| Diff Viewer | `diff_viewer.rs` (lines 240-322) |
| Status Bar | `status_bar/tests_status_bar_onboarding.rs` |
| Command Palette | `command_palette/tests.rs`, `tests_scoring.rs`, `tests_bugs.rs` |
| Glyphs | `glyphs.rs` tests |

**Gap Areas:** Extensions Modal, Questionnaire Panel, Plan Modal, Subagent Panel, Activity Panel, Permission Modal lack dedicated unit tests.
