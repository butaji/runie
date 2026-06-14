# Keybindings live in `~/.runie/config.toml`

## Context

ADR 0013 decided that keybindings would be loaded from a separate `~/.runie/keybindings.json` file by ConfigAgent. At that time the rest of Runie's user settings were not yet consolidated, so a dedicated file seemed reasonable.

Since then, `~/.runie/config.toml` has become the single place for provider, model, theme, UI flags, truncation, prompts, telemetry, and provider credentials. Keeping keybindings in a separate JSON file is now an outlier: users must edit two files, the code loads paths in two places, and the existing `config.toml` watcher cannot reload keybinding changes.

## Decision

1. **Keybindings move into `~/.runie/config.toml` under a `[keybindings]` table.**
   ```toml
   [keybindings]
   "ctrl+o" = "ToggleExpand"
   "alt+enter" = "FollowUp"
   ```
2. **Defaults remain code-defined in `runie-core/src/keybindings.rs`.** User entries override defaults; missing defaults remain available.
3. **`runie-term/src/keymap.rs` becomes a thin translation layer.** It normalizes crossterm events to combo strings, looks them up in the merged map, and only handles terminal-specific translations that cannot be expressed as configurable combos (e.g., Shift+Enter arriving as F3/F13).
4. **Existing `~/.runie/keybindings.json` files are auto-migrated.** On startup, if the JSON file exists, its contents are inserted into `~/.runie/config.toml` under `[keybindings]`, the JSON file is renamed to `keybindings.json.bak`, and the user is notified.
5. **ADR 0013 is superseded.** The JSON-file approach is deprecated and will be removed once migration has been in the wild for a reasonable time.

## Consequences

- **Positive:** One config file for all user settings. Hot reload for keybindings comes for free through the existing `config.toml` watcher.
- **Positive:** Single source of truth for default semantics; terminal-specific code cannot drift out of sync with declared defaults.
- **Trade-off:** Users with existing JSON files pay a one-time migration cost, but data is preserved.
- **Trade-off:** TOML table keys must be quoted when they contain `+` or other non-identifier characters, making the file slightly noisier than JSON for this particular section.
