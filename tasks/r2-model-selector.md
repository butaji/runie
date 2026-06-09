# Model Selector

**Status**: todo
**Milestone**: R2
**Category**: TUI Rendering

## Description

A dedicated dialog for selecting models. More optimized than the generic palette because model switching is the most frequent action. Groups by provider, shows costs, supports fuzzy filter.

**Design inspiration:**
- **Crush**: `Models` dialog with filterable list, provider tabs
- **pi**: `Ctrl+L` opens interactive picker with provider grouping

## Architecture

### Dialog State

```rust
pub enum DialogState {
    // ... other variants ...
    ModelSelector {
        filter: String,
        selected: usize,
        models: Vec<ModelInfo>,       // cached from provider catalog
        recent: Vec<String>,          // last 5 used models
    },
}

pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub display_name: String,
    pub cost_prompt: Option<f64>,
    pub cost_completion: Option<f64>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
}
```

### Events

```rust
Event::ToggleModelSelector,      // Ctrl+L, /model (no args)
Event::ModelSelectorFilter(char),
Event::ModelSelectorUp,
Event::ModelSelectorDown,
Event::ModelSelectorSelect,      // Enter
Event::ModelSelectorClose,       // Esc
```

### Visual Design

```
┌─ Select Model ───────────────────────────────┐
│ > claude                                     │
├──────────────────────────────────────────────┤
│  ★ Recent                                    │
│    gpt-4o                        $5.00/1M   │
│  ──────────────────────────────────────────  │
│  Anthropic                                   │
│  ▸ claude-3-5-sonnet             $3.00/1M   │
│    claude-3-opus                 $15.00/1M  │
│    claude-3-haiku                $0.25/1M   │
│  ──────────────────────────────────────────  │
│  OpenAI                                      │
│    gpt-4o-mini                   $0.15/1M   │
└──────────────────────────────────────────────┘
```

### Provider Grouping

Models grouped by provider with sticky headers. Filter matches name, provider, or display name.

```rust
fn filter_models(models: &[ModelInfo], query: &str) -> Vec<&ModelInfo> {
    let q = query.to_lowercase();
    models.iter()
        .filter(|m| {
            m.name.to_lowercase().contains(&q) ||
            m.provider.to_lowercase().contains(&q) ||
            m.display_name.to_lowercase().contains(&q)
        })
        .collect()
}
```

## Acceptance Criteria

- [ ] `Ctrl+L` opens model selector
- [ ] `/model` (no args) opens same dialog
- [ ] Shows all ~130 models grouped by provider
- [ ] "Recent" section at top (last 5 used)
- [ ] Fuzzy filter as user types
- [ ] Arrow Up/Down navigates
- [ ] Enter selects → `Event::SwitchModel { provider, model }`
- [ ] Esc closes without selection
- [ ] Cost shown as `$X.YY/1M` when available
- [ ] Current model marked with `★`

## Files

| File | Lines | Description |
|------|-------|-------------|
| `crates/runie-core/src/model.rs` | +30 | `ModelInfo`, `DialogState::ModelSelector` |
| `crates/runie-core/src/event.rs` | +5 | Model selector events |
| `crates/runie-core/src/update/dialog.rs` | +60 | Model selector update logic |
| `crates/runie-tui/src/ui.rs` | ~100 | `render_model_selector()` |
| `crates/runie-term/src/keymap.rs` | +1 | Map Ctrl+L |

## Tests

### Layer 1 — State/Logic
- [ ] `filter_matches_name` — "gpt" shows OpenAI models
- [ ] `filter_matches_provider` — "anthropic" filters by provider
- [ ] `filter_case_insensitive` — "GPT" matches "gpt-4o"
- [ ] `recent_shows_max_5` — recent list capped at 5
- [ ] `select_emits_switch_model` — selection produces correct event

### Layer 2 — Event Handling
- [ ] `ctrl_l_opens_selector` — keymap event pushes dialog
- [ ] `slash_model_no_args_opens_selector` — `/model` with no args

### Layer 3 — Rendering
- [ ] `selector_renders_groups` — TestBackend shows provider headers
- [ ] `selector_shows_cost` — cost badge visible
- [ ] `selector_marks_current` — current model has star

### Layer 4 — Smoke
- [ ] `model_selector_no_panic.sh` — tmux: Ctrl+L → filter → Enter → Esc
