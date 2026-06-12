# Model Cycling

**Status**: done
**Milestone**: R2
**Category**: Input & Commands

## Description

Cycle through a scoped subset of models with `Ctrl+P` (next) and `Shift+Ctrl+P` (previous). Scoped list is configurable; defaults to first 10 models from catalog.

## Architecture

```rust
// In AppState
pub scoped_models: Vec<String>,
pub scoped_index: usize,
```

### Config

```toml
[models]
scoped = ["gpt-4o", "claude-3-5-sonnet", "gemini-1.5-pro", "o3-mini"]
```

### Command Factories

```rust
fn cmd_cycle_next(_args: &str) -> Option<Event> {
    Some(Event::CycleModelNext)
}

fn cmd_cycle_prev(_args: &str) -> Option<Event> {
    Some(Event::CycleModelPrev)
}
```

### Update Logic

```rust
fn update_cycle_model(state: &mut AppState, delta: isize) {
    if state.scoped_models.is_empty() { return; }
    let len = state.scoped_models.len() as isize;
    state.scoped_index = ((state.scoped_index as isize + delta).rem_euclid(len)) as usize;
    let name = state.scoped_models[state.scoped_index].clone();
    // Resolve name to provider+model, then emit SwitchModel
}
```

### Keybindings

```json
{
  "app.model.cycleForward": "ctrl+p",
  "app.model.cycleBackward": "shift+ctrl+p"
}
```

**Note**: Ctrl+P cycles models by default. The command palette opens with the mapped keybinding, but `app.model.cycleForward` takes precedence. To open the palette, users can type `/` in the input field.

## Acceptance Criteria

- [ ] `Ctrl+P` → next model in scoped list
- [ ] `Shift+Ctrl+P` → previous model
- [ ] Scoped list in `config.toml` `[models.scoped]`
- [ ] Default: first 10 models from catalog
- [ ] Wraps at boundaries
- [ ] Emits `SwitchModel` event
- [ ] System message: "Switched to provider/model"

## Files

| File | Description |
|------|-------------|
| `crates/runie-core/src/model.rs` | `scoped_models`, `scoped_index` |
| `crates/runie-core/src/event.rs` | `CycleModelNext`, `CycleModelPrev` |
| `crates/runie-core/src/update/mod.rs` | Handle cycle events |
| `crates/runie-core/src/config_reload.rs` | Parse `models.scoped` |
| `crates/runie-term/src/keymap.rs` | Map semantic names |

## Tests

### Layer 1 — State/Logic
- [ ] `cycle_next_increments` — index +1
- [ ] `cycle_prev_decrements` — index -1
- [ ] `cycle_wraps_forward` — last → first
- [ ] `cycle_wraps_backward` — first → last
- [ ] `cycle_empty_noop` — empty list is safe

### Layer 2 — Event Handling
- [ ] `cycle_emits_switch_model` — event updates provider/model
