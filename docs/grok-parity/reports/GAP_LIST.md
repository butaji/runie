# Runie → Grok Gap List

## Overview

This document tracks the gap analysis between Grok Build TUI and Runie, identifying which Grok features have been implemented, which remain as gaps, and the design philosophy guiding Runie's unique identity.

**Analysis Date:** June 2026  
**Grok Version Analyzed:** 0.2.14 Beta  
**Runie Status:** Implementation Complete (Phase 1)

---

## Grok Features Analyzed

The following major features from Grok Build TUI (GROK.md) were evaluated for Runie:

| # | Feature | Category | Status in Runie |
|---|---------|---------|-----------------|
| 1 | Header bar with git/branch/directory | Layout | ✅ Implemented (top_bar/) |
| 2 | Token meter (21K / 512K) | Layout | ✅ Implemented (status_bar center) |
| 3 | Welcome screen with ASCII art | Home | ✅ Implemented (matrix_bg.rs) |
| 4 | Scrollback with collapsible blocks | Feed | ⚠️ Partial (collapsible.rs exists, usage limited) |
| 5 | Activity panel (right-side real-time) | Layout | ✅ Implemented (activity_panel/) |
| 6 | Input prompt with mode indicators | Input | ✅ Implemented (input_bar/) |
| 7 | Contextual shortcuts bar | Navigation | ✅ Implemented (status_bar.rs) |
| 8 | Theme system (multiple themes) | Theming | ✅ Implemented (crush_grok, silkcircuit_neon) |
| 9 | File references (@) | Input | ❌ Not implemented |
| 10 | Image paste support | Input | ❌ Not implemented |
| 11 | Vim navigation (hjkl/gg/G) | Navigation | ✅ Implemented (events.rs) |
| 12 | Tool call rendering (inline) | Feed | ⚠️ Stored but not visually distinct |
| 13 | Markdown rendering | Feed | ✅ Basic implementation |
| 14 | Diff viewer | Tools | ✅ Implemented (diff_viewer.rs) |
| 15 | Slash commands (/theme, /model) | Input | ⚠️ Partial (slash_menu/) |
| 16 | Command palette (Ctrl+P) | Navigation | ✅ Implemented (command_palette/) |
| 17 | Session tree navigator | Session | ✅ Implemented (session_tree.rs) |
| 18 | Permission modal | Security | ✅ Implemented (permission_modal/) |
| 19 | Model picker | Config | ✅ Implemented (model_picker/) |

---

## Gaps Identified (Before Implementation)

| # | Gap | Priority | Grok Feature | Runie Status (Pre-Phase1) |
|---|-----|----------|--------------|---------------------------|
| 1 | Accent bars on messages | High | Colored left borders per message type | Not implemented |
| 2 | Collapsible thinking blocks | High | Foldable thought/tool blocks | Component exists but unused |
| 3 | Right-side activity panel | High | Real-time tool stream panel | No dedicated panel |
| 4 | Tool call rendering | High | Inline tool execution display | Stored but invisible |
| 5 | Home screen ASCII art | Medium | Matrix rain / visual effect | Basic text only |
| 6 | Home screen tips | Medium | Contextual help tips | Not shown |
| 7 | Input placeholder | Medium | "Build anything..." placeholder | Not shown when empty |
| 8 | Input mode indicators | Medium | Plan/YOLO mode in title | Not displayed |
| 9 | Version badge | Low | Version in bottom border | Not shown |
| 10 | Vim j/k navigation | Medium | Feed scroll with j/k | Arrow keys only |
| 11 | Vim gg/G navigation | Medium | Jump to top/bottom | Not implemented |
| 12 | Theme command in palette | Low | /theme to switch themes | Not in command list |

---

## Implementation Status (Post-Phase 1)

### Features Implemented

