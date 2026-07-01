# Round 4 — Module Boundaries & TUI / DSL

## Findings

Large / mixed-responsibility files exceed the advertised 500-line limit and mix concerns:

- `crates/runie-tui/src/ui_actor.rs` — 794 lines (event routing, input dispatch, effects, animation, submit logic).
- `crates/runie-core/src/proto/message/mod.rs` — 769 lines (`ChatMessage`, `ToolCall`, builder, validation).
- `crates/runie-core/src/session/sqlite_store.rs` — 674 lines (schema, queries, import, tests).
- `crates/runie-core/src/actors/fff_indexer/mod.rs` — 669 lines (index, search, frecency, git status).
- `crates/runie-core/src/provider/provider_trait.rs` — 650 lines (errors, metadata, retry config, trait, tests).
- `crates/runie-core/src/event/durable.rs` — 648 lines (dual enum + conversions + tests).
- `crates/runie-core/src/actors/permission/ractor_permission.rs` — 601 lines (handle, state, handlers, tests).
- `crates/runie-core/src/actors/io/ractor_io.rs` — 587 lines (handle, actor, blocking IO, tests).
- `crates/runie-core/src/update/dialog/panel_handler.rs` — 586 lines (navigation, activation, form, settings).
- `crates/runie-agent/src/actor.rs` — 578 lines (messages, actor, turn setup, factory, tests).
- `crates/runie-core/src/actors/turn/ractor_turn.rs` — 553 lines (handle, state, message handlers, actor impl, tests).
- `crates/runie-core/src/model/state/domain_ops.rs` — 410 lines (init, trust, config, view, model helpers).

The command/DSL boundary is also blurry: slash commands, palette commands, form submits, and declarative YAML commands overlap without a single canonical action enum.

## Recommended changes

1. Split `ui_actor.rs` into focused modules: `input.rs`, `effects.rs`, `submit.rs`, `animation.rs`.
2. Split `panel_handler.rs` into `navigation.rs`, `activation.rs`, `form.rs`.
3. Split `runie-agent/src/actor.rs` into `state.rs`, `turn.rs`, `factory.rs`.
4. Split `ractor_turn.rs` into `state.rs`, `handlers.rs`, `actor.rs`.
5. Split remaining large core files to stay under the 500-line guideline.
6. Collapse command types into a single `Command`/`Action` enum routed through the registry.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Split `ui_actor.rs` | `tasks/split-uiactor-into-focused-modules.md` | **new** |
| Split `panel_handler.rs` | `tasks/split-panel-handler-into-focused-modules.md` | **new** |
| Split `runie-agent/src/actor.rs` | `tasks/split-runie-agent-actor-into-focused-modules.md` | **new** |
| Split `ractor_turn.rs` | `tasks/split-ractor-turn-actor-into-focused-modules.md` | **new** |
| Split remaining large core files | `tasks/split-large-core-files-into-focused-modules.md` | **new** |
| Collapse command types | `tasks/collapse-command-types-to-single-command-action-enum.md` | existing `todo` |
| Unify form submit paths | `tasks/unify-form-submit-paths-through-command-registry.md` | existing `todo` |
