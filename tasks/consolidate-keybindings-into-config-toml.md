# Consolidate Keybindings into `config.toml`

**Status**: done
**Milestone**: R3
**Category**: Configuration
**Priority**: P1

## Description

Keybindings currently live in a separate `~/.runie/keybindings.json` file loaded by `crates/runie-core/src/keybindings.rs`, while every other user setting lives in `~/.runie/config.toml` managed by `config_reload`. This split is inconsistent, duplicates path-handling logic, and forces users to edit two files for one application.

Additionally, `runie-term/src/keymap.rs` hardcodes many of the same defaults that already exist in `keybindings.rs`, creating two sources of truth for default bindings.

Move keybindings into a `[keybindings]` table inside `~/.runie/config.toml`, merge user overrides with code-defined defaults, and remove the hardcoded terminal-keymap duplicates.

## Acceptance Criteria

- [ ] `Config` in `crates/runie-core/src/config_reload/types.rs` gains a `[keybindings]` field (`HashMap<String, String>` or equivalent) parsed from TOML.
- [ ] Default keybindings remain defined in `crates/runie-core/src/keybindings.rs` as the single source of truth.
- [ ] On startup, user-defined `[keybindings]` entries override defaults; unspecified defaults remain available.
- [ ] `keybindings::load_keybindings` reads from the loaded `Config` instead of a separate JSON file path.
- [ ] The separate `~/.runie/keybindings.json` file is no longer read.
- [ ] Existing `~/.runie/keybindings.json` files are auto-migrated: contents are inserted into `~/.runie/config.toml` under `[keybindings]`, the JSON file is renamed to `keybindings.json.bak`, and a transient message informs the user.
- [ ] `runie-term/src/keymap.rs` no longer duplicates default semantic bindings; it only (a) normalizes crossterm events to combo strings, (b) looks them up in the merged map, and (c) handles terminal-specific translations that cannot be configured (e.g., Shift+Enter/F3/F13 quirks).
- [ ] The config watcher detects `[keybindings]` changes and emits a `KeybindingsReloaded` event (or equivalent) so the terminal keymap picks them up without restart.
- [ ] ADR 0013 is superseded/updated and `docs/CONTEXT.md` defines the canonical terms.

## Tests

### Layer 1 — State/Logic
- [ ] `config_keybindings_override_defaults` — parsing a config with `"ctrl+c" = "Abort"` under `[keybindings]` makes `ctrl+c` resolve to `Abort`, while other defaults remain unchanged.
- [ ] `config_keybindings_merge_with_defaults` — an empty `[keybindings]` table leaves all defaults intact.
- [ ] `keybindings_json_migration` — a mock `keybindings.json` is merged into a mock `config.toml` and the JSON file is renamed.
- [ ] `default_keybindings_resolve` (existing) still passes after removing hardcoded terminal duplicates.

### Layer 2 — Event Handling
- [ ] `config_watcher_emits_keybindings_reloaded` — writing a new `[keybindings]` entry to `config.toml` produces a reload event on the bus.
- [ ] `keymap_lookup_uses_config_bindings` — feeding a crossterm event whose combo is overridden in config returns the overridden `CoreEvent`.

### Layer 3 — Rendering
- [ ] No direct rendering changes. Help text and command palette already consume keybinding metadata; verify `cargo test -p runie-tui` still passes.

### Layer 4 — Smoke
- [ ] Start Runie, edit `~/.runie/config.toml` to remap `"ctrl+p"` to `"ToggleExpand"`, wait for the watcher interval, and confirm the new binding works in the TUI.
- [ ] Confirm that a pre-existing `keybindings.json` is migrated on first startup and the old file is renamed to `.bak`.

## Notes

**Why one file:**
The project already uses `~/.runie/config.toml` for provider, model, theme, UI flags, truncation, telemetry, and prompts. Keybindings are the only setting exiled to JSON. Consolidating reduces user friction and lets the existing config watcher handle hot reload.

**Why keep terminal-specific hardcodes:**
Some terminals send ambiguous escape sequences for modified Enter keys (Shift+Enter as F3/F13). These cannot be expressed as a user-configurable combo because the user sees "Shift+Enter" but the terminal sends something else. The terminal layer may still need a narrow translation pass for these cases.

**Migration safety:**
We must not silently drop user customizations. The auto-migrator reads the existing JSON, inserts entries into `config.toml`, rewrites the TOML, and renames the JSON file. If either read or write fails, we fall back to defaults and log a warning rather than crash.

## Out of scope

- Changing the default bindings themselves (this is a consolidation, not a UX redesign).
- Adding a UI keybinding editor.
- Refactoring the broader config-reload watcher from polling to notify (covered by `consolidate-config-reload-types`).

## Related

- `tasks/consolidate-config-reload-types.md` — broader config-reload cleanup; keep interfaces compatible.
- `docs/adr/0013-configurable-keybindings.md` — old JSON-based decision to supersede.

## Verification

```bash
# Layer 1 & 2
cargo test -p runie-core --lib keybindings
cargo test -p runie-term --lib keymap

# Layer 3
cargo test -p runie-tui

# Layer 4
./scripts/smoke-tmux.sh
```
