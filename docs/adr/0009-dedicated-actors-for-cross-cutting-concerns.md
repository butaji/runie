# Dedicated actors for cross-cutting concerns

Several cross-cutting concerns are isolated into dedicated actors:

- **TelemetryAgent**: Aggregates token usage, costs, timing from all relevant events.
- **SafetyAgent**: Validates dangerous operations (bash commands) before execution. Other actors consult it.
- **ClipboardAgent**: Handles clipboard interactions, including image paste.
- **FileLookupActor**: Resolves @-file references asynchronously.
- **CommandAgent**: Parses slash commands and key shortcuts, emits corresponding events.
- **ConfigAgent**: Loads and watches configuration, emits ConfigChanged events.

This keeps core logic clean and makes cross-cutting concerns testable in isolation.
