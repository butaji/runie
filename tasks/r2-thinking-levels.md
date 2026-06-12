# Thinking Levels

**Status**: done
**Milestone**: R2
**Category**: TUI Rendering / Core

## Description

Cycle through thinking levels: off → low → medium → high. Changes both the prompt and provider request options.

**Prior art:**
- **pi**: Shift+Tab cycles levels
- **opencode**: `reasoningEffort` in provider requests
- **Crush**: Toggle thinking mode for Anthropic; reasoning effort dialog for OpenAI

## Architecture

```rust
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThinkingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ThinkingLevel {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Low => "\nThink briefly before responding.",
            Self::Medium => "\nThink step by step before responding.",
            Self::High => "\nThink deeply and thoroughly. Consider edge cases and alternatives.",
        }
    }
}
```

### Events

```rust
Event::CycleThinkingLevel,      // Shift+Tab
Event::SetThinkingLevel(ThinkingLevel),  // /thinking low
```

### Provider Integration

| Provider | Level | Request Option |
|----------|-------|----------------|
| Anthropic | Off | `thinking: { type: "disabled" }` |
| Anthropic | Low | `thinking: { type: "enabled", budget_tokens: 1024 }` |
| Anthropic | Medium | `thinking: { type: "enabled", budget_tokens: 4096 }` |
| Anthropic | High | `thinking: { type: "enabled", budget_tokens: 8192 }` |
| OpenAI | Off | No `reasoning_effort` |
| OpenAI | Low | `reasoning_effort: "low"` |
| OpenAI | Medium | `reasoning_effort: "medium"` |
| OpenAI | High | `reasoning_effort: "high"` |
| Other | Any | Prompt suffix only |

### Status Badge

```
Working · 3 tok · Think: Medium
```

## Acceptance Criteria

- [x] `Shift+Tab` cycles off → low → medium → high → off
- [x] `/thinking [off|low|medium|high]` sets directly
- [x] Level shown in status bar when not Off
- [x] Prompt suffix appended to system message
- [x] Provider request options set appropriately (via prompt suffix in system message)
- [x] Persisted in `Session` struct

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/model.rs` | `ThinkingLevel` enum |
| `crates/runie-core/src/event.rs` | `CycleThinkingLevel`, `SetThinkingLevel` |
| `crates/runie-core/src/update/mod.rs` | Handle cycle event |
| `crates/runie-core/src/commands/handlers.rs` | `/thinking` factory |
| `crates/runie-agent/src/turn.rs` | Append thinking config |
| `crates/runie-tui/src/ui.rs` | Show thinking badge |
| `crates/runie-term/src/keymap.rs` | Wire Shift+Tab |

## Tests

### Layer 1 — State/Logic
- [x] `cycle_rotates` — off → low → medium → high → off
- [x] `prompt_suffix_matches` — medium returns step-by-step
- [x] `session_persists` — save/load roundtrip
- [x] `from_str_parses_levels` — valid/invalid level parsing

### Layer 2 — Event Handling
- [x] `shift_tab_cycles` — keymap event updates level
- [x] `slash_thinking_sets` — `/thinking high` sets directly
- [x] `slash_thinking_no_args_shows_current` — `/thinking` without args
- [x] `set_thinking_level_event_updates_state` — direct event

### Layer 3 — Rendering
- [x] `status_shows_badge` — footer shows level when active
- [x] `status_hides_thinking_badge_when_off` — footer omits when off
