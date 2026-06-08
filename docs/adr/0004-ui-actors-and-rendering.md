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
