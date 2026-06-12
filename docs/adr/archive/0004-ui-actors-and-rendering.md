# UI actors with hierarchical routing

The UI is structured as a hierarchy of actors with a root router:

```
UIRoot
  ├─ InputAgent ──→ input, cursor, history
  ├─ ScrollAgent ──→ scroll, viewport
  ├─ ChatAgent ────→ elements, streaming
  └─ PopupAgent ───→ hints, @-suggestions
```

UIRoot holds typed channels to each child and routes incoming events accordingly. Each child actor accumulates its own state from events it receives.

Rendering is a pure function from UIAgent state to Frame. `runie-tui` owns the UI actors and the ratatui rendering logic.

**Status:** This hierarchy is a future design target, not an R1 commitment.
The current codebase uses a single render loop with state updates (simpler,
477 tests passing). R1 focuses on modularizing the existing code (splitting
`update.rs` and `AppState`) rather than introducing actor boundaries in the UI.
UI actor extraction will be revisited when a concrete need arises (e.g.
background input processing or off-main-thread rendering).
