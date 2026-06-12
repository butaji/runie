# Unify Dialog / Command Palette under PanelStack DSL

**Status**: done

## Goal
Unify every dialog and command palette in the codebase under the existing `PanelStack` / `Panel` DSL. Eliminate bespoke per-dialog state (`filter`, `selected`, `category`) and bespoke handlers/renderers so that all dialogs are created, updated, and rendered through one shared abstraction.

## Motivation
The codebase had 129+ references to `DialogState::` variants with per-dialog fields and six bespoke dialog implementations (`CommandPalette`, `ModelSelector`, `Settings`, `ScopedModels`, `SessionTree`, plus the new `PanelStack`). The `dialog/builders.rs` DSL already provided the building blocks, but most code was not using it. Unifying reduces duplication and makes new dialogs trivial to add.

## Architecture
- `DialogState` enum variants now wrap a single `PanelStack`.
- `DialogState::panel_stack()` and `panel_stack_mut()` allow shared mutation regardless of variant.
- `update_dialog` extracts the `PanelStack`, routes events through `update_panel_stack`, and restores the original variant only when the handler did not activate an item.
- `update_panel_stack` handles navigation, filtering, activation, form handling, and emits `ItemAction`s for all dialog types.
- TUI rendering uses only `panel_dialog` in `crates/runie-tui/src/popups/panel.rs`; bespoke renderers in `popups.rs` are removed.
- Command palette, model selector, settings, scoped models, and session tree are opened through builder functions that produce a `PanelStack`.

## Acceptance Criteria
- [x] `DialogState` variants wrap `PanelStack`.
- [x] All dialog open paths use builder functions (`command_palette`, `model_selector`, `settings`, `scoped_models`, `session_tree`).
- [x] `update_dialog` and `update_panel_stack` are the single event-routing path for open dialogs.
- [x] Bespoke update handlers (`update_palette`, `update_model_selector`, `update_settings_dialog`, `update_scoped_models`, `update_session_tree`) are removed.
- [x] TUI renders all dialogs through `panel_dialog`.
- [x] Bespoke TUI renderers (`command_palette`, `model_selector_dialog`, `settings_dialog`, `scoped_models_dialog`, `session_tree_dialog`) are removed.
- [x] Commands with `CommandResult::OpenDialog(DialogType)` still open the correct dialog through the unified builders.
- [x] Form buttons (`Action` / `FormSubmit`) are activated by `Enter`/Submit and by accelerator keys.
- [x] `cargo build -p runie-core -p runie-tui -p runie-term` succeeds.

## Tests
### Layer 2 — Event Handling
- Update existing dialog event tests (`model_selector`, `scoped_models`, `settings_dialog`, `palette`, `session_tree`, `dialog_theme_switch`) to use the new `PanelStack`-backed `DialogState`.
- Add tests verifying `/settings`, `/model`, `/scoped-models`, `/tree` open the correct `DialogState` variant via the unified builders.
- Add tests verifying `Submit` on a palette item executes the command and replaces the dialog with the result.

### Layer 3 — Rendering
- Update TUI render tests to use `panel_dialog` instead of removed bespoke renderers.
- Verify popup background still hides underlying content for command palette, settings, model selector, and generic panel dialogs.

### Current Status
- `cargo build`, `cargo test`, and `cargo clippy --all-targets --all-features` all pass.
- All dialog open paths use the unified `PanelStack` DSL.
- TUI renders all dialogs through `panel_dialog`.
- E2E coverage in `crates/runie-term/tests/e2e.rs` exercises settings/model/theme/scoped-models dialogs with no panic.

## Next Steps
1. Re-enable cranelift backend in `.cargo/config.toml` once nightly toolchain is available.