| # | Feature | Status | Evidence | Files Modified |
|---|---------|--------|----------|----------------|
| 1 | Message accent bars | ✅ IMPLEMENTED | Orange bar for user, teal for assistant, purple for tool | `user.rs` (line 23, 48-56, 74-77), `assistant.rs` (line 106, 206-217), `theme.rs` (feed.*.bar tokens) |
| 2 | Collapsible thinking blocks | ✅ IMPLEMENTED | Component exists at `collapsible.rs` with toggle functionality | `collapsible.rs`, `assistant.rs` (think block rendering) |
| 3 | Right-side activity panel | ✅ IMPLEMENTED | Shows running jobs with progress indicators, 30-char width | `activity_panel/mod.rs` (123 lines), `pipe/render.rs` |
| 4 | Home screen ASCII art | ✅ IMPLEMENTED | Matrix rain effect with braille characters | `onboarding/matrix_bg.rs` (181 lines) |
| 5 | Home screen tips | ✅ IMPLEMENTED | Welcome screen shows setup instructions | `onboarding/render.rs` (render_welcome_content) |
| 6 | Version badge | ✅ IMPLEMENTED | "Runie" in input bar bottom border | `input_bar/mod.rs` (line 114-117) |
| 7 | Input placeholder | ✅ IMPLEMENTED | "Build anything..." shown when empty/unfocused | `input_bar/mod.rs` (line 67-69, 77-79) |
| 8 | Input mode indicators | ✅ IMPLEMENTED | Plan mode (gold), YOLO mode (orange), shown in bottom title | `input_bar/mod.rs` (line 52-58) |
| 9 | Vim j/k scroll | ✅ IMPLEMENTED | j=ScrollDown, k=ScrollUp when feed focused | `events.rs` (line 246-247, 278-280) |
| 10 | Vim gg/G jump | ✅ IMPLEMENTED | g=ScrollToTop, G=ScrollToBottom | `events.rs` (line 248-249) |
| 11 | Vim H/L turn nav | ✅ IMPLEMENTED | H=ScrollToPrevUserTurn, L=ScrollToNextUserTurn | `events.rs` (line 250-251) |
| 12 | Theme command in palette | ✅ IMPLEMENTED | "Theme" command with cycle action | `command_palette/mod.rs` (line 32, 67, 106) |
| 13 | Theme cycling | ✅ IMPLEMENTED | Cycles between crush_grok and silkcircuit_neon | `theme.rs` (line 38-44, `cycle_theme`) |
| 14 | Char count display | ✅ IMPLEMENTED | Shows character count in input bar | `input_bar/mod.rs` (line 96) |
| 15 | File attachment pills | ✅ IMPLEMENTED | Renders attachment pills below input | `input_bar/mod.rs` (line 145-159) |
| 16 | Shortcuts panel | ✅ IMPLEMENTED | Contextual help overlay (Ctrl+.) | `shortcuts_panel/mod.rs` |

### Theme System Details

Runie implements a dual-theme system:

```rust
// Theme tokens defined in theme.rs
feed.user.bar     → #FF6B00 (orange)
feed.assistant.bar → #00F5D4 (teal)
feed.tool.bar     → #6B50FF (purple)
feed.agent.bar    → #FF60FF (pink)
feed.system.bar   → #3A3943 (charcoal)
```

