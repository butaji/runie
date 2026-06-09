# Scoped Model Filtering (/scoped-models)

**Status**: todo
**Milestone**: R2
**Category**: Input & Commands

## Description

Configure which models appear in the Ctrl+P cycling list. Users can enable/disable individual models or entire providers.

## Architecture

```rust
pub struct ScopedModelsState {
    pub open: bool,
    pub selected: usize,
}

// In AppState
pub scoped_models: Vec<ScopedModel>,

pub struct ScopedModel {
    pub name: String,
    pub provider: String,
    pub enabled: bool,
}
```

### Events

```rust
Event::ToggleScopedModelsDialog,
Event::ScopedModelToggle { name: String },
Event::ScopedModelEnableAll,
Event::ScopedModelDisableAll,
Event::ScopedModelToggleProvider { provider: String },
```

### Config

```toml
[models]
scoped = ["gpt-4o", "claude-3-sonnet"]  # explicit list
# OR
scoped_all = true  # all models, managed via /scoped-models
```

## Acceptance Criteria

- [ ] `/scoped-models` opens model filter dialog
- [ ] Dialog lists all ~130 models with checkboxes
- [ ] Space toggles individual model enabled/disabled
- [ ] `a` enables all, `x` disables all
- [ ] `p` toggles all models for selected provider
- [ ] Only enabled models appear in Ctrl+P cycling
- [ ] Persisted in config.toml

## Tests

### Layer 1
- [ ] `toggle_model_excludes_from_cycle` — disabled model skipped
- [ ] `enable_all_includes_all` — all models in cycle list
- [ ] `disable_all_excludes_all` — no models in cycle list
- [ ] `provider_toggle_affects_all` — provider on/off

### Layer 2
- [ ] `slash_scoped_models_opens_dialog` — event pushed
