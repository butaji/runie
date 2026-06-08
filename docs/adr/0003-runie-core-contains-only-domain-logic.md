# runie-core contains only domain logic

`runie-core` owns:
- `Event` type (unified)
- Domain state (`AppState`, messages, queue)
- Session persistence (event log + snapshots)
- Provider trait

`runie-core` does NOT own:
- `Element` enum or UI transforms → moved to `runie-tui`
- Tool execution → owned by `runie-agent`
- Rendering → owned by `runie-tui`

This ensures `runie-core` is TUI-agnostic and could be used by non-interactive binaries.