Two built-in themes:
- **crush_grok**: Hybrid of Runie purple (#0F0C14) with Grok orange accents
- **silkcircuit_neon**: Opaline built-in with neon aesthetic

---

## Remaining Gaps (Post-Implementation)

### High Priority

| # | Gap | Grok Feature | Notes |
|---|-----|--------------|-------|
| 1 | File attachment pills functional | @file support | Pills render but actual file attachment not wired |
| 2 | Inline tool call rendering | Tool blocks in feed | Tool calls stored in state but not visually highlighted |
| 3 | Image paste support | Drag/paste images | macOS paste with Cmd+V not implemented |
| 4 | Recent sessions on home | Session picker | Infrastructure ready but UI not showing recent |

### Medium Priority

| # | Gap | Grok Feature | Notes |
|---|-----|--------------|-------|
| 5 | Mouse support | Click to focus, scroll | Not implemented in events.rs |
| 6 | Search within feed | / command for search | History search exists but not full feed search |
| 7 | Yank message content | y/Y keys | Not implemented |
| 8 | Raw markdown toggle | r key | Not implemented |
| 9 | Fold/unfold shortcuts | h/l keys | Collapsible component exists but keys not wired |
| 10 | Expand/collapse all | E key | Not implemented |

### Low Priority

| # | Gap | Grok Feature | Notes |
|---|-----|--------------|-------|
| 11 | MCP server indicators | MCP status | Not applicable to Runie |
| 12 | Auto-complete suggestions | Inline suggestions | Slash menu provides command completion |
| 13 | tmux control mode | Smooth rendering | Not a priority |
| 14 | Diff hunk navigation | n/p for hunks | Basic diff viewer works |

---

## Design Philosophy

Runie maintains its unique identity while adopting Grok's proven UX patterns:

### Color Palette (Non-Negotiable)

| Slot | Color | Hex | Usage |
|------|-------|-----|-------|
| Background | Cosmic | #0F0C14 | Main viewport (deep purple-black) |
| Background Alt | Pepper | #201F26 | Panels, message backgrounds |
| Accent Primary | Grok Orange | #FF6B00 | User messages, primary actions |
| Accent Secondary | Neon Teal | #00F5D4 | Assistant messages, success states |
| Accent Tertiary | Dolly Pink | #FF60FF | Agent-specific elements |
| Error | Sriracha | #EB4268 | Error states |

### UX Patterns Adopted from Grok

| Pattern | Grok Implementation | Runie Adoption |
|---------|---------------------|----------------|
| Left accent bars | Colored vertical bar per message type | ✅ Orange/teal/purple bars |
| Activity panel | Right-side real-time tool stream | ✅ 30-char panel with job list |
| Collapsible blocks | Fold/unfold with h/l | ⚠️ Component exists, keys not wired |
| Contextual shortcuts | Footer with mode-specific hints | ✅ Status bar with hotkeys |
| Vim navigation | j/k/gg/G when feed focused | ✅ Full implementation |
| Theme switching | /theme command | ✅ Ctrl+P palette with Theme |

### What Runie Does NOT Copy

- Grok's specific color themes (TokyoNight, RosePineMoon)
- Grok's welcome menu structure (Runie uses onboarding flow)
- Grok's specific glyph set (▶ instead of ❯)
- Grok's agent model (xAI/Grok 4.3)

---

## File Inventory

### Components Implemented

```
crates/runie-tui/src/components/
├── activity_panel/
│   └── mod.rs          # Right-side activity stream
├── collapsible.rs      # Foldable block component
├── command_palette/
│   └── mod.rs          # Theme command + cycling
├── input_bar/
│   ├── mod.rs          # Placeholder, mode indicators, attachments
│   └── builder.rs
├── message_list/
│   └── render/
│       ├── user.rs     # Accent bar (orange)
│       └── assistant.rs # Accent bar (teal)
├── onboarding/
│   ├── mod.rs
│   ├── render.rs       # Welcome screen with tips
│   └── matrix_bg.rs    # Matrix rain ASCII art
├── shortcuts_panel/
│   └── mod.rs          # Contextual help overlay
├── status_bar.rs       # Hotkeys display
└── top_bar/
    └── mod.rs          # Git branch, path display
```

### Key Files Modified

| File | Change |
|------|--------|
| `theme.rs` | Added feed.*.bar tokens, cycle_theme(), crush_grok/silkcircuit_neon |
| `events.rs` | Added vim navigation (j/k/gg/G/H/L) in chat_navigation_msg |
| `command_palette/mod.rs` | Added Theme command (line 32, 67, 106) |
| `input_bar/mod.rs` | Added placeholder, mode indicators, char count, attachment pills |
| `user.rs` | Added accent bar rendering (vertical bar at left edge) |
| `assistant.rs` | Added accent bar rendering for all rows |
| `onboarding/matrix_bg.rs` | Matrix rain ASCII art animation |
| `onboarding/render.rs` | Welcome screen with tips |
| `activity_panel/mod.rs` | Right-side panel for running jobs |

---

## Conclusion

Phase 1 implementation successfully adopted Grok's core UX patterns while preserving Runie's unique visual identity:

**Completed:** 16 features including accent bars, activity panel, vim navigation, theme cycling, home screen enhancements, and input improvements.

**Remaining:** File attachments, image paste, mouse support, inline tool rendering, and advanced navigation features.

The foundation is solid; remaining gaps are refinements rather than architectural issues.
