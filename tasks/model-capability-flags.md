# Model Capability Flags + Runtime Provider Switching

**Status**: todo
**Milestone**: R3
**Category**: Providers & Models
**Priority**: P1

**Depends on**: llm-event-normalization
**Blocks**: (enables `r2-model-cycling`, `r2-scoped-models`, `r2-thinking-levels`)

## Description

Runie currently switches models by string name and branches on provider names
in several places. Aider’s `ModelSettings` and Goose’s `SharedProvider` show
that provider/model behavior should be driven by capability flags, not string
matching. This task adds a capability model and makes providers swappable at
runtime without restarting the session.

## Acceptance Criteria

- [ ] `crates/runie-core/src/model_catalog.rs` `ModelInfo` extended with:
  ```rust
  pub struct ModelCapabilities {
      pub streaming: bool,
      pub supports_vision: bool,
      pub supports_tools: bool,
      pub supports_reasoning: bool,
      pub max_context_tokens: usize,
      pub max_output_tokens: usize,
      pub cache_control: bool,
  }
  ```
- [ ] Catalog entries populated for all ~130 known models.
- [ ] `runie-provider/src/lib.rs` exposes `SharedProvider`:
  ```rust
  pub type SharedProvider = Arc<Mutex<Option<Arc<dyn Provider>>>>;
  ```
  (or equivalent) so the `AgentActor` can swap providers/models mid-session.
- [ ] `AgentActor` reads capabilities before each turn and adapts:
  - Falls back to non-streaming if model lacks streaming.
  - Disables vision if model lacks it.
  - Selects thinking/reasoning parameters only if supported.
- [ ] `Event::SwitchModel { provider, model }` updates the active shared
  provider and emits a durable `ModelSwitched` event.
- [ ] `/model` slash command and model selector dialog use the capability flags
  to filter incompatible options.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `model_capabilities_detect_streaming` — flag is respected.
- [ ] `switch_model_updates_shared_provider` — after `SwitchModel`, the next
  turn uses the new provider.
- [ ] `scoped_models_filtered_by_capability` — vision-only models hidden when
  no image is attached.

### Layer 2 — Event Handling
- [ ] `switch_model_event_emits_model_switched` — durable event is appended.

### Layer 3 — Rendering
- [ ] `model_selector_shows_capability_badges` — each item shows streaming,
  tools, vision icons.

## Notes

**Files touched:**
- `crates/runie-core/src/model_catalog.rs`
- `crates/runie-provider/src/lib.rs`
- `crates/runie-agent/src/turn.rs`
- `crates/runie-core/src/commands/handlers/model.rs`

**Out of scope:**
- OAuth / managed provider login (already excluded by design).
- Per-provider pricing/cost tracking (covered by token tracking).
