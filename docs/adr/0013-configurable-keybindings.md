# Configurable keybindings via ConfigAgent

Keybinding mappings are loaded from `keybindings.json` at startup by ConfigAgent. ConfigAgent emits `KeybindingsLoaded { map }` and watches for file changes to support hot reload.

Actors subscribe to config updates and use the current keybinding map.

Default keybindings:

| Key | Action | Actor |
|-----|--------|-------|
| Backspace | Delete char | InputAgent |
| Ctrl+W | Delete word | InputAgent |
| Ctrl+U | Delete to start | InputAgent |
| Ctrl+K | Delete to end | InputAgent |
| Ctrl+D | Delete char at cursor | InputAgent |
| Left/Right | Cursor move | InputAgent |
| Alt+Left/Right | Word jump | InputAgent |
| Home/End | Line start/end | InputAgent |
| Ctrl+Z | Undo | InputAgent |
| Ctrl+Shift+Z | Redo | InputAgent |
| Up/Down | History | InputAgent |
| Ctrl+J | Newline | InputAgent |
| Shift+Enter | Newline | InputAgent |
| Tab | Path completion | InputAgent |
| Esc | Clear/cancel | InputAgent |
| !command | Run bash | InputAgent |
| Up/Down | Scroll | ScrollAgent |
| PageUp/PageDown | Page scroll | ScrollAgent |
| Home/End | Scroll top/bottom | ScrollAgent |
| Mouse wheel | Scroll | ScrollAgent |
| Ctrl+Shift+E | Collapse/expand | ChatAgent |
| Ctrl+P | Commands panel | CommandAgent |
| Ctrl+V | Image paste | ClipboardAgent |
| Alt+Up | Dequeue | Orchestrator |
| Ctrl+G | External editor | Orchestrator |
